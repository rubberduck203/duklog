//! Reusable TUI widgets.

pub mod form;
pub mod status_bar;

pub use form::{Form, FormField, draw_form};
pub use status_bar::{StatusBarContext, draw_status_bar};
