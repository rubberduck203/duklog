use std::fmt;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::LogHeader;
use crate::model::validation::{
    ValidationError, validate_callsign, validate_grid_square, validate_section, validate_tx_count,
};

/// ARRL Field Day operating class (sent as part of every QSO exchange).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FdClass {
    /// Club or non-club group, 3+ persons, portable.
    A,
    /// 1–2 person portable.
    B,
    /// Mobile (vehicle, maritime, aeronautical).
    C,
    /// Home station on commercial power.
    D,
    /// Home station on emergency/alternative power only.
    E,
    /// Emergency Operations Center.
    F,
}

impl fmt::Display for FdClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
        };
        f.write_str(s)
    }
}

/// Field Day power category, which determines the QSO point multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FdPowerCategory {
    /// ≤5 W non-commercial power (×5 multiplier).
    Qrp,
    /// ≤100 W any source (×2 multiplier).
    Low,
    /// >100 W (×1 multiplier).
    High,
}

/// ARRL Field Day log.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDayLog {
    pub(crate) header: LogHeader,
    pub(crate) tx_count: u8,
    pub(crate) class: FdClass,
    pub(crate) section: String,
    pub(crate) power: FdPowerCategory,
}

impl FieldDayLog {
    /// Creates a new Field Day log, validating all fields.
    ///
    /// `section` is stored as-is (callers should normalise to uppercase).
    /// `tx_count` must be ≥ 1. `section` must be non-empty.
    ///
    /// Generates `log_id` as `"FD-{callsign}-{YYYYMMDD-HHMMSS}"`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        station_callsign: String,
        operator: Option<String>,
        tx_count: u8,
        class: FdClass,
        section: String,
        power: FdPowerCategory,
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
        let log_id = format!("FD-{}-{}", station_callsign, now.format("%Y%m%d-%H%M%S"));

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
            power,
        })
    }

    /// Returns the sent exchange string, e.g. `"1B EPA"`.
    pub(crate) fn sent_exchange(&self) -> String {
        format!("{}{} {}", self.tx_count, self.class, self.section)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{FdClass, FdPowerCategory, FieldDayLog, Log, ValidationError};

    #[test]
    fn valid_field_day_log_creation() {
        let log = Log::FieldDay(
            FieldDayLog::new(
                "W1AW".to_string(),
                Some("KD9XYZ".to_string()),
                3,
                FdClass::A,
                "EPA".to_string(),
                FdPowerCategory::High,
                "FN31".to_string(),
            )
            .unwrap(),
        );
        assert_eq!(log.header().station_callsign, "W1AW");
        assert_eq!(log.header().operator, Some("KD9XYZ".to_string()));
        assert_eq!(log.park_ref(), None);
        assert!(log.header().log_id.starts_with("FD-W1AW-"));
        assert!(!log.is_activated());
        assert_eq!(log.needs_for_activation(), 0);
    }

    #[test]
    fn field_day_zero_tx_count_rejected() {
        let result = FieldDayLog::new(
            "W1AW".to_string(),
            None,
            0,
            FdClass::B,
            "EPA".to_string(),
            FdPowerCategory::Low,
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::InvalidTxCount));
    }

    #[test]
    fn field_day_empty_section_rejected() {
        let result = FieldDayLog::new(
            "W1AW".to_string(),
            None,
            1,
            FdClass::B,
            String::new(),
            FdPowerCategory::Low,
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::EmptySection));
    }
}
