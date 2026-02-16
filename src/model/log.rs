use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::qso::Qso;
use super::validation::{
    ValidationError, validate_callsign, validate_grid_square, validate_park_ref,
};

/// An activation session containing station info and a collection of QSOs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Log {
    pub station_callsign: String,
    pub operator: String,
    pub park_ref: String,
    pub grid_square: String,
    pub qsos: Vec<Qso>,
    pub created_at: DateTime<Utc>,
    pub log_id: String,
}

impl Log {
    /// Creates a new log, validating all fields.
    ///
    /// Generates `log_id` as `"{park_ref}-{YYYYMMDD-HHMMSS}"`.
    pub fn new(
        station_callsign: String,
        operator: String,
        park_ref: String,
        grid_square: String,
    ) -> Result<Self, ValidationError> {
        validate_callsign(&station_callsign)?;
        validate_callsign(&operator)?;
        validate_park_ref(&park_ref)?;
        validate_grid_square(&grid_square)?;

        let now = Utc::now();
        let log_id = format!("{}-{}", park_ref, now.format("%Y%m%d-%H%M%S"));

        Ok(Self {
            station_callsign,
            operator,
            park_ref,
            grid_square,
            qsos: Vec::new(),
            created_at: now,
            log_id,
        })
    }

    /// Adds a QSO to this log.
    pub fn add_qso(&mut self, qso: Qso) {
        self.qsos.push(qso);
    }

    /// Counts QSOs with timestamps on the given date (UTC).
    pub(crate) fn qso_count_on_date(&self, date: NaiveDate) -> usize {
        self.qsos
            .iter()
            .filter(|q| q.timestamp.date_naive() == date)
            .count()
    }

    /// Counts QSOs logged today (UTC).
    pub fn qso_count_today(&self) -> usize {
        self.qso_count_on_date(Utc::now().date_naive())
    }

    /// Returns the number of additional QSOs needed for a valid POTA activation today.
    pub fn needs_for_activation(&self) -> usize {
        10_usize.saturating_sub(self.qso_count_today())
    }

    /// Returns `true` if this log has at least 10 QSOs today (valid POTA activation).
    pub fn is_activated(&self) -> bool {
        self.qso_count_today() >= 10
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};

    use super::*;
    use crate::model::band::Band;
    use crate::model::mode::Mode;

    fn make_log() -> Log {
        Log::new(
            "W1AW".to_string(),
            "W1AW".to_string(),
            "K-0001".to_string(),
            "FN31".to_string(),
        )
        .unwrap()
    }

