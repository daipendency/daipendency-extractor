use std::any::Any;

#[derive(Debug, Clone, PartialEq)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: std::path::PathBuf,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub public_members: Vec<String>,
}

pub trait ApiRepresentation {
    fn as_any(&self) -> &dyn Any;
    fn modules(&self) -> Vec<Module>;
}
