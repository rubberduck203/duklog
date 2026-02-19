use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::band::Band;
use super::mode::Mode;
use super::qso::Qso;
use super::validation::{
    ValidationError, validate_callsign, validate_grid_square, validate_park_ref,
};

/// The key that determines whether two QSOs are considered duplicates:
/// same callsign (case-insensitive), band, and mode.
fn duplicate_key(qso: &Qso) -> (String, Band, Mode) {
    (qso.their_call.to_lowercase(), qso.band, qso.mode)
}

/// An activation session containing station info and a collection of QSOs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Log {
    pub station_callsign: String,
    pub operator: Option<String>,
    pub park_ref: Option<String>,
    pub grid_square: String,
    pub qsos: Vec<Qso>,
    pub created_at: DateTime<Utc>,
    pub log_id: String,
}

impl Log {
    /// Creates a new log, validating all fields.
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

    /// Counts unique contacts on the given date (UTC).
    ///
    /// Uniqueness is determined by (callsign, band, mode) — duplicate contacts
    /// with the same station on the same band and mode do not count toward the
    /// activation threshold.
    pub(crate) fn qso_count_on_date(&self, date: NaiveDate) -> usize {
        self.qsos
            .iter()
            .filter(|q| q.timestamp.date_naive() == date)
            .map(duplicate_key)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    /// Counts QSOs logged today (UTC).
    pub fn qso_count_today(&self) -> usize {
        self.qso_count_on_date(Utc::now().date_naive())
    }

    /// Returns the number of additional QSOs needed for a valid POTA activation today.
    pub fn needs_for_activation(&self) -> usize {
        10_usize.saturating_sub(self.qso_count_today())
    }

    /// Returns QSOs from today (UTC) that match the given callsign, band, and mode.
    ///
    /// Callsign comparison is case-insensitive. A non-empty result indicates a
    /// potential duplicate contact within the current UTC day.
    pub fn find_duplicates(&self, qso: &Qso) -> Vec<&Qso> {
        let today = Utc::now().date_naive();
        let key = duplicate_key(qso);
        self.qsos
            .iter()
            .filter(|q| q.timestamp.date_naive() == today && duplicate_key(q) == key)
            .collect()
    }

    /// Replaces the QSO at `index` with `qso`, returning the old QSO.
    ///
    /// Returns `None` if `index` is out of bounds.
    pub fn replace_qso(&mut self, index: usize, qso: Qso) -> Option<Qso> {
        self.qsos
            .get_mut(index)
            .map(|slot| std::mem::replace(slot, qso))
    }

    /// Returns `true` if this log has at least 10 QSOs today (valid POTA activation).
    pub fn is_activated(&self) -> bool {
        self.qso_count_today() >= 10
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};
    use quickcheck_macros::quickcheck;

    use super::*;
    use crate::model::band::Band;
    use crate::model::mode::Mode;

    fn make_log() -> Log {
        Log::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap()
    }

    fn make_qso_on_date(date: NaiveDate) -> Qso {
        make_qso_on_date_with_call("KD9XYZ", date)
    }

    fn make_qso_on_date_with_call(call: &str, date: NaiveDate) -> Qso {
        let timestamp = date
            .and_hms_opt(12, 0, 0)
            .map(|dt| Utc.from_utc_datetime(&dt))
            .unwrap();
        Qso::new(
            call.to_string(),
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
    fn valid_log_creation_with_park() {
        let log = make_log();
        assert_eq!(log.station_callsign, "W1AW");
        assert_eq!(log.operator, Some("W1AW".to_string()));
        assert_eq!(log.park_ref, Some("K-0001".to_string()));
        assert_eq!(log.grid_square, "FN31");
        assert_eq!(log.qsos.len(), 0);
        assert!(log.log_id.starts_with("K-0001-"));
    }

    #[test]
    fn valid_log_creation_without_park() {
        let log = Log::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            None,
            "FN31".to_string(),
        )
        .unwrap();
        assert_eq!(log.operator, Some("W1AW".to_string()));
        assert_eq!(log.park_ref, None);
        assert!(log.log_id.starts_with("W1AW-"));
    }

    #[test]
    fn invalid_station_callsign() {
        let result = Log::new(
            String::new(),
            Some("W1AW".to_string()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::EmptyCallsign));
    }

    #[test]
    fn invalid_operator() {
        let result = Log::new(
            "W1AW".to_string(),
            Some(String::new()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        );
        assert_eq!(result, Err(ValidationError::EmptyCallsign));
    }

    #[test]
    fn none_operator_succeeds() {
        let log = Log::new(
            "W1AW".to_string(),
            None,
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        assert_eq!(log.operator, None);
    }

    #[test]
    fn invalid_park_ref() {
        let result = Log::new(
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
        let result = Log::new(
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

    // --- Activation tests (qso_count_on_date) ---

    #[test]
    fn qso_count_on_date_filters_correctly() {
        let mut log = make_log();
        let date1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

        // Two distinct calls on date1, one on date2
        log.add_qso(make_qso_on_date_with_call("KD9XYZ", date1));
        log.add_qso(make_qso_on_date_with_call("W3ABC", date1));
        log.add_qso(make_qso_on_date_with_call("N0CALL", date2));

        assert_eq!(log.qso_count_on_date(date1), 2);
        assert_eq!(log.qso_count_on_date(date2), 1);
    }

    fn make_log_with_n_qsos_on_date(n: usize, date: NaiveDate) -> Log {
        let mut log = make_log();
        for i in 0..n {
            log.add_qso(make_qso_on_date_with_call(&format!("W{i}AW"), date));
        }
        log
    }

    #[quickcheck]
    fn qso_count_on_date_equals_added_count(n: u8) -> bool {
        let n = n as usize;
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let log = make_log_with_n_qsos_on_date(n, date);
        log.qso_count_on_date(date) == n
    }

    #[quickcheck]
    fn activation_threshold_property(n: u8) -> bool {
        let n = n as usize;
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let log = make_log_with_n_qsos_on_date(n, date);
        let count = log.qso_count_on_date(date);
        let needs = 10_usize.saturating_sub(count);
        let activated = count >= 10;

        // Verify all three are consistent
        count == n && needs == 10_usize.saturating_sub(n) && activated == (n >= 10)
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
        for i in 0..n {
            let qso = Qso::new(
                format!("W{i}AW"),
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

    #[quickcheck]
    fn qso_count_today_matches_added(n: u8) -> bool {
        let mut log = make_log();
        add_today_qsos(&mut log, n as usize);
        log.qso_count_today() == n as usize
    }

    #[quickcheck]
    fn needs_for_activation_property(n: u8) -> bool {
        let mut log = make_log();
        add_today_qsos(&mut log, n as usize);
        log.needs_for_activation() == 10_usize.saturating_sub(n as usize)
    }

    #[quickcheck]
    fn is_activated_property(n: u8) -> bool {
        let mut log = make_log();
        add_today_qsos(&mut log, n as usize);
        log.is_activated() == (n as usize >= 10)
    }

    #[test]
    fn duplicate_qso_not_counted_for_activation() {
        let mut log = make_log();
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        // Same call+band+mode twice on the same date
        log.add_qso(make_qso_on_date_with_call("KD9XYZ", date));
        log.add_qso(make_qso_on_date_with_call("KD9XYZ", date));
        assert_eq!(log.qso_count_on_date(date), 1);
    }

    #[test]
    fn different_band_same_call_counts_separately() {
        let mut log = make_log();
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let ts = Utc.from_utc_datetime(&date.and_hms_opt(12, 0, 0).unwrap());
        let qso1 = Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M20,
            Mode::Ssb,
            ts,
            String::new(),
            None,
        )
        .unwrap();
        let qso2 = Qso::new(
            "KD9XYZ".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M40,
            Mode::Ssb,
            ts,
            String::new(),
            None,
        )
        .unwrap();
        log.add_qso(qso1);
        log.add_qso(qso2);
        assert_eq!(log.qso_count_on_date(date), 2);
    }

    // --- find_duplicates ---

    fn make_candidate(call: &str, band: Band, mode: Mode) -> Qso {
        Qso::new(
            call.to_string(),
            mode.default_rst().to_string(),
            mode.default_rst().to_string(),
            band,
            mode,
            Utc::now(),
            String::new(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn find_duplicates_empty_log_returns_empty() {
        let log = make_log();
        let qso = make_candidate("KD9XYZ", Band::M20, Mode::Ssb);
        assert_eq!(log.find_duplicates(&qso).len(), 0);
    }

    #[test]
    fn find_duplicates_exact_match_detected() {
        let mut log = make_log();
        let existing = make_candidate("KD9XYZ", Band::M20, Mode::Ssb);
        log.add_qso(existing.clone());
        let candidate = make_candidate("KD9XYZ", Band::M20, Mode::Ssb);
        let dups = log.find_duplicates(&candidate);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].their_call, "KD9XYZ");
        assert_eq!(dups[0].band, Band::M20);
        assert_eq!(dups[0].mode, Mode::Ssb);
    }

    #[test]
    fn find_duplicates_different_band_not_flagged() {
        let mut log = make_log();
        log.add_qso(make_candidate("KD9XYZ", Band::M20, Mode::Ssb));
        let candidate = make_candidate("KD9XYZ", Band::M40, Mode::Ssb);
        assert_eq!(log.find_duplicates(&candidate).len(), 0);
    }

    #[test]
    fn find_duplicates_different_mode_not_flagged() {
        let mut log = make_log();
        log.add_qso(make_candidate("KD9XYZ", Band::M20, Mode::Ssb));
        let candidate = make_candidate("KD9XYZ", Band::M20, Mode::Cw);
        assert_eq!(log.find_duplicates(&candidate).len(), 0);
    }

    #[test]
    fn find_duplicates_case_insensitive_callsign() {
        let mut log = make_log();
        log.add_qso(make_candidate("KD9XYZ", Band::M20, Mode::Ssb));
        // Lowercase callsign passes validation (alphanumeric chars allowed)
        let candidate = make_candidate("kd9xyz", Band::M20, Mode::Ssb);
        let dups = log.find_duplicates(&candidate);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].their_call, "KD9XYZ");
    }

    #[test]
    fn find_duplicates_returns_all_matching_qsos() {
        let mut log = make_log();
        log.add_qso(make_candidate("KD9XYZ", Band::M20, Mode::Ssb));
        log.add_qso(make_candidate("KD9XYZ", Band::M40, Mode::Ssb)); // different band
        log.add_qso(make_candidate("KD9XYZ", Band::M20, Mode::Ssb)); // second match
        let candidate = make_candidate("KD9XYZ", Band::M20, Mode::Ssb);
        let dups = log.find_duplicates(&candidate);
        assert_eq!(dups.len(), 2);
        assert!(
            dups.iter()
                .all(|q| q.band == Band::M20 && q.mode == Mode::Ssb)
        );
    }

    #[test]
    fn find_duplicates_ignores_previous_day_qsos() {
        let mut log = make_log();
        // Add a QSO with yesterday's timestamp
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
        )
        .unwrap();
        log.add_qso(old_qso);
        let candidate = make_candidate("KD9XYZ", Band::M20, Mode::Ssb);
        assert_eq!(log.find_duplicates(&candidate).len(), 0);
    }

    // --- replace_qso ---

    #[test]
    fn replace_qso_at_valid_index_returns_old() {
        let mut log = make_log();
        let qso1 = make_qso_on_date(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        log.add_qso(qso1.clone());

        let qso2 = make_qso_on_date(NaiveDate::from_ymd_opt(2026, 2, 20).unwrap());
        let old = log.replace_qso(0, qso2.clone());
        assert_eq!(old, Some(qso1));
        assert_eq!(log.qsos[0], qso2);
    }

    #[test]
    fn replace_qso_out_of_bounds_returns_none() {
        let mut log = make_log();
        let qso = make_qso_on_date(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        assert_eq!(log.replace_qso(0, qso), None);
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

    #[test]
    fn serde_round_trip_without_park() {
        let log = Log::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            None,
            "FN31".to_string(),
        )
        .unwrap();
        let json = serde_json::to_string(&log).unwrap();
        let deserialized: Log = serde_json::from_str(&json).unwrap();
        assert_eq!(log, deserialized);
    }

    #[test]
    fn serde_round_trip_none_operator() {
        let log = Log::new(
            "W1AW".to_string(),
            None,
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        let json = serde_json::to_string(&log).unwrap();
        let deserialized: Log = serde_json::from_str(&json).unwrap();
        assert_eq!(log, deserialized);
        assert_eq!(deserialized.operator, None);
    }
}
