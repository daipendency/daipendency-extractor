use thiserror::Error;

/// Error whilst extracting public API
#[derive(Error, Debug)]
pub enum ExtractionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Malformed(String),
}
