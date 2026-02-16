//! Actions returned by screen event handlers.

use crate::model::Log;

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
    /// Quit the application.
    Quit,
}
