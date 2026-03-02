use std::fmt;
use std::sync::LazyLock;

use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::LogHeader;
use crate::model::validation::{
    ValidationError, validate_callsign, validate_grid_square, validate_section, validate_tx_count,
};

static FD_EXCHANGE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+[A-F] \S+$").expect("valid hardcoded regex"));

/// Validates a Field Day received exchange string (e.g. `"3A CT"`, `"1F DX"`).
///
/// Format: one or more digits, a class letter A–F, a space, then a non-whitespace section.
pub fn validate_fd_exchange(s: &str) -> Result<(), ValidationError> {
    if FD_EXCHANGE_RE.is_match(s) {
        Ok(())
    } else {
        Err(ValidationError::InvalidFdExchange(s.to_string()))
    }
}

/// Parses a Field Day class from a string.
///
/// Accepts `"A"`–`"F"` (case-insensitive). Returns an error for any other value.
pub fn parse_fd_class(s: &str) -> Result<FdClass, ValidationError> {
    match s.to_uppercase().as_str() {
        "A" => Ok(FdClass::A),
        "B" => Ok(FdClass::B),
        "C" => Ok(FdClass::C),
        "D" => Ok(FdClass::D),
        "E" => Ok(FdClass::E),
        "F" => Ok(FdClass::F),
        _ => Err(ValidationError::InvalidFdClass(s.to_string())),
    }
}

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
    use chrono::{TimeZone, Utc};
    use quickcheck_macros::quickcheck;

    use crate::model::band::Band;
    use crate::model::mode::Mode;
    use crate::model::qso::Qso;
    use crate::model::{FdClass, FdPowerCategory, FieldDayLog, Log, ValidationError};

    use super::{parse_fd_class, validate_fd_exchange};

    impl quickcheck::Arbitrary for FdClass {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            *g.choose(&[
                FdClass::A,
                FdClass::B,
                FdClass::C,
                FdClass::D,
                FdClass::E,
                FdClass::F,
            ])
            .unwrap()
        }
    }

    // --- parse_fd_class ---

    #[test]
    fn parse_fd_class_valid_letters() {
        assert_eq!(parse_fd_class("A"), Ok(FdClass::A));
        assert_eq!(parse_fd_class("B"), Ok(FdClass::B));
        assert_eq!(parse_fd_class("C"), Ok(FdClass::C));
        assert_eq!(parse_fd_class("D"), Ok(FdClass::D));
        assert_eq!(parse_fd_class("E"), Ok(FdClass::E));
        assert_eq!(parse_fd_class("F"), Ok(FdClass::F));
    }

    #[test]
    fn parse_fd_class_lowercase_accepted() {
        assert_eq!(parse_fd_class("a"), Ok(FdClass::A));
        assert_eq!(parse_fd_class("f"), Ok(FdClass::F));
    }

    #[test]
    fn parse_fd_class_invalid_returns_err() {
        assert_eq!(
            parse_fd_class("G"),
            Err(ValidationError::InvalidFdClass("G".to_string()))
        );
    }

    #[quickcheck]
    fn parse_fd_class_invalid_string_returns_err(s: String) -> bool {
        if !s.is_ascii() {
            return true;
        }
        let upper = s.to_uppercase();
        let valid = matches!(upper.as_str(), "A" | "B" | "C" | "D" | "E" | "F");
        if valid {
            return true; // skip valid inputs
        }
        parse_fd_class(&s).is_err()
    }

    #[quickcheck]
    fn parse_fd_class_round_trip(class: FdClass) -> bool {
        parse_fd_class(&class.to_string()) == Ok(class)
    }

    #[quickcheck]
    fn parse_fd_class_lowercase_round_trip(class: FdClass) -> bool {
        parse_fd_class(&class.to_string().to_lowercase()) == Ok(class)
    }

    #[test]
    fn display_label_field_day_returns_exchange() {
        let log = Log::FieldDay(
            FieldDayLog::new(
                "W1AW".to_string(),
                None,
                1,
                FdClass::B,
                "EPA".to_string(),
                FdPowerCategory::Low,
                "FN31".to_string(),
            )
            .unwrap(),
        );
        assert_eq!(log.display_label(), "1B EPA");
    }

    #[test]
    fn field_day_find_duplicates_spans_all_dates() {
        let mut log = Log::FieldDay(
            FieldDayLog::new(
                "W1AW".to_string(),
                None,
                1,
                FdClass::B,
                "EPA".to_string(),
                FdPowerCategory::Low,
                "FN31".to_string(),
            )
            .unwrap(),
        );
        let yesterday = Utc::now().date_naive().pred_opt().unwrap();
        let old_ts = Utc.from_utc_datetime(&yesterday.and_hms_opt(12, 0, 0).unwrap());
        let old_qso = Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            old_ts,
            String::new(),
            None,
            None,
            None,
        )
        .unwrap();
        log.add_qso(old_qso);

        let candidate = Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc::now(),
            String::new(),
            None,
            None,
            None,
        )
        .unwrap();
        // FD logs scope duplicates across ALL dates — yesterday's QSO is found
        assert_eq!(log.find_duplicates(&candidate).len(), 1);
    }

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

    // --- validate_fd_exchange ---

    #[test]
    fn fd_exchange_valid_all_classes() {
        for cls in &["A", "B", "C", "D", "E", "F"] {
            let s = format!("3{cls} CT");
            assert_eq!(validate_fd_exchange(&s), Ok(()), "should accept {s}");
        }
    }

    #[test]
    fn fd_exchange_valid_multi_tx() {
        assert_eq!(validate_fd_exchange("12A EPA"), Ok(()));
    }

    #[test]
    fn fd_exchange_valid_dx_section() {
        assert_eq!(validate_fd_exchange("1F DX"), Ok(()));
    }

    #[test]
    fn fd_exchange_empty_is_invalid() {
        assert!(validate_fd_exchange("").is_err());
    }

    #[test]
    fn fd_exchange_wrong_class_letter() {
        assert_eq!(
            validate_fd_exchange("3Z CT"),
            Err(ValidationError::InvalidFdExchange("3Z CT".to_string()))
        );
    }

    #[test]
    fn fd_exchange_no_space() {
        assert!(validate_fd_exchange("3ACT").is_err());
    }

    #[test]
    fn fd_exchange_no_section() {
        assert!(validate_fd_exchange("3A ").is_err());
    }

    #[test]
    fn fd_exchange_no_count() {
        assert!(validate_fd_exchange("A CT").is_err());
    }

    #[quickcheck]
    fn fd_exchange_valid_constructed_always_accepted(
        count: u8,
        cls_idx: u8,
        section: String,
    ) -> bool {
        if !section.is_ascii() {
            return true;
        }
        let section = section.trim();
        if section.is_empty() || section.contains(char::is_whitespace) {
            return true;
        }
        let count = (count % 127) + 1; // 1-127
        let cls = ['A', 'B', 'C', 'D', 'E', 'F'][(cls_idx % 6) as usize];
        let exchange = format!("{count}{cls} {section}");
        validate_fd_exchange(&exchange).is_ok()
    }

    #[quickcheck]
    fn fd_exchange_wrong_class_letter_rejected(s: String) -> bool {
        if !s.is_ascii() {
            return true;
        }
        // Build an exchange with a class letter outside A-F
        let invalid = format!("1Z {s}");
        validate_fd_exchange(&invalid).is_err()
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
