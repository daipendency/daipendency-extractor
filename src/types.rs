use tree_sitter::Tree;

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
pub struct Namespace {
    pub name: String,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub source_code: String,
    pub doc_comment: Option<String>,
}

impl Namespace {
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_symbol_found() {
        let symbol = Symbol {
            name: "test_symbol".to_string(),
            source_code: "fn test() {}".to_string(),
            doc_comment: None,
        };
        let namespace = Namespace {
            name: "test_namespace".to_string(),
            symbols: vec![symbol],
        };

        let found = namespace.get_symbol("test_symbol");
        assert!(found.is_some(), "Should find existing symbol");
        assert_eq!(
            found.unwrap().name,
            "test_symbol",
            "Found symbol should have correct name"
        );
    }

    #[test]
    fn test_get_symbol_not_found() {
        let namespace = Namespace {
            name: "test_namespace".to_string(),
            symbols: vec![],
        };

        assert!(
            namespace.get_symbol("nonexistent").is_none(),
            "Should not find nonexistent symbol"
        );
    }
}
