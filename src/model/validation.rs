use std::sync::LazyLock;

use regex::Regex;
use thiserror::Error;

/// Validation errors for domain model fields.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("callsign cannot be empty")]
    EmptyCallsign,
    #[error("invalid callsign: {0}")]
    InvalidCallsign(String),
    #[error("invalid park reference: {0}")]
    InvalidParkRef(String),
    #[error("invalid grid square: {0}")]
    InvalidGridSquare(String),
}

static PARK_REF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]{1,3}-\d{4,5}$").expect("valid hardcoded regex"));

static GRID_SQUARE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-R]{2}[0-9]{2}([a-x]{2})?$").expect("valid hardcoded regex"));

/// Validates a callsign: must be non-empty and contain only ASCII alphanumeric characters or `/`.
pub fn validate_callsign(callsign: &str) -> Result<(), ValidationError> {
    match callsign {
        "" => Err(ValidationError::EmptyCallsign),
        s if s.chars().all(|c| c.is_ascii_alphanumeric() || c == '/') => Ok(()),
        _ => Err(ValidationError::InvalidCallsign(callsign.to_string())),
    }
}

/// Validates a POTA park reference (e.g., `K-0001`, `VE-01234`).
pub fn validate_park_ref(park_ref: &str) -> Result<(), ValidationError> {
    if PARK_REF_RE.is_match(park_ref) {
        Ok(())
    } else {
        Err(ValidationError::InvalidParkRef(park_ref.to_string()))
    }
}

/// Validates a Maidenhead grid square (e.g., `FN31` or `FN31pr`).
pub fn validate_grid_square(grid: &str) -> Result<(), ValidationError> {
    if GRID_SQUARE_RE.is_match(grid) {
        Ok(())
    } else {
        Err(ValidationError::InvalidGridSquare(grid.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use quickcheck_macros::quickcheck;

    use super::*;

    // --- validate_callsign ---

    #[test]
    fn callsign_simple() {
        assert_eq!(validate_callsign("W1AW"), Ok(()));
    }

    #[test]
    fn callsign_with_slash() {
        assert_eq!(validate_callsign("W1AW/P"), Ok(()));
    }

    #[test]
    fn callsign_empty() {
        assert_eq!(validate_callsign(""), Err(ValidationError::EmptyCallsign));
    }

    #[test]
    fn callsign_invalid_chars() {
        assert_eq!(
            validate_callsign("W1 AW"),
            Err(ValidationError::InvalidCallsign("W1 AW".to_string()))
        );
    }

    #[quickcheck]
    fn callsign_nonempty_alnum_slash_is_valid(s: String) -> bool {
        if s.is_empty() {
            return true; // skip empty
        }
        let filtered: String = s
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '/')
            .collect();
        if filtered.is_empty() {
            return true; // skip if no valid chars
        }
        validate_callsign(&filtered).is_ok()
    }

    // --- validate_park_ref ---

    #[test]
    fn park_ref_us() {
        assert_eq!(validate_park_ref("K-0001"), Ok(()));
    }

    #[test]
    fn park_ref_canada() {
        assert_eq!(validate_park_ref("VE-01234"), Ok(()));
    }

    #[test]
    fn park_ref_three_letter() {
        assert_eq!(validate_park_ref("JAS-1234"), Ok(()));
    }

    #[test]
    fn park_ref_too_few_digits() {
        assert_eq!(
            validate_park_ref("K-001"),
            Err(ValidationError::InvalidParkRef("K-001".to_string()))
        );
    }

    #[test]
    fn park_ref_too_many_digits() {
        assert_eq!(
            validate_park_ref("K-123456"),
            Err(ValidationError::InvalidParkRef("K-123456".to_string()))
        );
    }

    #[test]
    fn park_ref_lowercase() {
        assert_eq!(
            validate_park_ref("k-0001"),
            Err(ValidationError::InvalidParkRef("k-0001".to_string()))
        );
    }

    #[test]
    fn park_ref_empty() {
        assert_eq!(
            validate_park_ref(""),
            Err(ValidationError::InvalidParkRef(String::new()))
        );
    }

    #[test]
    fn park_ref_no_dash() {
        assert_eq!(
            validate_park_ref("K0001"),
            Err(ValidationError::InvalidParkRef("K0001".to_string()))
        );
    }

    #[quickcheck]
    fn park_ref_valid_format_always_accepted(prefix_len: u8, num: u32) -> bool {
        let prefix_len = (prefix_len % 3) + 1; // 1-3
        let prefix: String = (0..prefix_len).map(|i| (b'A' + (i % 26)) as char).collect();
        let num = (num % 90000) + 1000; // 4-5 digits
        let park_ref = format!("{prefix}-{num:04}");
        validate_park_ref(&park_ref).is_ok()
    }

    // --- validate_grid_square ---

    #[test]
    fn grid_four_char() {
        assert_eq!(validate_grid_square("FN31"), Ok(()));
    }

    #[test]
    fn grid_six_char() {
        assert_eq!(validate_grid_square("FN31pr"), Ok(()));
    }

    #[test]
    fn grid_boundary_values() {
        assert_eq!(validate_grid_square("AA00"), Ok(()));
        assert_eq!(validate_grid_square("RR99"), Ok(()));
        assert_eq!(validate_grid_square("AA00aa"), Ok(()));
        assert_eq!(validate_grid_square("RR99xx"), Ok(()));
    }

    #[test]
    fn grid_out_of_range_field() {
        assert_eq!(
            validate_grid_square("SA00"),
            Err(ValidationError::InvalidGridSquare("SA00".to_string()))
        );
    }

    #[test]
    fn grid_uppercase_subsquare() {
        assert_eq!(
            validate_grid_square("FN31PR"),
            Err(ValidationError::InvalidGridSquare("FN31PR".to_string()))
        );
    }

    #[test]
    fn grid_five_chars() {
        assert_eq!(
            validate_grid_square("FN31p"),
            Err(ValidationError::InvalidGridSquare("FN31p".to_string()))
        );
    }

    #[test]
    fn grid_empty() {
        assert_eq!(
            validate_grid_square(""),
            Err(ValidationError::InvalidGridSquare(String::new()))
        );
    }

    #[test]
    fn grid_subsquare_out_of_range() {
        assert_eq!(
            validate_grid_square("FN31yy"),
            Err(ValidationError::InvalidGridSquare("FN31yy".to_string()))
        );
    }

    #[quickcheck]
    fn grid_valid_four_char_always_accepted(f1: u8, f2: u8, d1: u8, d2: u8) -> bool {
        let f1 = (f1 % 18) + b'A'; // A-R
        let f2 = (f2 % 18) + b'A';
        let d1 = (d1 % 10) + b'0';
        let d2 = (d2 % 10) + b'0';
        let grid = format!("{}{}{}{}", f1 as char, f2 as char, d1 as char, d2 as char);
        validate_grid_square(&grid).is_ok()
    }
}
