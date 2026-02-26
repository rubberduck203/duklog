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
}
