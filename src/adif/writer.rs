use bytes::BytesMut;
use difa::write::TagEncoder;
use difa::{Datum, Field, Tag};
use tokio_util::codec::Encoder;

use super::error::AdifError;
use crate::model::{Log, Qso};

// Encodes a tag into the buffer.
fn encode(encoder: &mut TagEncoder, buf: &mut BytesMut, tag: Tag) -> Result<(), AdifError> {
    encoder.encode(tag, buf)?;
    Ok(())
}

// Creates a field tag from a name and value.
fn field_tag(name: &str, value: impl Into<Datum>) -> Tag {
    Tag::Field(Field::new(name, value))
}

// Converts a BytesMut buffer to a String.
fn buf_to_string(buf: BytesMut) -> Result<String, AdifError> {
    Ok(String::from_utf8(buf.into())?)
}

// The SIG/MY_SIG value for POTA contacts.
const POTA_SIG: &str = "POTA";
// CONTEST_ID values for contest logs.
const FIELD_DAY_CONTEST_ID: &str = "ARRL-FIELD-DAY";
const WFD_CONTEST_ID: &str = "WFD";

/// Encodes log-type-specific ADIF fields.
///
/// - POTA: emits `MY_SIG`/`MY_SIG_INFO` (when log has a park ref) and
///   `SIG`/`SIG_INFO` (when QSO has their park set).
/// - Field Day: emits `CONTEST_ID`, `STX_STRING`, and `SRX_STRING` (when present).
/// - Winter Field Day: emits `CONTEST_ID`, `STX_STRING`, and `SRX_STRING` (when present).
fn encode_type_specific_fields(
    encoder: &mut TagEncoder,
    buf: &mut BytesMut,
    log: &Log,
    qso: &Qso,
) -> Result<(), AdifError> {
    match log {
        Log::General(_) => {}
        Log::Pota(pota) => {
            encode(encoder, buf, field_tag("MY_SIG", POTA_SIG))?;
            encode(
                encoder,
                buf,
                field_tag("MY_SIG_INFO", pota.park_ref.as_str()),
            )?;
            if let Some(ref their_park) = qso.their_park {
                encode(encoder, buf, field_tag("SIG", POTA_SIG))?;
                encode(encoder, buf, field_tag("SIG_INFO", their_park.as_str()))?;
            }
        }
        Log::FieldDay(fd) => {
            encode(encoder, buf, field_tag("CONTEST_ID", FIELD_DAY_CONTEST_ID))?;
            encode(
                encoder,
                buf,
                field_tag("STX_STRING", fd.sent_exchange().as_str()),
            )?;
            if let Some(ref exch) = qso.exchange_rcvd {
                encode(encoder, buf, field_tag("SRX_STRING", exch.as_str()))?;
            }
        }
        Log::WinterFieldDay(wfd) => {
            encode(encoder, buf, field_tag("CONTEST_ID", WFD_CONTEST_ID))?;
            encode(
                encoder,
                buf,
                field_tag("STX_STRING", wfd.sent_exchange().as_str()),
            )?;
            if let Some(ref exch) = qso.exchange_rcvd {
                encode(encoder, buf, field_tag("SRX_STRING", exch.as_str()))?;
            }
        }
    }
    Ok(())
}

/// Formats the ADIF file header for a log.
///
/// Includes `ADIF_VER`, `PROGRAMID`, `PROGRAMVERSION`, and `CREATED_TIMESTAMP`,
/// terminated by `<eoh>`.
pub fn format_header(log: &Log) -> Result<String, AdifError> {
    let mut encoder = TagEncoder::new();
    let mut buf = BytesMut::new();

    encode(&mut encoder, &mut buf, field_tag("ADIF_VER", "3.1.6"))?;
    buf.extend_from_slice(b"\n");
    encode(&mut encoder, &mut buf, field_tag("PROGRAMID", "duklog"))?;
    buf.extend_from_slice(b"\n");
    encode(
        &mut encoder,
        &mut buf,
        field_tag("PROGRAMVERSION", env!("CARGO_PKG_VERSION")),
    )?;
    buf.extend_from_slice(b"\n");

    let timestamp = log.header().created_at.format("%Y%m%d %H%M%S").to_string();
    encode(
        &mut encoder,
        &mut buf,
        field_tag("CREATED_TIMESTAMP", timestamp.as_str()),
    )?;
    buf.extend_from_slice(b"\n");

    encode(&mut encoder, &mut buf, Tag::Eoh)?;
    buf.extend_from_slice(b"\n");

    buf_to_string(buf)
}

