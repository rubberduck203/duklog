use chrono::Utc;

use super::LogHeader;
use crate::model::validation::{
    ValidationError, validate_callsign, validate_grid_square, validate_park_ref,
};

/// POTA (Parks on the Air) activation log.
#[derive(Debug, Clone, PartialEq)]
pub struct PotaLog {
    pub(crate) header: LogHeader,
    pub(crate) park_ref: Option<String>,
}

impl PotaLog {
    /// Creates a new POTA log, validating all fields.
    ///
    /// When `operator` is `Some`, it is validated as a callsign. `None` means
    /// the operator is the same as the station callsign (the common solo case).
    ///
    /// Generates `log_id` as `"{park_ref}-{YYYYMMDD-HHMMSS}"` when a park ref
    /// is provided, or `"{callsign}-{YYYYMMDD-HHMMSS}"` otherwise.
    pub fn new(
        station_callsign: String,
        operator: Option<String>,
        park_ref: Option<String>,
        grid_square: String,
    ) -> Result<Self, ValidationError> {
        validate_callsign(&station_callsign)?;
        if let Some(ref op) = operator {
            validate_callsign(op)?;
        }
        if let Some(ref park) = park_ref {
            validate_park_ref(park)?;
        }
        validate_grid_square(&grid_square)?;

        let now = Utc::now();
        let id_prefix = park_ref.as_deref().unwrap_or(&station_callsign);
        let log_id = format!("{}-{}", id_prefix, now.format("%Y%m%d-%H%M%S"));

        Ok(Self {
            header: LogHeader {
                station_callsign,
                operator,
                grid_square,
                qsos: Vec::new(),
                created_at: now,
                log_id,
            },
            park_ref,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Log, PotaLog, ValidationError};

    #[test]
    fn display_label_with_park_returns_park_ref() {
        let log = Log::Pota(
            PotaLog::new(
                "W1AW".to_string(),
                Some("W1AW".to_string()),
                Some("K-0001".to_string()),
                "FN31".to_string(),
            )
            .unwrap(),
        );
        assert_eq!(log.display_label(), "K-0001");
    }

    #[test]
    fn display_label_pota_without_park_returns_callsign() {
        let log =
            Log::Pota(PotaLog::new("W1AW".to_string(), None, None, "FN31".to_string()).unwrap());
        assert_eq!(log.display_label(), "W1AW");
    }

    #[test]
    fn valid_pota_log_creation_with_park() {
        let log = Log::Pota(
            PotaLog::new(
                "W1AW".to_string(),
                Some("W1AW".to_string()),
                Some("K-0001".to_string()),
                "FN31".to_string(),
            )
            .unwrap(),
        );
        assert_eq!(log.header().station_callsign, "W1AW");
        assert_eq!(log.header().operator, Some("W1AW".to_string()));
        assert_eq!(log.park_ref(), Some("K-0001"));
        assert_eq!(log.header().grid_square, "FN31");
        assert_eq!(log.header().qsos.len(), 0);
        assert!(log.header().log_id.starts_with("K-0001-"));
    }

    #[test]
    fn valid_pota_log_creation_without_park() {
        let log = Log::Pota(
            PotaLog::new(
                "W1AW".to_string(),
                Some("W1AW".to_string()),
                None,
                "FN31".to_string(),
            )
            .unwrap(),
        );
        assert_eq!(log.header().operator, Some("W1AW".to_string()));
        assert_eq!(log.park_ref(), None);
        assert!(log.header().log_id.starts_with("W1AW-"));
    }

    #[test]
    fn invalid_station_callsign() {
        let result = PotaLog::new(
            String::new(),
            Some("W1AW".to_string()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::EmptyCallsign));
    }

    #[test]
    fn invalid_operator() {
        let result = PotaLog::new(
            "W1AW".to_string(),
            Some(String::new()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::EmptyCallsign));
    }

    #[test]
    fn none_operator_succeeds() {
        let log = Log::Pota(
            PotaLog::new(
                "W1AW".to_string(),
                None,
                Some("K-0001".to_string()),
                "FN31".to_string(),
            )
            .unwrap(),
        );
        assert_eq!(log.header().operator, None);
    }

    #[test]
    fn invalid_park_ref() {
        let result = PotaLog::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            Some("bad".to_string()),
            "FN31".to_string(),
        );
        assert_eq!(
            result,
            Err(ValidationError::InvalidParkRef("bad".to_string()))
        );
    }

    #[test]
    fn invalid_grid_square() {
        let result = PotaLog::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            Some("K-0001".to_string()),
            "ZZ99".to_string(),
        );
        assert_eq!(
            result,
            Err(ValidationError::InvalidGridSquare("ZZ99".to_string()))
        );
    }
}
