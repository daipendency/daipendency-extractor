use std::fmt;
use tree_sitter::Tree;

#[derive(Debug, Clone, PartialEq)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub documentation: String,
}

#[derive(Debug)]
pub struct SourceFile {
    pub path: std::path::PathBuf,
    pub content: String,
    pub tree: Option<Tree>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module<T: fmt::Display> {
    pub name: String,
    pub public_members: Vec<T>,
}

pub trait ApiRepresentation {
    fn modules(&self) -> Vec<Module<Box<dyn fmt::Display>>>;
}