/// Formats a single QSO record ending with `<eor>`.
///
/// Includes per-log fields (station callsign, park ref) alongside per-QSO
/// fields. OPERATOR is emitted only when set and different from the station
/// callsign. POTA fields are only emitted when the relevant park references
/// are present. FREQ is emitted for any log type when `qso.frequency` is set.
pub fn format_qso(log: &Log, qso: &Qso) -> Result<String, AdifError> {
    let mut encoder = TagEncoder::new();
    let mut buf = BytesMut::new();

    encode(
        &mut encoder,
        &mut buf,
        field_tag("STATION_CALLSIGN", log.header().station_callsign.as_str()),
    )?;
    if let Some(ref op) = log.header().operator
        && op != &log.header().station_callsign
    {
        encode(&mut encoder, &mut buf, field_tag("OPERATOR", op.as_str()))?;
    }
    encode(
        &mut encoder,
        &mut buf,
        field_tag("CALL", qso.their_call.as_str()),
    )?;
    encode(
        &mut encoder,
        &mut buf,
        field_tag("QSO_DATE", Datum::Date(qso.timestamp.date_naive())),
    )?;
    encode(
        &mut encoder,
        &mut buf,
        field_tag("TIME_ON", Datum::Time(qso.timestamp.time())),
    )?;
    encode(
        &mut encoder,
        &mut buf,
        field_tag("BAND", qso.band.adif_str()),
    )?;
    encode(
        &mut encoder,
        &mut buf,
        field_tag("MODE", qso.mode.adif_str()),
    )?;
    encode(
        &mut encoder,
        &mut buf,
        field_tag("RST_SENT", qso.rst_sent.as_str()),
    )?;
    encode(
        &mut encoder,
        &mut buf,
        field_tag("RST_RCVD", qso.rst_rcvd.as_str()),
    )?;
    if !log.header().grid_square.is_empty() {
        encode(
            &mut encoder,
            &mut buf,
            field_tag("MY_GRIDSQUARE", log.header().grid_square.as_str()),
        )?;
    }

    encode_type_specific_fields(&mut encoder, &mut buf, log, qso)?;

    if let Some(freq) = qso.frequency {
        let mhz = format!("{:.3}", f64::from(freq) / 1000.0);
        encode(&mut encoder, &mut buf, field_tag("FREQ", mhz.as_str()))?;
    }

    if !qso.comments.is_empty() {
        encode(
            &mut encoder,
            &mut buf,
            field_tag("COMMENT", qso.comments.as_str()),
        )?;
    }

    encode(&mut encoder, &mut buf, Tag::Eor)?;

    buf_to_string(buf)
}

