//! Log persistence (ADIF) and file export.
//!
//! Each log is stored as a single `.adif` file. The ADIF header encodes all
//! log metadata; subsequent records encode individual QSOs. Appending a QSO
//! is an O(1) file append — no read or rewrite required.

mod error;
mod export;
mod manager;

pub use error::StorageError;
pub use export::{default_export_path, export_adif};
pub use manager::LogManager;
