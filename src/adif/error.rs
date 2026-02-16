use std::string::FromUtf8Error;

/// Errors that can occur during ADIF formatting.
#[derive(Debug, thiserror::Error)]
pub enum AdifError {
    /// The difa encoder failed to encode a tag.
    #[error("ADIF encoding error: {0}")]
    Encode(#[from] difa::Error),

    /// The encoded output contained invalid UTF-8.
    #[error("ADIF output contained invalid UTF-8: {0}")]
    Utf8(#[from] FromUtf8Error),
}
