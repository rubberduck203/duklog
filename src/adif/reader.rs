//! Async ADIF log reader.
//!
//! Parses internal `.adif` files written by the storage layer and reconstructs
//! [`Log`](crate::model::Log) values. The `APP_DUKLOG_*` header fields encode
//! the log type and all type-specific metadata.

use std::path::Path;

use chrono::{DateTime, Utc};
use difa::{Datum, Record, RecordStream};
use futures::StreamExt;
use tokio::io::BufReader;

use super::error::AdifError;
use crate::model::{
    Band, FdPowerCategory, FieldDayLog, GeneralLog, Log, LogHeader, Mode, PotaLog, Qso, WfdLog,
    parse_fd_class, parse_wfd_class, validate_tx_count,
};

/// Reads an ADIF file and reconstructs the [`Log`] it encodes.
///
/// The file must have been written by [`format_adif`](super::writer::format_adif).
/// The `APP_DUKLOG_LOG_TYPE` header field determines the log variant, and
/// `APP_DUKLOG_LOG_ID` provides the log identifier.
pub async fn read_log(path: &Path) -> Result<Log, AdifError> {
    let file = tokio::fs::File::open(path).await.map_err(difa::Error::Io)?;
    let reader = BufReader::new(file);
    let mut stream = RecordStream::new(reader, true);

    let header_record = match stream.next().await {
        Some(Ok(rec)) if rec.is_header() => rec,
        Some(Ok(_)) => {
            return Err(AdifError::InvalidLog("first record is not a header".into()));
        }
        Some(Err(e)) => return Err(AdifError::Encode(e)),
        None => {
            return Err(AdifError::InvalidLog("file contains no records".into()));
        }
    };

    let station_callsign = get_str(&header_record, "station_callsign")?;
    let operator = header_record
        .get("operator")
        .map(|d| d.as_str().into_owned());
    let grid_square = header_record
        .get("my_gridsquare")
        .map(|d| d.as_str().into_owned())
        .unwrap_or_default();
    let created_at = parse_created_timestamp(&header_record)?;
    let log_id = get_str(&header_record, "app_duklog_log_id")?;
    let log_type = get_str(&header_record, "app_duklog_log_type")?;

    // Extract type-specific fields from the header record before consuming the stream.
    let park_ref = header_record
        .get("app_duklog_park_ref")
        .map(|d| d.as_str().into_owned());
    let tx_count = parse_opt_tx_count(&header_record)?;
    let fd_class = parse_opt_fd_class(&header_record)?;
    let wfd_class = parse_opt_wfd_class(&header_record)?;
    let section = header_record
        .get("app_duklog_section")
        .map(|d| d.as_str().into_owned());
    let power = parse_opt_power(&header_record)?;

    let mut qsos = Vec::new();
    while let Some(result) = stream.next().await {
        let record = result?;
        qsos.push(parse_qso(&record)?);
    }

    let header = LogHeader {
        station_callsign,
        operator,
        grid_square,
        qsos,
        created_at,
        log_id,
    };

    match log_type.as_str() {
        "general" => Ok(Log::General(GeneralLog { header })),
        "pota" => {
            let park_ref = park_ref.ok_or_else(|| {
                AdifError::InvalidLog("POTA log missing APP_DUKLOG_PARK_REF".into())
            })?;
            Ok(Log::Pota(PotaLog { header, park_ref }))
        }
        "field_day" => {
            let tx_count = tx_count.ok_or_else(|| {
                AdifError::InvalidLog("FieldDay log missing APP_DUKLOG_TX_COUNT".into())
            })?;
            validate_tx_count(tx_count).map_err(|e| AdifError::InvalidLog(e.to_string()))?;
            let class = fd_class.ok_or_else(|| {
                AdifError::InvalidLog("FieldDay log missing APP_DUKLOG_FD_CLASS".into())
            })?;
            let section = section.filter(|s| !s.is_empty()).ok_or_else(|| {
                AdifError::InvalidLog("FieldDay log missing APP_DUKLOG_SECTION".into())
            })?;
            let power = power.ok_or_else(|| {
                AdifError::InvalidLog("FieldDay log missing APP_DUKLOG_POWER".into())
            })?;
            Ok(Log::FieldDay(FieldDayLog {
                header,
                tx_count,
                class,
                section,
                power,
            }))
        }
        "wfd" => {
            let tx_count = tx_count.ok_or_else(|| {
                AdifError::InvalidLog("WFD log missing APP_DUKLOG_TX_COUNT".into())
            })?;
            validate_tx_count(tx_count).map_err(|e| AdifError::InvalidLog(e.to_string()))?;
            let class = wfd_class.ok_or_else(|| {
                AdifError::InvalidLog("WFD log missing APP_DUKLOG_WFD_CLASS".into())
            })?;
            let section = section.filter(|s| !s.is_empty()).ok_or_else(|| {
                AdifError::InvalidLog("WFD log missing APP_DUKLOG_SECTION".into())
            })?;
            Ok(Log::WinterFieldDay(WfdLog {
                header,
                tx_count,
                class,
                section,
            }))
        }
        other => Err(AdifError::InvalidLog(format!("unknown log type: {other}"))),
    }
}

