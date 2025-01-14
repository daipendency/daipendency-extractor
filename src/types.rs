#[derive(Debug)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub documentation: String,
    pub entry_point: std::path::PathBuf,
}

#[derive(Debug)]
pub struct Namespace {
    pub name: String,
    pub symbols: Vec<Symbol>,
    pub missing_symbols: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub source_code: String,
}

impl Namespace {
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .iter()
            .find(|s| s.name == name)
            .or_else(|| self.missing_symbols.iter().find(|s| s.name == name))
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
        };
        let namespace = Namespace {
            name: "test_namespace".to_string(),
            symbols: vec![symbol],
            missing_symbols: vec![],
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
            missing_symbols: vec![],
        };

        assert!(
            namespace.get_symbol("nonexistent").is_none(),
            "Should not find nonexistent symbol"
        );
    }

    #[test]
    fn test_get_missing_symbol() {
        let symbol = Symbol {
            name: "missing_symbol".to_string(),
            source_code: "fn missing() {}".to_string(),
        };
        let namespace = Namespace {
            name: "test_namespace".to_string(),
            symbols: vec![],
            missing_symbols: vec![symbol],
        };

        let found = namespace.get_symbol("missing_symbol");
        assert!(found.is_some(), "Should find missing symbol");
        assert_eq!(
            found.unwrap().name,
            "missing_symbol",
            "Found symbol should have correct name"
        );
    }
}
