use thiserror::Error;

#[derive(Debug)]
pub struct LibraryMetadata {
    pub name: String,
    pub version: Option<String>,
    pub documentation: String,
    pub entry_point: std::path::PathBuf,
}

#[derive(Error, Debug)]
pub enum LibraryMetadataError {
    #[error(transparent)]
    MissingManifest(#[from] std::io::Error),
    #[error("{0}")]
    MalformedManifest(String),
}
