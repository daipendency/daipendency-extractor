use std::any::Any;
use std::fmt;

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
pub struct Module<T: fmt::Display> {
    pub name: String,
    pub public_members: Vec<T>,
}

pub trait ApiRepresentation {
    fn as_any(&self) -> &dyn Any;
    fn modules(&self) -> Vec<Module<Box<dyn fmt::Display>>>;
}
