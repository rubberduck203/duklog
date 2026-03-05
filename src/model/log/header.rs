use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};

use crate::model::band::Band;
use crate::model::mode::Mode;
use crate::model::qso::Qso;

/// The key that determines whether two QSOs are considered duplicates:
/// same callsign (case-insensitive), band, and mode.
fn duplicate_key(qso: &Qso) -> (String, Band, Mode) {
    (qso.their_call.to_lowercase(), qso.band, qso.mode)
}

/// Fields shared by every log type.
#[derive(Debug, Clone, PartialEq)]
pub struct LogHeader {
    pub(crate) station_callsign: String,
    pub(crate) operator: Option<String>,
    pub(crate) grid_square: String,
    pub(crate) qsos: Vec<Qso>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) log_id: String,
}

impl LogHeader {
    /// Counts unique contacts on the given date (UTC).
    ///
    /// Uniqueness is determined by (callsign, band, mode) — duplicate contacts
    /// with the same station on the same band and mode do not count separately.
    pub(crate) fn qso_count_on_date(&self, date: NaiveDate) -> usize {
        self.qsos
            .iter()
            .filter(|q| q.timestamp.date_naive() == date)
            .map(duplicate_key)
            .collect::<HashSet<_>>()
            .len()
    }

    /// Counts QSOs logged today (UTC).
    pub(crate) fn qso_count_today(&self) -> usize {
        self.qso_count_on_date(Utc::now().date_naive())
    }

    /// Returns QSOs matching the given callsign, band, and mode, optionally
    /// scoped to a specific date.
    ///
    /// When `on` is `Some(date)`, only QSOs logged on that exact date are
    /// considered. When `on` is `None`, all QSOs in the log are searched.
    pub(crate) fn find_duplicates_on(&self, qso: &Qso, on: Option<NaiveDate>) -> Vec<&Qso> {
        let key = duplicate_key(qso);
        self.qsos
            .iter()
            .filter(|q| {
                on.is_none_or(|date| q.timestamp.date_naive() == date) && duplicate_key(q) == key
            })
            .collect()
    }

    /// Replaces the QSO at `index` with `qso`, returning the old QSO.
    ///
    /// Returns `None` if `index` is out of bounds.
    pub(crate) fn replace_qso(&mut self, index: usize, qso: Qso) -> Option<Qso> {
        self.qsos
            .get_mut(index)
            .map(|slot| std::mem::replace(slot, qso))
    }

    /// Adds a QSO to this log.
    pub(crate) fn add_qso(&mut self, qso: Qso) {
        self.qsos.push(qso);
    }

    /// Removes and returns the QSO at `index`.
    ///
    /// Returns `None` if `index` is out of bounds.
    pub(crate) fn remove_qso(&mut self, index: usize) -> Option<Qso> {
        (index < self.qsos.len()).then(|| self.qsos.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use quickcheck_macros::quickcheck;

    use super::*;
    use crate::model::mode::Mode;

    fn make_header_with_n_qsos(n: usize) -> LogHeader {
        let mut header = LogHeader {
            station_callsign: "W1AW".into(),
            operator: None,
            grid_square: "FN31".into(),
            qsos: vec![],
            created_at: Utc::now(),
            log_id: "test".into(),
        };
        for i in 0..n {
            let qso = Qso::new(
                format!("W{i}AW"),
                "59".into(),
                "59".into(),
                Band::M20,
                Mode::Ssb,
                Utc::now(),
                String::new(),
                None,
                None,
                None,
            )
            .unwrap();
            header.add_qso(qso);
        }
        header
    }

    #[quickcheck]
    fn remove_qso_valid_index_returns_correct_qso(n: u8) -> bool {
        let n = (n as usize).max(1);
        let mut header = make_header_with_n_qsos(n);
        // First QSO was inserted with call "W0AW"
        let removed = header.remove_qso(0);
        removed.is_some_and(|q| q.their_call == "W0AW")
    }

    #[quickcheck]
    fn remove_qso_out_of_bounds_returns_none(n: u8) -> bool {
        let n = n as usize;
        let mut header = make_header_with_n_qsos(n);
        header.remove_qso(n).is_none()
    }

    #[quickcheck]
    fn remove_qso_decrements_length(n: u8) -> bool {
        let n = (n as usize).max(1);
        let mut header = make_header_with_n_qsos(n);
        header.remove_qso(0);
        header.qsos.len() == n - 1
    }
}