fn get_str(record: &Record, field: &str) -> Result<String, AdifError> {
    record
        .get(field)
        .map(|d| d.as_str().into_owned())
        .ok_or_else(|| AdifError::InvalidLog(format!("missing required field: {field}")))
}

fn parse_created_timestamp(record: &Record) -> Result<DateTime<Utc>, AdifError> {
    let s = get_str(record, "created_timestamp")?;
    chrono::NaiveDateTime::parse_from_str(&s, "%Y%m%d %H%M%S")
        .map(|dt| dt.and_utc())
        .map_err(|_| AdifError::InvalidLog(format!("invalid CREATED_TIMESTAMP: {s}")))
}

fn parse_opt_tx_count(record: &Record) -> Result<Option<u8>, AdifError> {
    record
        .get("app_duklog_tx_count")
        .map(|d| {
            let s = d.as_str();
            s.trim()
                .parse::<u8>()
                .map_err(|_| AdifError::InvalidLog(format!("invalid APP_DUKLOG_TX_COUNT: {s}")))
        })
        .transpose()
}

fn parse_opt_fd_class(record: &Record) -> Result<Option<crate::model::FdClass>, AdifError> {
    record
        .get("app_duklog_fd_class")
        .map(|d| {
            let s = d.as_str();
            parse_fd_class(&s)
                .map_err(|_| AdifError::InvalidLog(format!("invalid APP_DUKLOG_FD_CLASS: {s}")))
        })
        .transpose()
}

fn parse_opt_wfd_class(record: &Record) -> Result<Option<crate::model::WfdClass>, AdifError> {
    record
        .get("app_duklog_wfd_class")
        .map(|d| {
            let s = d.as_str();
            parse_wfd_class(&s)
                .map_err(|_| AdifError::InvalidLog(format!("invalid APP_DUKLOG_WFD_CLASS: {s}")))
        })
        .transpose()
}

fn parse_opt_power(record: &Record) -> Result<Option<FdPowerCategory>, AdifError> {
    record
        .get("app_duklog_power")
        .map(|d| {
            let s = d.as_str();
            FdPowerCategory::from_adif_str(&s)
                .ok_or_else(|| AdifError::InvalidLog(format!("invalid APP_DUKLOG_POWER: {s}")))
        })
        .transpose()
}

