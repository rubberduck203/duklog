//! Log persistence (JSONL) and ADIF file export.
//!
//! Each log is stored as a single `.jsonl` file: line 1 is metadata,
//! lines 2+ are individual QSO records. This makes appending a QSO a
//! single-line file append with no read/rewrite.

mod error;
mod export;
mod manager;

pub use error::StorageError;
pub use export::{default_export_path, export_adif};
pub use manager::LogManager;
