//! ADIF document formatting and reading using the [`difa`] crate.
//!
//! Formatting functions convert [`Log`](crate::model::Log) and
//! [`Qso`](crate::model::Qso) types into ADIF v3.1.6 text. No I/O — the
//! storage layer handles writing to disk. The reader reconstructs a `Log`
//! from an `.adif` file previously written by the formatter.

mod error;
mod reader;
// High-level ADIF document formatting.
mod writer;

pub use error::AdifError;
pub use reader::read_log;
pub use writer::{format_adif, format_header, format_qso};
