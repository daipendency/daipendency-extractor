use thiserror::Error;

#[derive(Debug)]
pub struct LibraryMetadata {
    pub name: String,
    pub version: Option<String>,
    pub documentation: String,
    pub entry_point: std::path::PathBuf,
}

#[derive(Error, Debug)]
#[error("{0}")]
pub struct LibraryMetadataError(String);
