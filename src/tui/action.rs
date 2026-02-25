//! Actions returned by screen event handlers.

use crossterm::event::KeyEvent;

use crate::model::{Log, Qso};

use super::app::Screen;

/// An action that a screen handler returns to the [`App`](super::App).
///
/// The `App` interprets these to update global state and navigate between
/// screens.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// No state change needed.
    None,
    /// Navigate to the given screen.
    Navigate(Screen),
    /// Select an existing log as the active session.
    SelectLog(Log),
    /// Create and persist a new log, then make it active.
    CreateLog(Log),
    /// Add a QSO to the active log.
    AddQso(Qso),
    /// Open the QSO at the given index for editing.
    EditQso(usize),
    /// Replace the QSO at the given index with an edited version.
    UpdateQso(usize, Qso),
    /// Export the active log to ADIF.
    ExportLog,
    /// Delete the log with the given ID from storage.
    DeleteLog(String),
    /// Quit the application.
    Quit,
}

/// Common behavior for all screen state types.
pub trait ScreenState {
    /// Process a key event and return an [`Action`] for the `App` to apply.
    fn handle_key(&mut self, key: KeyEvent) -> Action;
}
