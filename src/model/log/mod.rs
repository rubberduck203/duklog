use chrono::{NaiveDate, Utc};

use super::qso::Qso;

mod field_day;
mod general;
mod header;
mod pota;
mod wfd;

pub use field_day::{FdClass, FdPowerCategory, FieldDayLog, parse_fd_class};
pub use general::GeneralLog;
pub use header::LogHeader;
pub use pota::PotaLog;
pub use wfd::{WfdClass, WfdLog, parse_wfd_class};

/// Minimum unique QSOs required for a valid POTA activation (per UTC day).
const POTA_ACTIVATION_THRESHOLD: usize = 10;

/// Any log session. The variant determines type-specific behavior and ADIF output.
#[derive(Debug, Clone, PartialEq)]
pub enum Log {
    /// General-purpose log — no type-specific fields.
    General(GeneralLog),
    /// POTA (Parks on the Air) activation log.
    Pota(PotaLog),
    /// ARRL Field Day contest log.
    FieldDay(FieldDayLog),
    /// Winter Field Day contest log.
    WinterFieldDay(WfdLog),
}

impl Log {
    /// Returns a reference to the shared log header.
    pub fn header(&self) -> &LogHeader {
        match self {
            Self::General(l) => &l.header,
            Self::Pota(l) => &l.header,
            Self::FieldDay(l) => &l.header,
            Self::WinterFieldDay(l) => &l.header,
        }
    }

    /// Returns a mutable reference to the shared log header.
    pub fn header_mut(&mut self) -> &mut LogHeader {
        match self {
            Self::General(l) => &mut l.header,
            Self::Pota(l) => &mut l.header,
            Self::FieldDay(l) => &mut l.header,
            Self::WinterFieldDay(l) => &mut l.header,
        }
    }

    /// Returns the POTA park reference for this log, or `None` for non-POTA logs.
    pub fn park_ref(&self) -> Option<&str> {
        match self {
            Self::Pota(p) => p.park_ref.as_deref(),
            _ => None,
        }
    }

    /// Adds a QSO to this log.
    pub fn add_qso(&mut self, qso: Qso) {
        self.header_mut().add_qso(qso);
    }

    #[cfg(test)]
    pub(crate) fn qso_count_on_date(&self, date: NaiveDate) -> usize {
        self.header().qso_count_on_date(date)
    }

    /// Counts QSOs logged today (UTC).
    pub fn qso_count_today(&self) -> usize {
        self.header().qso_count_today()
    }

    /// Returns the number of additional QSOs needed for a valid POTA activation today.
    ///
    /// For non-POTA logs, always returns `0`.
    pub fn needs_for_activation(&self) -> usize {
        match self {
            Self::Pota(_) => POTA_ACTIVATION_THRESHOLD.saturating_sub(self.qso_count_today()),
            _ => 0,
        }
    }

    /// Returns `true` if this log has met its activation threshold.
    ///
    /// For POTA logs: ≥10 unique QSOs today (UTC). For all other types: always `false`.
    pub fn is_activated(&self) -> bool {
        match self {
            Self::Pota(_) => self.qso_count_today() >= POTA_ACTIVATION_THRESHOLD,
            _ => false,
        }
    }

    /// Returns QSOs matching the given callsign, band, and mode within the
    /// applicable scope for this log type.
    ///
    /// - POTA and General logs: scoped to today (UTC) — a non-empty result
    ///   indicates a potential duplicate within the current UTC day.
    /// - Field Day and Winter Field Day logs: scoped across the entire log —
    ///   these events span multiple UTC calendar days.
    ///
    /// Callsign comparison is case-insensitive.
    pub fn find_duplicates(&self, qso: &Qso) -> Vec<&Qso> {
        match self {
            Self::FieldDay(_) | Self::WinterFieldDay(_) => self.find_duplicates_on(qso, None),
            _ => self.find_duplicates_on(qso, Some(Utc::now().date_naive())),
        }
    }

    /// Returns QSOs matching the given callsign, band, and mode, optionally
    /// scoped to a specific date.
    ///
    /// When `on` is `Some(date)`, only QSOs logged on that exact date are
    /// considered. When `on` is `None`, all QSOs in the log are searched.
    pub(crate) fn find_duplicates_on(&self, qso: &Qso, on: Option<NaiveDate>) -> Vec<&Qso> {
        self.header().find_duplicates_on(qso, on)
    }

    /// Replaces the QSO at `index` with `qso`, returning the old QSO.
    ///
    /// Returns `None` if `index` is out of bounds.
    pub fn replace_qso(&mut self, index: usize, qso: Qso) -> Option<Qso> {
        self.header_mut().replace_qso(index, qso)
    }

