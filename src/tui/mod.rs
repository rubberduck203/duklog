//! TUI: App state, event loop, screens, widgets.

pub mod action;
pub mod app;
pub mod error;
pub mod screens;
pub mod widgets;

#[cfg(test)]
pub mod test_utils;

pub use app::App;
pub use error::AppError;