fn parse_qso(record: &Record) -> Result<Qso, AdifError> {
    let their_call = get_str(record, "call")?;

    let date = record
        .get("qso_date")
        .and_then(Datum::as_date)
        .ok_or_else(|| AdifError::InvalidLog("QSO missing QSO_DATE".into()))?;
    let time = record
        .get("time_on")
        .and_then(Datum::as_time)
        .ok_or_else(|| AdifError::InvalidLog("QSO missing TIME_ON".into()))?;
    let timestamp = date.and_time(time).and_utc();

    let band_str = get_str(record, "band")?;
    let band = Band::from_adif_str(&band_str)
        .ok_or_else(|| AdifError::InvalidLog(format!("unknown BAND: {band_str}")))?;

    let mode_str = get_str(record, "mode")?;
    let mode = Mode::from_adif_str(&mode_str)
        .ok_or_else(|| AdifError::InvalidLog(format!("unknown MODE: {mode_str}")))?;

    let rst_sent = get_str(record, "rst_sent")?;
    let rst_rcvd = get_str(record, "rst_rcvd")?;

    let comments = record
        .get("comment")
        .map(|d| d.as_str().into_owned())
        .unwrap_or_default();

    let their_park = record.get("sig_info").map(|d| d.as_str().into_owned());
    let exchange_rcvd = record.get("srx_string").map(|d| d.as_str().into_owned());
    let frequency = record
        .get("freq")
        .and_then(|d| d.as_str().parse::<f64>().ok())
        .map(|mhz| (mhz * 1000.0).round() as u32);

    Qso::new(
        their_call,
        rst_sent,
        rst_rcvd,
        band,
        mode,
        timestamp,
        comments,
        their_park,
        exchange_rcvd,
        frequency,
    )
    .map_err(|e| AdifError::InvalidLog(e.to_string()))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    use super::*;
    use crate::adif::format_adif;
    use crate::model::{
        Band, FdClass, FdPowerCategory, FieldDayLog, GeneralLog, Mode, PotaLog, Qso, WfdClass,
        WfdLog,
    };

    fn make_pota_log() -> Log {
        let mut log = PotaLog::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            "K-0001".to_string(),
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log.header.log_id = "K-0001-20260216-120000".to_string();
        Log::Pota(log)
    }

    fn make_general_log() -> Log {
        let mut log = GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log.header.log_id = "W1AW-20260216-120000".to_string();
        Log::General(log)
    }

    fn make_fd_log() -> Log {
        let mut log = FieldDayLog::new(
            "W1AW".to_string(),
            None,
            2,
            FdClass::B,
            "EPA".to_string(),
            FdPowerCategory::Low,
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log.header.log_id = "FD-W1AW-20260216-120000".to_string();
        Log::FieldDay(log)
    }

    fn make_wfd_log() -> Log {
        let mut log = WfdLog::new(
            "W1AW".to_string(),
            None,
            1,
            WfdClass::H,
            "EPA".to_string(),
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log.header.log_id = "WFD-W1AW-20260216-120000".to_string();
        Log::WinterFieldDay(log)
    }

    fn make_qso() -> Qso {
        Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
            String::new(),
            None,
            None,
            None,
        )
        .unwrap()
    }

    fn make_p2p_qso() -> Qso {
        Qso::new(
            "N0CALL".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M40,
            Mode::Cw,
            Utc.with_ymd_and_hms(2026, 2, 16, 15, 0, 0).unwrap(),
            "P2P".to_string(),
            Some("K-1234".to_string()),
            None,
            None,
        )
        .unwrap()
    }

    async fn round_trip(log: &Log) -> Log {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.adif");
        let content = format_adif(log).unwrap();
        tokio::fs::write(&path, content).await.unwrap();
        read_log(&path).await.unwrap()
    }

    #[tokio::test]
    async fn pota_log_round_trips() {
        let log = make_pota_log();
        let loaded = round_trip(&log).await;
        assert_eq!(log, loaded);
    }

    #[tokio::test]
    async fn pota_log_with_qsos_round_trips() {
        let mut log = make_pota_log();
        log.add_qso(make_qso());
        log.add_qso(make_p2p_qso());
        let loaded = round_trip(&log).await;
        assert_eq!(log, loaded);
    }

    #[tokio::test]
    async fn general_log_round_trips() {
        let log = make_general_log();
        let loaded = round_trip(&log).await;
        assert_eq!(log, loaded);
    }

    #[tokio::test]
    async fn fd_log_round_trips() {
        let log = make_fd_log();
        let loaded = round_trip(&log).await;
        assert_eq!(log, loaded);
    }

    #[tokio::test]
    async fn wfd_log_round_trips() {
        let log = make_wfd_log();
        let loaded = round_trip(&log).await;
        assert_eq!(log, loaded);
    }

    #[tokio::test]
    async fn pota_log_with_freq_qso_round_trips() {
        let mut log = make_pota_log();
        let qso = Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
            String::new(),
            None,
            None,
            Some(14_225),
        )
        .unwrap();
        log.add_qso(qso);
        let loaded = round_trip(&log).await;
        assert_eq!(loaded.header().qsos[0].frequency, Some(14_225));
    }

    #[tokio::test]
    async fn qso_only_file_without_header_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("qso-only.adif");
        // A valid QSO record but no <eoh> header section
        let content =
            "<CALL:6>KD9XYZ<QSO_DATE:8>20260216<TIME_ON:6>143000<BAND:3>20M<MODE:3>SSB<eor>\n";
        tokio::fs::write(&path, content).await.unwrap();
        let result = read_log(&path).await;
        let Err(AdifError::InvalidLog(msg)) = result else {
            panic!("expected InvalidLog error, got {result:?}");
        };
        assert!(
            msg.contains("header"),
            "error should mention 'header', got: {msg}"
        );
    }

    #[tokio::test]
    async fn empty_adif_file_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.adif");
        tokio::fs::write(&path, b"").await.unwrap();
        let result = read_log(&path).await;
        assert!(matches!(result, Err(AdifError::InvalidLog(_))));
    }

    #[tokio::test]
    async fn missing_log_type_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.adif");
        // Valid ADIF header but missing APP_DUKLOG_LOG_TYPE
        let content = "<STATION_CALLSIGN:4>W1AW\n<APP_DUKLOG_LOG_ID:4>test\n<CREATED_TIMESTAMP:15>20260216 120000\n<eoh>\n\n";
        tokio::fs::write(&path, content).await.unwrap();
        let result = read_log(&path).await;
        assert!(matches!(result, Err(AdifError::InvalidLog(_))));
    }

    #[tokio::test]
    async fn fd_log_missing_section_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fd-bad.adif");
        let content = "<STATION_CALLSIGN:4>W1AW\n<APP_DUKLOG_LOG_ID:8>fd-nosec\n<CREATED_TIMESTAMP:15>20260216 120000\n<APP_DUKLOG_LOG_TYPE:9>field_day\n<APP_DUKLOG_TX_COUNT:1>1\n<APP_DUKLOG_FD_CLASS:1>B\n<APP_DUKLOG_POWER:3>low\n<eoh>\n\n";
        tokio::fs::write(&path, content).await.unwrap();
        let result = read_log(&path).await;
        assert!(
            matches!(result, Err(AdifError::InvalidLog(_))),
            "expected InvalidLog, got {result:?}"
        );
    }
}
