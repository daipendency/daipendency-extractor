use std::any::Any;
use std::fmt::Debug;
use std::path::PathBuf;

pub trait ApiRepresentation: Debug + Any {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
}

/// Structure representing library information
#[derive(Debug)]
pub struct LibraryInfo {
    /// Name of the library
    pub name: String,
    /// Version of the library
    pub version: String,
    /// Documentation content
    pub documentation: String,
    /// API signatures and types
    pub api: String,
}