/// Formats a complete ADIF file (header + all QSO records).
pub fn format_adif(log: &Log) -> Result<String, AdifError> {
    log.header()
        .qsos
        .iter()
        .try_fold(format_header(log)?, |mut output, qso| {
            output.push_str(&format_qso(log, qso)?);
            Ok(output)
        })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use quickcheck_macros::quickcheck;

    use super::*;
    use crate::model::{Band, GeneralLog, Mode, PotaLog};

    fn make_log() -> Log {
        let mut log = PotaLog::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            "K-0001".to_string(),
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::Pota(log)
    }

    fn make_log_distinct_operator() -> Log {
        let mut log = PotaLog::new(
            "W1AW".to_string(),
            Some("N0CALL".to_string()),
            "K-0001".to_string(),
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::Pota(log)
    }

    fn make_log_none_operator() -> Log {
        let mut log = PotaLog::new(
            "W1AW".to_string(),
            None,
            "K-0001".to_string(),
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::Pota(log)
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
            Mode::Ssb,
            Utc.with_ymd_and_hms(2026, 2, 16, 15, 0, 0).unwrap(),
            String::new(),
            Some("K-1234".to_string()),
            None,
            None,
        )
        .unwrap()
    }

    fn make_qso_with_comment() -> Qso {
        Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
            "Good signal".to_string(),
            None,
            None,
            None,
        )
        .unwrap()
    }

    // --- format_header tests ---

    #[test]
    fn header_contains_adif_ver() {
        let header = format_header(&make_log()).unwrap();
        assert!(header.contains("<ADIF_VER:5>3.1.6"));
    }

    #[test]
    fn header_contains_programid() {
        let header = format_header(&make_log()).unwrap();
        assert!(header.contains("<PROGRAMID:6>duklog"));
    }

    #[test]
    fn header_contains_programversion() {
        let header = format_header(&make_log()).unwrap();
        let version = env!("CARGO_PKG_VERSION");
        let expected = format!("<PROGRAMVERSION:{}>{}", version.len(), version);
        assert!(header.contains(&expected));
    }

    #[test]
    fn header_contains_created_timestamp() {
        let header = format_header(&make_log()).unwrap();
        assert!(header.contains("<CREATED_TIMESTAMP:15>20260216 120000"));
    }

    #[test]
    fn header_ends_with_eoh() {
        let header = format_header(&make_log()).unwrap();
        assert!(header.contains("<eoh>"));
        assert!(header.ends_with("<eoh>\n\n"));
    }

    // --- format_qso tests ---

    #[test]
    fn qso_contains_required_fields() {
        let record = format_qso(&make_log(), &make_qso()).unwrap();

        assert!(record.contains("<STATION_CALLSIGN:4>W1AW"));
        assert!(record.contains("<CALL:6>KD9XYZ"));
        assert!(record.contains("<QSO_DATE:8>20260216"));
        assert!(record.contains("<TIME_ON:6>143000"));
        assert!(record.contains("<BAND:3>20M"));
        assert!(record.contains("<MODE:3>SSB"));
        assert!(record.contains("<RST_SENT:2>59"));
        assert!(record.contains("<RST_RCVD:2>59"));
        assert!(record.contains("<MY_GRIDSQUARE:4>FN31"));
    }

    #[test]
    fn qso_same_operator_excludes_operator_field() {
        let record = format_qso(&make_log(), &make_qso()).unwrap();
        assert!(!record.contains("OPERATOR"));
    }

    #[test]
    fn qso_none_operator_excludes_operator_field() {
        let record = format_qso(&make_log_none_operator(), &make_qso()).unwrap();
        assert!(!record.contains("OPERATOR"));
    }

    #[test]
    fn qso_distinct_operator() {
        let record = format_qso(&make_log_distinct_operator(), &make_qso()).unwrap();

        assert!(record.contains("<STATION_CALLSIGN:4>W1AW"));
        assert!(record.contains("<OPERATOR:6>N0CALL"));
    }

    #[test]
    fn qso_with_park_includes_my_sig() {
        let record = format_qso(&make_log(), &make_qso()).unwrap();

        assert!(record.contains("<MY_SIG:4>POTA"));
        assert!(record.contains("<MY_SIG_INFO:6>K-0001"));
    }

    #[test]
    fn general_log_excludes_pota_sig_fields() {
        let log =
            Log::General(GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap());
        let record = format_qso(&log, &make_qso()).unwrap();

        assert!(
            !record.contains("MY_SIG"),
            "general log must not emit MY_SIG"
        );
        assert!(
            !record.contains("SIG_INFO"),
            "general log must not emit SIG_INFO"
        );
    }

    #[test]
    fn qso_p2p_includes_sig() {
        let record = format_qso(&make_log(), &make_p2p_qso()).unwrap();

        assert!(record.contains("<SIG:4>POTA"));
        assert!(record.contains("<SIG_INFO:6>K-1234"));
        // P2P: both activator and hunter park fields present
        assert!(record.contains("<MY_SIG_INFO:6>K-0001"));
    }

    #[test]
    fn qso_without_their_park_excludes_sig() {
        let record = format_qso(&make_log(), &make_qso()).unwrap();

        assert!(!record.contains("<SIG:"));
        assert!(!record.contains("<SIG_INFO:"));
    }

    #[test]
    fn qso_with_comment() {
        let record = format_qso(&make_log(), &make_qso_with_comment()).unwrap();

        assert!(record.contains("<COMMENT:11>Good signal"));
    }

    #[test]
    fn qso_without_comment_excludes_comment_field() {
        let record = format_qso(&make_log(), &make_qso()).unwrap();

        assert!(!record.contains("COMMENT"));
    }

    #[test]
    fn qso_ends_with_eor() {
        let record = format_qso(&make_log(), &make_qso()).unwrap();

        assert!(record.ends_with("<eor>\n"));
    }

    // --- format_adif tests ---

    #[test]
    fn adif_empty_log_header_only() {
        let output = format_adif(&make_log()).unwrap();

        assert!(output.contains("<eoh>"));
        assert!(!output.contains("<eor>"));
    }

    #[test]
    fn adif_with_two_qsos() {
        let mut log = make_log();
        log.add_qso(make_qso());
        log.add_qso(make_p2p_qso());

        let output = format_adif(&log).unwrap();
        let eor_count = output.matches("<eor>").count();
        assert_eq!(eor_count, 2);
        assert!(output.contains("<eoh>"));
    }

    #[quickcheck]
    fn adif_eor_count_matches_qso_count(n: u8) -> bool {
        let n = n.min(20) as usize;
        let mut log = make_log();
        for _ in 0..n {
            log.add_qso(make_qso());
        }
        let output = format_adif(&log).unwrap();
        output.matches("<eor>").count() == n
    }

    #[quickcheck]
    fn qso_call_field_has_correct_byte_length(call: String) -> bool {
        // Skip strings that fail callsign validation or contain ADIF delimiters
        if call.is_empty() || call.len() > 20 || call.contains('<') || call.contains('>') {
            return true;
        }
        let qso = match Qso::new(
            call.clone(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
            String::new(),
            None,
            None,
            None,
        ) {
            Ok(q) => q,
            Err(_) => return true,
        };
        let record = format_qso(&make_log(), &qso).unwrap();
        let expected = format!("<CALL:{}>{}", call.len(), call);
        record.contains(&expected)
    }

    fn make_fd_log() -> Log {
        let mut log = crate::model::FieldDayLog::new(
            "W1AW".to_string(),
            None,
            1,
            crate::model::FdClass::B,
            "EPA".to_string(),
            crate::model::FdPowerCategory::Low,
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::FieldDay(log)
    }

    fn make_wfd_log() -> Log {
        let mut log = crate::model::WfdLog::new(
            "W1AW".to_string(),
            None,
            1,
            crate::model::WfdClass::H,
            "EPA".to_string(),
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::WinterFieldDay(log)
    }

    fn make_qso_with_exchange(exchange: &str) -> Qso {
        Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
            String::new(),
            None,
            Some(exchange.to_string()),
            None,
        )
        .unwrap()
    }

    fn make_qso_with_exchange_and_freq(exchange: &str, freq: u32) -> Qso {
        Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
            String::new(),
            None,
            Some(exchange.to_string()),
            Some(freq),
        )
        .unwrap()
    }

    // --- Field Day ADIF tests ---

    #[test]
    fn field_day_qso_contains_contest_id() {
        let record = format_qso(&make_fd_log(), &make_qso()).unwrap();
        // "ARRL-FIELD-DAY" is 14 characters
        assert!(
            record.contains("<CONTEST_ID:14>ARRL-FIELD-DAY"),
            "FD record must contain CONTEST_ID"
        );
    }

    #[test]
    fn field_day_qso_contains_stx_string() {
        let record = format_qso(&make_fd_log(), &make_qso()).unwrap();
        // Log is "1B EPA" (1 tx, class B, section EPA)
        assert!(
            record.contains("STX_STRING"),
            "FD record must contain STX_STRING"
        );
        assert!(
            record.contains("1B EPA"),
            "STX_STRING should match sent exchange"
        );
    }

    #[test]
    fn field_day_qso_with_exchange_contains_srx_string() {
        let record = format_qso(&make_fd_log(), &make_qso_with_exchange("3A CT")).unwrap();
        assert!(
            record.contains("SRX_STRING"),
            "FD record with exchange should contain SRX_STRING"
        );
        assert!(
            record.contains("3A CT"),
            "SRX_STRING should match received exchange"
        );
    }

    #[test]
    fn field_day_qso_without_exchange_omits_srx_string() {
        let record = format_qso(&make_fd_log(), &make_qso()).unwrap();
        assert!(
            !record.contains("SRX_STRING"),
            "FD record without exchange must not contain SRX_STRING"
        );
    }

    // --- Winter Field Day ADIF tests ---

    #[test]
    fn wfd_qso_contains_contest_id() {
        let record = format_qso(&make_wfd_log(), &make_qso()).unwrap();
        assert!(
            record.contains("<CONTEST_ID:3>WFD"),
            "WFD record must contain CONTEST_ID"
        );
    }

    #[test]
    fn wfd_qso_contains_stx_and_srx() {
        let record = format_qso(&make_wfd_log(), &make_qso_with_exchange("2H EPA")).unwrap();
        assert!(record.contains("STX_STRING"), "WFD must contain STX_STRING");
        assert!(record.contains("SRX_STRING"), "WFD must contain SRX_STRING");
        assert!(
            record.contains("2H EPA"),
            "SRX_STRING should match exchange"
        );
    }

    #[test]
    fn fd_qso_with_frequency_contains_freq_field() {
        let record = format_qso(
            &make_fd_log(),
            &make_qso_with_exchange_and_freq("3A CT", 14225),
        )
        .unwrap();
        assert!(
            record.contains("<FREQ:"),
            "FD record with frequency must contain FREQ"
        );
        assert!(
            record.contains("14.225"),
            "FREQ must be emitted in MHz (not kHz)"
        );
    }

    #[test]
    fn fd_qso_without_frequency_omits_freq() {
        let record = format_qso(&make_fd_log(), &make_qso_with_exchange("3A CT")).unwrap();
        assert!(
            !record.contains("<FREQ:"),
            "FD record without frequency must not emit FREQ"
        );
    }

    #[test]
    fn wfd_qso_with_frequency_contains_freq_field() {
        let record = format_qso(
            &make_wfd_log(),
            &make_qso_with_exchange_and_freq("2H EPA", 14225),
        )
        .unwrap();
        assert!(
            record.contains("<FREQ:"),
            "WFD record with frequency must contain FREQ"
        );
        // frequency stored as kHz; ADIF FREQ is in MHz
        assert!(
            record.contains("14.225"),
            "FREQ must be emitted in MHz (not kHz)"
        );
    }

    #[test]
    fn wfd_qso_without_frequency_omits_freq() {
        let record = format_qso(&make_wfd_log(), &make_qso_with_exchange("2H EPA")).unwrap();
        assert!(
            !record.contains("<FREQ:"),
            "WFD record without frequency must not emit FREQ"
        );
    }

    fn make_qso_with_freq(freq: u32) -> Qso {
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
            Some(freq),
        )
        .unwrap()
    }

    fn make_general_log() -> Log {
        Log::General(GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap())
    }

    // --- General / POTA FREQ tests ---

    #[test]
    fn general_qso_with_frequency_emits_freq() {
        let log = make_general_log();
        let record = format_qso(&log, &make_qso_with_freq(14225)).unwrap();
        assert!(
            record.contains("<FREQ:"),
            "general QSO with freq must emit FREQ"
        );
        assert!(record.contains("14.225"), "FREQ must be in MHz");
    }

    #[test]
    fn pota_qso_with_frequency_emits_freq() {
        let record = format_qso(&make_log(), &make_qso_with_freq(7200)).unwrap();
        assert!(
            record.contains("<FREQ:"),
            "POTA QSO with freq must emit FREQ"
        );
        assert!(record.contains("7.200"), "FREQ must be in MHz");
    }

    #[test]
    fn general_qso_without_frequency_omits_freq() {
        let log = make_general_log();
        let record = format_qso(&log, &make_qso()).unwrap();
        assert!(
            !record.contains("<FREQ:"),
            "general QSO without freq must not emit FREQ"
        );
    }

    #[test]
    fn non_pota_qso_with_their_park_excludes_sig_info() {
        // General log with a QSO that has their_park set — should not emit SIG/SIG_INFO
        let log =
            Log::General(GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap());
        // Directly set their_park, bypassing Qso::new validation for this test
        let mut qso = make_qso();
        qso.their_park = Some("K-1234".to_string());
        let record = format_qso(&log, &qso).unwrap();
        assert!(
            !record.contains("<SIG:"),
            "general log must not emit SIG even when their_park is set"
        );
        assert!(
            !record.contains("<SIG_INFO:"),
            "general log must not emit SIG_INFO even when their_park is set"
        );
    }

    #[test]
    fn field_day_log_excludes_pota_sig_fields() {
        let log = Log::FieldDay(
            crate::model::FieldDayLog::new(
                "W1AW".to_string(),
                None,
                1,
                crate::model::FdClass::B,
                "EPA".to_string(),
                crate::model::FdPowerCategory::Low,
                "FN31".to_string(),
            )
            .unwrap(),
        );
        let record = format_qso(&log, &make_qso()).unwrap();
        assert!(
            !record.contains("MY_SIG"),
            "field day log must not emit MY_SIG"
        );
        assert!(
            !record.contains("SIG_INFO"),
            "field day log must not emit SIG_INFO"
        );
    }

    #[test]
    fn wfd_log_excludes_pota_sig_fields() {
        let log = Log::WinterFieldDay(
            crate::model::WfdLog::new(
                "W1AW".to_string(),
                None,
                1,
                crate::model::WfdClass::H,
                "EPA".to_string(),
                "FN31".to_string(),
            )
            .unwrap(),
        );
        let record = format_qso(&log, &make_qso()).unwrap();
        assert!(!record.contains("MY_SIG"), "WFD log must not emit MY_SIG");
        assert!(
            !record.contains("SIG_INFO"),
            "WFD log must not emit SIG_INFO"
        );
    }

    #[test]
    fn adif_header_precedes_records() {
        let mut log = make_log();
        log.add_qso(make_qso());

        let output = format_adif(&log).unwrap();
        let eoh_pos = output.find("<eoh>").unwrap();
        let eor_pos = output.find("<eor>").unwrap();
        assert!(eoh_pos < eor_pos);
    }
}