    fn make_qso_on_date(date: NaiveDate) -> Qso {
        let timestamp = date
            .and_hms_opt(12, 0, 0)
            .map(|dt| Utc.from_utc_datetime(&dt))
            .unwrap();
        Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            timestamp,
            String::new(),
            None,
        )
        .unwrap()
    }

    // --- Construction validation ---

    #[test]
    fn valid_log_creation() {
        let log = make_log();
        assert_eq!(log.station_callsign, "W1AW");
        assert_eq!(log.operator, "W1AW");
        assert_eq!(log.park_ref, "K-0001");
        assert_eq!(log.grid_square, "FN31");
        assert_eq!(log.qsos.len(), 0);
        assert!(log.log_id.starts_with("K-0001-"));
    }

    #[test]
    fn invalid_station_callsign() {
        let result = Log::new(
            String::new(),
            "W1AW".to_string(),
            "K-0001".to_string(),
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::EmptyCallsign));
    }

    #[test]
    fn invalid_operator() {
        let result = Log::new(
            "W1AW".to_string(),
            String::new(),
            "K-0001".to_string(),
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::EmptyCallsign));
    }

    #[test]
    fn invalid_park_ref() {
        let result = Log::new(
            "W1AW".to_string(),
            "W1AW".to_string(),
            "bad".to_string(),
            "FN31".to_string(),
        );
        assert_eq!(
            result,
            Err(ValidationError::InvalidParkRef("bad".to_string()))
        );
    }

    #[test]
    fn invalid_grid_square() {
        let result = Log::new(
            "W1AW".to_string(),
            "W1AW".to_string(),
            "K-0001".to_string(),
            "ZZ99".to_string(),
        );
        assert_eq!(
            result,
            Err(ValidationError::InvalidGridSquare("ZZ99".to_string()))
        );
    }

    // --- QSO operations ---

    #[test]
    fn add_qso_increments_count() {
        let mut log = make_log();
        assert_eq!(log.qsos.len(), 0);
        let qso = Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc::now(),
            String::new(),
            None,
        )
        .unwrap();
        log.add_qso(qso);
        assert_eq!(log.qsos.len(), 1);
    }

    // --- Activation boundary tests (using qso_count_on_date) ---

    #[test]
    fn qso_count_on_date_filters_correctly() {
        let mut log = make_log();
        let date1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

        log.add_qso(make_qso_on_date(date1));
        log.add_qso(make_qso_on_date(date1));
        log.add_qso(make_qso_on_date(date2));

        assert_eq!(log.qso_count_on_date(date1), 2);
        assert_eq!(log.qso_count_on_date(date2), 1);
    }

    fn make_log_with_n_qsos(n: usize) -> (Log, NaiveDate) {
        let mut log = make_log();
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        for _ in 0..n {
            log.add_qso(make_qso_on_date(date));
        }
        (log, date)
    }

    #[test]
    fn activation_at_9_qsos() {
        let (log, date) = make_log_with_n_qsos(9);
        assert_eq!(log.qso_count_on_date(date), 9);
        // Simulate needs_for_activation / is_activated using qso_count_on_date
        let count = log.qso_count_on_date(date);
        assert_eq!(10_usize.saturating_sub(count), 1);
        assert!(count < 10);
    }

    #[test]
    fn activation_at_10_qsos() {
        let (log, date) = make_log_with_n_qsos(10);
        assert_eq!(log.qso_count_on_date(date), 10);
        let count = log.qso_count_on_date(date);
        assert_eq!(10_usize.saturating_sub(count), 0);
        assert!(count >= 10);
    }

    #[test]
    fn activation_at_11_qsos() {
        let (log, date) = make_log_with_n_qsos(11);
        assert_eq!(log.qso_count_on_date(date), 11);
        let count = log.qso_count_on_date(date);
        assert_eq!(10_usize.saturating_sub(count), 0);
        assert!(count >= 10);
    }

    #[test]
    fn utc_midnight_boundary() {
        let mut log = make_log();
        let date1 = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();

        // 23:59:59 on date1
        let ts_before = Utc.from_utc_datetime(&date1.and_hms_opt(23, 59, 59).unwrap());
        // 00:00:00 on date2
        let ts_after = Utc.from_utc_datetime(&date2.and_hms_opt(0, 0, 0).unwrap());

        let qso1 = Qso::new(
            "W1AW".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            ts_before,
            String::new(),
            None,
        )
        .unwrap();
        let qso2 = Qso::new(
            "W1AW".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            ts_after,
            String::new(),
            None,
        )
        .unwrap();

        log.add_qso(qso1);
        log.add_qso(qso2);

        assert_eq!(log.qso_count_on_date(date1), 1);
        assert_eq!(log.qso_count_on_date(date2), 1);
    }

    // --- qso_count_today / needs_for_activation / is_activated ---

    fn add_today_qsos(log: &mut Log, n: usize) {
        for _ in 0..n {
            let qso = Qso::new(
                "KD9XYZ".to_string(),
                "59".to_string(),
                "59".to_string(),
                Band::M20,
                Mode::Ssb,
                Utc::now(),
                String::new(),
                None,
            )
            .unwrap();
            log.add_qso(qso);
        }
    }

    #[test]
    fn qso_count_today_empty() {
        let log = make_log();
        assert_eq!(log.qso_count_today(), 0);
    }

    #[test]
    fn qso_count_today_with_qsos() {
        let mut log = make_log();
        add_today_qsos(&mut log, 3);
        assert_eq!(log.qso_count_today(), 3);
    }

    #[test]
    fn needs_for_activation_at_0() {
        let log = make_log();
        assert_eq!(log.needs_for_activation(), 10);
    }

    #[test]
    fn needs_for_activation_at_9() {
        let mut log = make_log();
        add_today_qsos(&mut log, 9);
        assert_eq!(log.needs_for_activation(), 1);
    }

    #[test]
    fn needs_for_activation_at_10() {
        let mut log = make_log();
        add_today_qsos(&mut log, 10);
        assert_eq!(log.needs_for_activation(), 0);
    }

    #[test]
    fn needs_for_activation_at_11() {
        let mut log = make_log();
        add_today_qsos(&mut log, 11);
        assert_eq!(log.needs_for_activation(), 0);
    }

    #[test]
    fn is_activated_at_9() {
        let mut log = make_log();
        add_today_qsos(&mut log, 9);
        assert!(!log.is_activated());
    }

    #[test]
    fn is_activated_at_10() {
        let mut log = make_log();
        add_today_qsos(&mut log, 10);
        assert!(log.is_activated());
    }

    #[test]
    fn is_activated_at_11() {
        let mut log = make_log();
        add_today_qsos(&mut log, 11);
        assert!(log.is_activated());
    }

    // --- Serde ---

    #[test]
    fn serde_round_trip() {
        let mut log = make_log();
        let qso = Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            Utc::now(),
            String::new(),
            None,
        )
        .unwrap();
        log.add_qso(qso);

        let json = serde_json::to_string(&log).unwrap();
        let deserialized: Log = serde_json::from_str(&json).unwrap();
        assert_eq!(log, deserialized);
    }
}
