use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::band::Band;
use super::mode::Mode;
use super::validation::{ValidationError, validate_callsign, validate_park_ref};

/// A single contact (QSO) record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Qso {
    pub their_call: String,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub band: Band,
    pub mode: Mode,
    pub timestamp: DateTime<Utc>,
    pub comments: String,
    pub their_park: Option<String>,
}

impl Qso {
    /// Creates a new QSO, validating the callsign and optional park reference.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        their_call: String,
        rst_sent: String,
        rst_rcvd: String,
        band: Band,
        mode: Mode,
        timestamp: DateTime<Utc>,
        comments: String,
        their_park: Option<String>,
    ) -> Result<Self, ValidationError> {
        validate_callsign(&their_call)?;
        if let Some(ref park) = their_park {
            validate_park_ref(park)?;
        }
        Ok(Self {
            their_call,
            rst_sent,
            rst_rcvd,
            band,
            mode,
            timestamp,
            comments,
            their_park,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn make_qso() -> Qso {
        Qso::new(
            "W1AW".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc::now(),
            String::new(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn valid_qso() {
        let qso = make_qso();
        assert_eq!(qso.their_call, "W1AW");
        assert_eq!(qso.rst_sent, "59");
        assert_eq!(qso.rst_rcvd, "59");
        assert_eq!(qso.band, Band::M20);
        assert_eq!(qso.mode, Mode::Ssb);
        assert_eq!(qso.comments, "");
        assert_eq!(qso.their_park, None);
    }

    #[test]
    fn valid_p2p_qso() {
        let qso = Qso::new(
            "KD9XYZ".to_string(),
            "-10".to_string(),
            "-15".to_string(),
            Band::M40,
            Mode::Ft8,
            Utc::now(),
            "P2P".to_string(),
            Some("K-1234".to_string()),
        )
        .unwrap();
        assert_eq!(qso.their_call, "KD9XYZ");
        assert_eq!(qso.their_park, Some("K-1234".to_string()));
        assert_eq!(qso.comments, "P2P");
    }

    #[test]
    fn empty_callsign_rejected() {
        let result = Qso::new(
            String::new(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc::now(),
            String::new(),
            None,
        );
        assert_eq!(result, Err(ValidationError::EmptyCallsign));
    }

    #[test]
    fn invalid_park_ref_rejected() {
        let result = Qso::new(
            "W1AW".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc::now(),
            String::new(),
            Some("bad".to_string()),
        );
        assert_eq!(
            result,
            Err(ValidationError::InvalidParkRef("bad".to_string()))
        );
    }

    #[test]
    fn field_values_preserved() {
        let ts = Utc::now();
        let qso = Qso::new(
            "N0CALL/P".to_string(),
            "599".to_string(),
            "579".to_string(),
            Band::M40,
            Mode::Cw,
            ts,
            "test comment".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(qso.their_call, "N0CALL/P");
        assert_eq!(qso.rst_sent, "599");
        assert_eq!(qso.rst_rcvd, "579");
        assert_eq!(qso.band, Band::M40);
        assert_eq!(qso.mode, Mode::Cw);
        assert_eq!(qso.timestamp, ts);
        assert_eq!(qso.comments, "test comment");
    }

    #[test]
    fn serde_round_trip() {
        let qso = make_qso();
        let json = serde_json::to_string(&qso).unwrap();
        let deserialized: Qso = serde_json::from_str(&json).unwrap();
        assert_eq!(qso, deserialized);
    }
}
