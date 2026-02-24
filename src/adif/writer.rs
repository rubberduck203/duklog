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

/// Encodes log-type-specific ADIF fields (currently POTA).
///
/// Emits `MY_SIG`/`MY_SIG_INFO` when the log has a park reference, and
/// `SIG`/`SIG_INFO` when the QSO has their park set.
fn encode_type_specific_fields(
    encoder: &mut TagEncoder,
    buf: &mut BytesMut,
    log: &Log,
    qso: &Qso,
) -> Result<(), AdifError> {
    if let Some(ref park) = log.park_ref {
        encode(encoder, buf, field_tag("MY_SIG", POTA_SIG))?;
        encode(encoder, buf, field_tag("MY_SIG_INFO", park.as_str()))?;
    }

    if let Some(ref their_park) = qso.their_park {
        encode(encoder, buf, field_tag("SIG", POTA_SIG))?;
        encode(encoder, buf, field_tag("SIG_INFO", their_park.as_str()))?;
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

    let timestamp = log.created_at.format("%Y%m%d %H%M%S").to_string();
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
/// are present.
pub fn format_qso(log: &Log, qso: &Qso) -> Result<String, AdifError> {
    let mut encoder = TagEncoder::new();
    let mut buf = BytesMut::new();

    encode(
        &mut encoder,
        &mut buf,
        field_tag("STATION_CALLSIGN", log.station_callsign.as_str()),
    )?;
    if let Some(ref op) = log.operator
        && op != &log.station_callsign
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
    encode(
        &mut encoder,
        &mut buf,
        field_tag("MY_GRIDSQUARE", log.grid_square.as_str()),
    )?;

    encode_type_specific_fields(&mut encoder, &mut buf, log, qso)?;

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
    log.qsos
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
    use crate::model::{Band, Mode};

    fn make_log() -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log
    }

    fn make_log_without_park() -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            None,
            "FN31".to_string(),
        )
        .unwrap();
        log.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log
    }

    fn make_log_distinct_operator() -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            Some("N0CALL".to_string()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log
    }

    fn make_log_none_operator() -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            None,
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log
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
    fn qso_without_park_excludes_my_sig() {
        let record = format_qso(&make_log_without_park(), &make_qso()).unwrap();

        assert!(!record.contains("MY_SIG"));
        assert!(!record.contains("MY_SIG_INFO"));
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
        ) {
            Ok(q) => q,
            Err(_) => return true,
        };
        let record = format_qso(&make_log_without_park(), &qso).unwrap();
        let expected = format!("<CALL:{}>{}", call.len(), call);
        record.contains(&expected)
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
