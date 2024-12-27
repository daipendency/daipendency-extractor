use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq)]
pub struct ApiDefinitions {
    pub functions: Vec<String>,
    pub structs: Vec<String>,
    pub enums: Vec<String>,
    pub traits: Vec<String>,
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
