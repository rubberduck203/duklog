//! ADIF document formatting using the [`difa`] crate.
//!
//! Pure formatting functions that convert [`Log`](crate::model::Log) and
//! [`Qso`](crate::model::Qso) types into ADIF v3.1.6 text. No I/O — the
//! storage layer handles writing to disk.

mod error;
/// High-level ADIF document formatting.
mod writer;

pub use error::AdifError;
pub use writer::{format_adif, format_header, format_qso};
