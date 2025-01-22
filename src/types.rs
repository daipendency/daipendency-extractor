#[derive(Debug)]
pub struct Namespace {
    pub name: String,
    pub symbols: Vec<Symbol>,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub source_code: String,
}

impl Namespace {
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }
}

#[cfg(test)]
mod tests {
    use assertables::{assert_none, assert_some};

    use super::*;

    #[test]
    fn get_symbol_found() {
        let symbol = Symbol {
            name: "test_symbol".to_string(),
            source_code: "fn test() {}".to_string(),
        };
        let namespace = Namespace {
            name: "test_namespace".to_string(),
            symbols: vec![symbol],
            doc_comment: None,
        };

        let found = namespace.get_symbol("test_symbol");

        assert_some!(found);
        assert_eq!(found.unwrap().name, "test_symbol");
    }

    #[test]
    fn get_symbol_not_found() {
        let namespace = Namespace {
            name: "test_namespace".to_string(),
            symbols: vec![],
            doc_comment: None,
        };

        let symbol = namespace.get_symbol("nonexistent");

        assert_none!(symbol);
    }
}
