use tree_sitter::{Node, Tree};

#[derive(Debug)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub documentation: String,
}

#[derive(Debug)]
pub struct SourceFile {
    pub path: std::path::PathBuf,
    pub content: String,
    pub tree: Tree,
}

#[derive(Debug)]
pub struct Module<'a> {
    pub name: String,
    pub symbols: Vec<Symbol<'a>>,
}

#[derive(Debug)]
pub struct Symbol<'a> {
    pub name: String,
    pub node: Node<'a>,
    pub source_code: String,
}
