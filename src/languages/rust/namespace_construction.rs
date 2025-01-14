use super::symbol_resolution::ResolvedSymbol;
use crate::types::Namespace;
use std::collections::HashMap;

/// Construct the final namespace hierarchy using the resolved symbols.
pub fn construct_namespaces(
    resolved_symbols: &[ResolvedSymbol],
    crate_name: &str,
) -> Vec<Namespace> {
    let mut namespace_map: HashMap<String, Namespace> = HashMap::new();

    // Create root namespace
    namespace_map.insert(
        String::new(),
        Namespace {
            name: crate_name.to_string(),
            symbols: Vec::new(),
            missing_symbols: Vec::new(),
        },
    );

    // Group symbols by namespace
    for resolved_symbol in resolved_symbols {
        let namespace_path = resolved_symbol.modules.join("::");
        let namespace_name = if namespace_path.is_empty() {
            crate_name.to_string()
        } else {
            format!("{}::{}", crate_name, namespace_path)
        };

        let namespace = namespace_map
            .entry(namespace_path)
            .or_insert_with(|| Namespace {
                name: namespace_name,
                symbols: Vec::new(),
                missing_symbols: Vec::new(),
            });

        namespace.symbols.push(resolved_symbol.symbol.clone());
    }

    namespace_map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Symbol;

    const STUB_CRATE_NAME: &str = "test_crate";

    #[test]
    fn test_construct_namespaces_root_only() {
        let resolved_symbols = vec![ResolvedSymbol {
            symbol: Symbol {
                name: "test".to_string(),
                source_code: "pub fn test() {}".to_string(),
            },
            modules: Vec::new(),
        }];

        let namespaces = construct_namespaces(&resolved_symbols, STUB_CRATE_NAME);
        assert_eq!(namespaces.len(), 1);
        assert_eq!(namespaces[0].name, STUB_CRATE_NAME);
        assert_eq!(namespaces[0].symbols.len(), 1);
        assert_eq!(namespaces[0].symbols[0].name, "test");
    }

    #[test]
    fn test_construct_namespaces_with_modules() {
        let resolved_symbols = vec![
            ResolvedSymbol {
                symbol: Symbol {
                    name: "root_fn".to_string(),
                    source_code: "pub fn root_fn() {}".to_string(),
                },
                modules: Vec::new(),
            },
            ResolvedSymbol {
                symbol: Symbol {
                    name: "module_fn".to_string(),
                    source_code: "pub fn module_fn() {}".to_string(),
                },
                modules: vec!["module".to_string()],
            },
        ];

        let namespaces = construct_namespaces(&resolved_symbols, STUB_CRATE_NAME);
        assert_eq!(namespaces.len(), 2);

        let root = namespaces
            .iter()
            .find(|n| n.name == STUB_CRATE_NAME)
            .unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert_eq!(root.symbols[0].name, "root_fn");

        let module = namespaces
            .iter()
            .find(|n| n.name == format!("{}::module", STUB_CRATE_NAME))
            .unwrap();
        assert_eq!(module.symbols.len(), 1);
        assert_eq!(module.symbols[0].name, "module_fn");
    }

    #[test]
    fn test_construct_namespaces_nested_modules() {
        let resolved_symbols = vec![
            ResolvedSymbol {
                symbol: Symbol {
                    name: "root_fn".to_string(),
                    source_code: "pub fn root_fn() {}".to_string(),
                },
                modules: Vec::new(),
            },
            ResolvedSymbol {
                symbol: Symbol {
                    name: "nested_fn".to_string(),
                    source_code: "pub fn nested_fn() {}".to_string(),
                },
                modules: vec!["outer".to_string(), "inner".to_string()],
            },
        ];

        let namespaces = construct_namespaces(&resolved_symbols, STUB_CRATE_NAME);
        assert_eq!(namespaces.len(), 2);

        let root = namespaces
            .iter()
            .find(|n| n.name == STUB_CRATE_NAME)
            .unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert_eq!(root.symbols[0].name, "root_fn");

        let nested = namespaces
            .iter()
            .find(|n| n.name == format!("{}::outer::inner", STUB_CRATE_NAME))
            .unwrap();
        assert_eq!(nested.symbols.len(), 1);
        assert_eq!(nested.symbols[0].name, "nested_fn");
    }

    #[test]
    fn test_construct_namespaces_multiple_symbols_per_namespace() {
        let resolved_symbols = vec![
            ResolvedSymbol {
                symbol: Symbol {
                    name: "root_fn1".to_string(),
                    source_code: "pub fn root_fn1() {}".to_string(),
                },
                modules: Vec::new(),
            },
            ResolvedSymbol {
                symbol: Symbol {
                    name: "root_fn2".to_string(),
                    source_code: "pub fn root_fn2() {}".to_string(),
                },
                modules: Vec::new(),
            },
            ResolvedSymbol {
                symbol: Symbol {
                    name: "module_fn1".to_string(),
                    source_code: "pub fn module_fn1() {}".to_string(),
                },
                modules: vec!["module".to_string()],
            },
            ResolvedSymbol {
                symbol: Symbol {
                    name: "module_fn2".to_string(),
                    source_code: "pub fn module_fn2() {}".to_string(),
                },
                modules: vec!["module".to_string()],
            },
        ];

        let namespaces = construct_namespaces(&resolved_symbols, STUB_CRATE_NAME);
        assert_eq!(namespaces.len(), 2);

        let root = namespaces
            .iter()
            .find(|n| n.name == STUB_CRATE_NAME)
            .unwrap();
        assert_eq!(root.symbols.len(), 2);
        assert!(root.symbols.iter().any(|s| s.name == "root_fn1"));
        assert!(root.symbols.iter().any(|s| s.name == "root_fn2"));

        let module = namespaces
            .iter()
            .find(|n| n.name == format!("{}::module", STUB_CRATE_NAME))
            .unwrap();
        assert_eq!(module.symbols.len(), 2);
        assert!(module.symbols.iter().any(|s| s.name == "module_fn1"));
        assert!(module.symbols.iter().any(|s| s.name == "module_fn2"));
    }

    #[test]
    fn test_construct_namespaces_reexported_symbols() {
        let resolved_symbols = vec![
            ResolvedSymbol {
                symbol: Symbol {
                    name: "Format".to_string(),
                    source_code: "pub enum Format { Plain, Rich }".to_string(),
                },
                modules: Vec::new(),
            },
            ResolvedSymbol {
                symbol: Symbol {
                    name: "Format".to_string(),
                    source_code: "pub enum Format { Plain, Rich }".to_string(),
                },
                modules: vec!["text".to_string()],
            },
            ResolvedSymbol {
                symbol: Symbol {
                    name: "Format".to_string(),
                    source_code: "pub enum Format { Plain, Rich }".to_string(),
                },
                modules: vec!["text".to_string(), "formatter".to_string()],
            },
        ];

        let namespaces = construct_namespaces(&resolved_symbols, STUB_CRATE_NAME);
        assert_eq!(namespaces.len(), 3);

        let root = namespaces
            .iter()
            .find(|n| n.name == STUB_CRATE_NAME)
            .unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert!(root.symbols.iter().any(|s| s.name == "Format"));

        let text = namespaces
            .iter()
            .find(|n| n.name == format!("{}::text", STUB_CRATE_NAME))
            .unwrap();
        assert_eq!(text.symbols.len(), 1);
        assert!(text.symbols.iter().any(|s| s.name == "Format"));

        let formatter = namespaces
            .iter()
            .find(|n| n.name == format!("{}::text::formatter", STUB_CRATE_NAME))
            .unwrap();
        assert_eq!(formatter.symbols.len(), 1);
        assert!(formatter.symbols.iter().any(|s| s.name == "Format"));
    }

    #[test]
    fn test_construct_namespaces_empty_input() {
        let resolved_symbols = Vec::new();
        let namespaces = construct_namespaces(&resolved_symbols, STUB_CRATE_NAME);
        assert_eq!(namespaces.len(), 1);
        assert_eq!(namespaces[0].name, STUB_CRATE_NAME);
        assert_eq!(namespaces[0].symbols.len(), 0);
    }

    #[test]
    fn test_construct_namespaces_preserve_source_code() {
        let source_code = "pub struct Config {\n    pub name: String,\n    pub value: i32,\n}";
        let resolved_symbols = vec![ResolvedSymbol {
            symbol: Symbol {
                name: "Config".to_string(),
                source_code: source_code.to_string(),
            },
            modules: Vec::new(),
        }];

        let namespaces = construct_namespaces(&resolved_symbols, STUB_CRATE_NAME);
        assert_eq!(namespaces.len(), 1);
        assert_eq!(namespaces[0].symbols.len(), 1);
        assert_eq!(namespaces[0].symbols[0].source_code, source_code);
    }
}
