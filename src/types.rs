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
pub struct Namespace<'a> {
    pub name: String,
    pub symbols: Vec<Symbol<'a>>,
}

#[derive(Debug)]
pub struct Symbol<'a> {
    pub name: String,
    pub node: Node<'a>,
    pub source_code: String,
    pub doc_comment: Option<String>,
}

impl<'a> Namespace<'a> {
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol<'a>> {
        self.symbols.iter().find(|s| s.name == name)
    }
}
