use std::path::PathBuf;

use chrono::NaiveDate;

/// Errors that can occur during storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// An I/O error occurred while reading or writing a file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A JSON serialization or deserialization error occurred.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// An ADIF formatting error occurred during export.
    #[error("ADIF error: {0}")]
    Adif(#[from] crate::adif::AdifError),

    /// The platform does not provide a data directory.
    #[error("could not determine XDG data directory")]
    NoDataDir,

    /// The platform does not provide a home directory.
    #[error("could not determine home directory")]
    NoHomeDir,

    /// A JSONL log file exists but contains no metadata line.
    #[error("log file is empty: {0}")]
    EmptyLogFile(PathBuf),

    /// A log already exists for the same station, operator, park reference, and grid square on the same UTC day.
    ///
    /// Produced by [`crate::storage::LogManager::create_log`] when a duplicate is detected.
    #[error("a log already exists for {callsign} on {date} UTC")]
    DuplicateLog {
        /// The station callsign of the conflicting log.
        callsign: String,
        /// The UTC date of the conflict.
        date: NaiveDate,
    },

    /// A log file contains valid JSON but is missing required fields for its declared log type.
    #[error("corrupt log metadata: {0}")]
    CorruptMetadata(String),
}