    /// Returns a short display label for this log.
    ///
    /// - POTA: park reference if present, otherwise station callsign.
    /// - Field Day / Winter Field Day: sent exchange string (e.g. `"1B EPA"`).
    /// - General: station callsign.
    pub fn display_label(&self) -> String {
        match self {
            Self::Pota(p) => p
                .park_ref
                .as_deref()
                .unwrap_or(&p.header.station_callsign)
                .to_string(),
            Self::General(l) => l.header.station_callsign.clone(),
            Self::FieldDay(l) => l.sent_exchange(),
            Self::WinterFieldDay(l) => l.sent_exchange(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};
    use quickcheck_macros::quickcheck;

    use super::*;
    use crate::model::band::Band;
    use crate::model::mode::Mode;

    fn make_log() -> Log {
        Log::Pota(
            PotaLog::new(
                "W1AW".to_string(),
                Some("W1AW".to_string()),
                Some("K-0001".to_string()),
                "FN31".to_string(),
            )
            .unwrap(),
        )
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
            None,
            None,
        )
        .unwrap()
    }

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
            None,
            None,
        )
        .unwrap()
    }

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
                None,
                None,
            )
            .unwrap();
            log.add_qso(qso);
        }
    }

    fn make_log_with_n_qsos_on_date(n: usize, date: NaiveDate) -> Log {
        let mut log = make_log();
        for i in 0..n {
            log.add_qso(make_qso_on_date_with_call(&format!("W{i}AW"), date));
        }
        log
    }

    // --- QSO operations ---

    #[test]
    fn add_qso_increments_count() {
        let mut log = make_log();
        assert_eq!(log.header().qsos.len(), 0);
        let qso = Qso::new(
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
        log.add_qso(qso);
        assert_eq!(log.header().qsos.len(), 1);
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
        let needs = POTA_ACTIVATION_THRESHOLD.saturating_sub(count);
        let activated = count >= POTA_ACTIVATION_THRESHOLD;

        // Verify all three are consistent
        count == n
            && needs == POTA_ACTIVATION_THRESHOLD.saturating_sub(n)
            && activated == (n >= POTA_ACTIVATION_THRESHOLD)
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
            None,
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
            None,
            None,
        )
        .unwrap();

        log.add_qso(qso1);
        log.add_qso(qso2);

        assert_eq!(log.qso_count_on_date(date1), 1);
        assert_eq!(log.qso_count_on_date(date2), 1);
    }

    // --- qso_count_today / needs_for_activation / is_activated ---

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
        log.needs_for_activation() == POTA_ACTIVATION_THRESHOLD.saturating_sub(n as usize)
    }

    #[quickcheck]
    fn is_activated_property(n: u8) -> bool {
        let mut log = make_log();
        add_today_qsos(&mut log, n as usize);
        log.is_activated() == (n as usize >= POTA_ACTIVATION_THRESHOLD)
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
            None,
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
            None,
            None,
        )
        .unwrap();
        log.add_qso(qso1);
        log.add_qso(qso2);
        assert_eq!(log.qso_count_on_date(date), 2);
    }

    // --- find_duplicates ---

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
            None,
            None,
        )
        .unwrap();
        log.add_qso(old_qso);
        let candidate = make_candidate("KD9XYZ", Band::M20, Mode::Ssb);
        assert_eq!(log.find_duplicates(&candidate).len(), 0);
    }

    // --- find_duplicates_on ---

    #[test]
    fn find_duplicates_on_none_searches_all_dates() {
        let mut log = make_log();
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
        log.add_qso(make_candidate("KD9XYZ", Band::M20, Mode::Ssb));

        let candidate = make_candidate("KD9XYZ", Band::M20, Mode::Ssb);
        // None means search all — should find both
        assert_eq!(log.find_duplicates_on(&candidate, None).len(), 2);
    }

    #[test]
    fn find_duplicates_on_some_filters_to_date() {
        let mut log = make_log();
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
        log.add_qso(make_candidate("KD9XYZ", Band::M20, Mode::Ssb));

        let candidate = make_candidate("KD9XYZ", Band::M20, Mode::Ssb);
        let today = Utc::now().date_naive();
        // Some(today) — should find only today's QSO
        assert_eq!(log.find_duplicates_on(&candidate, Some(today)).len(), 1);
        // Some(yesterday) — should find only yesterday's QSO
        assert_eq!(log.find_duplicates_on(&candidate, Some(yesterday)).len(), 1);
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
        assert_eq!(log.header().qsos[0], qso2);
    }

    #[test]
    fn replace_qso_out_of_bounds_returns_none() {
        let mut log = make_log();
        let qso = make_qso_on_date(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        assert_eq!(log.replace_qso(0, qso), None);
    }
}
