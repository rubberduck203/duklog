use crate::storage::StorageError;

/// Errors that can occur in the TUI layer.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// An I/O error occurred (terminal, event reading, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A storage error occurred while persisting data.
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}
