use std::fmt;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::LogHeader;
use crate::model::validation::{
    ValidationError, validate_callsign, validate_grid_square, validate_section, validate_tx_count,
};

/// Winter Field Day operating class (sent as part of every QSO exchange).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WfdClass {
    /// Home — inside a permanent livable residence.
    H,
    /// Indoor — weather-protected building on permanent foundation.
    I,
    /// Outdoor — partly or fully exposed shelter.
    O,
    /// Mobile — RV, car, van, boat, or similar.
    M,
}

impl fmt::Display for WfdClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::H => "H",
            Self::I => "I",
            Self::O => "O",
            Self::M => "M",
        };
        f.write_str(s)
    }
}

/// Winter Field Day log.
#[derive(Debug, Clone, PartialEq)]
pub struct WfdLog {
    pub(crate) header: LogHeader,
    pub(crate) tx_count: u8,
    pub(crate) class: WfdClass,
    pub(crate) section: String,
}

impl WfdLog {
    /// Creates a new Winter Field Day log, validating all fields.
    ///
    /// `section` is stored as-is (callers should normalise to uppercase).
    /// `tx_count` must be ≥ 1. `section` must be non-empty.
    ///
    /// Generates `log_id` as `"WFD-{callsign}-{YYYYMMDD-HHMMSS}"`.
    pub fn new(
        station_callsign: String,
        operator: Option<String>,
        tx_count: u8,
        class: WfdClass,
        section: String,
        grid_square: String,
    ) -> Result<Self, ValidationError> {
        validate_callsign(&station_callsign)?;
        if let Some(ref op) = operator {
            validate_callsign(op)?;
        }
        validate_tx_count(tx_count)?;
        validate_section(&section)?;
        validate_grid_square(&grid_square)?;

        let now = Utc::now();
        let log_id = format!("WFD-{}-{}", station_callsign, now.format("%Y%m%d-%H%M%S"));

        Ok(Self {
            header: LogHeader {
                station_callsign,
                operator,
                grid_square,
                qsos: Vec::new(),
                created_at: now,
                log_id,
            },
            tx_count,
            class,
            section,
        })
    }

    /// Returns the sent exchange string, e.g. `"1H EPA"`.
    pub(crate) fn sent_exchange(&self) -> String {
        format!("{}{} {}", self.tx_count, self.class, self.section)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Log, ValidationError, WfdClass, WfdLog};

    #[test]
    fn valid_wfd_log_creation() {
        let log = Log::WinterFieldDay(
            WfdLog::new(
                "W1AW".to_string(),
                None,
                1,
                WfdClass::O,
                "EPA".to_string(),
                "FN31".to_string(),
            )
            .unwrap(),
        );
        assert_eq!(log.header().station_callsign, "W1AW");
        assert_eq!(log.park_ref(), None);
        assert!(log.header().log_id.starts_with("WFD-W1AW-"));
        assert!(!log.is_activated());
        assert_eq!(log.needs_for_activation(), 0);
    }

    #[test]
    fn wfd_zero_tx_count_rejected() {
        let result = WfdLog::new(
            "W1AW".to_string(),
            None,
            0,
            WfdClass::H,
            "EPA".to_string(),
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::InvalidTxCount));
    }

    #[test]
    fn wfd_empty_section_rejected() {
        let result = WfdLog::new(
            "W1AW".to_string(),
            None,
            1,
            WfdClass::H,
            String::new(),
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::EmptySection));
    }
}
