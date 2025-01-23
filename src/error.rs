use thiserror::Error;

/// Error whilst extracting public API
#[derive(Error, Debug)]
pub enum ExtractionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Malformed(String),
}

/// Error whilst resolving a dependency path
#[derive(Error, Debug)]
pub enum DependencyResolutionError {
    #[error("Failed to retrieve dependency: {0}")]
    RetrievalFailure(String),
    #[error("'{0}' is not a dependency")]
    MissingDependency(String),
}
