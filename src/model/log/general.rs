use chrono::Utc;

use super::LogHeader;
use crate::model::validation::{ValidationError, validate_callsign, validate_grid_square};

/// General-purpose log — no type-specific setup fields.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralLog {
    pub(crate) header: LogHeader,
}

impl GeneralLog {
    /// Creates a new general log, validating all fields.
    ///
    /// When `operator` is `Some`, it is validated as a callsign. `None` means
    /// the operator is the same as the station callsign (the common solo case).
    ///
    /// Generates `log_id` as `"{callsign}-{YYYYMMDD-HHMMSS}"`.
    pub fn new(
        station_callsign: String,
        operator: Option<String>,
        grid_square: String,
    ) -> Result<Self, ValidationError> {
        validate_callsign(&station_callsign)?;
        if let Some(ref op) = operator {
            validate_callsign(op)?;
        }
        validate_grid_square(&grid_square)?;

        let now = Utc::now();
        let log_id = format!("{}-{}", station_callsign, now.format("%Y%m%d-%H%M%S"));

        Ok(Self {
            header: LogHeader {
                station_callsign,
                operator,
                grid_square,
                qsos: Vec::new(),
                created_at: now,
                log_id,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{GeneralLog, Log};

    #[test]
    fn valid_general_log_creation() {
        let log =
            Log::General(GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap());
        assert_eq!(log.header().station_callsign, "W1AW");
        assert_eq!(log.park_ref(), None);
        assert!(log.header().log_id.starts_with("W1AW-"));
    }
}
