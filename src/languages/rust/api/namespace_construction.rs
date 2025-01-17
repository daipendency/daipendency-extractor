use super::symbol_resolution::SymbolResolution;
use crate::types::Namespace;
use std::collections::HashMap;

/// Construct the final namespace hierarchy using the resolved symbols.
pub fn construct_namespaces(
    symbol_resolution: SymbolResolution,
    crate_name: &str,
) -> Vec<Namespace> {
    let mut namespace_map: HashMap<String, Namespace> = HashMap::new();

    // Create root namespace
    namespace_map.insert(
        crate_name.to_string(),
        Namespace {
            name: crate_name.to_string(),
            symbols: Vec::new(),
            missing_symbols: Vec::new(),
            doc_comment: symbol_resolution.doc_comments.get("").cloned(),
        },
    );

    // Group symbols by namespace
    symbol_resolution
        .symbols
        .iter()
        .for_each(|resolved_symbol| {
            resolved_symbol.modules.iter().for_each(|module_path| {
                let namespace_name = if module_path.is_empty() {
                    crate_name.to_string()
                } else {
                    format!("{}::{}", crate_name, module_path)
                };

                namespace_map
                    .entry(namespace_name.clone())
                    .or_insert_with(|| Namespace {
                        name: namespace_name,
                        symbols: Vec::new(),
                        missing_symbols: Vec::new(),
                        doc_comment: symbol_resolution.doc_comments.get(module_path).cloned(),
                    })
                    .symbols
                    .push(resolved_symbol.symbol.clone());
            });
        });

    namespace_map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::api::symbol_resolution::ResolvedSymbol;
    use crate::types::Symbol;

    const STUB_CRATE_NAME: &str = "test_crate";
    const STUB_SYMBOL_NAME: &str = "test";

    fn stub_symbol(name: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            source_code: format!("pub fn {}() {{}}", name).to_string(),
        }
    }

    fn get_namespace<'a>(name: &str, namespaces: &'a [Namespace]) -> Option<&'a Namespace> {
        namespaces.iter().find(|n| n.name == name)
    }

    #[test]
    fn root_namespace() {
        let symbol = stub_symbol(STUB_SYMBOL_NAME);
        let resolved_symbols = vec![ResolvedSymbol {
            symbol: symbol.clone(),
            modules: vec![String::new()],
        }];

        let namespaces = construct_namespaces(
            SymbolResolution {
                symbols: resolved_symbols,
                doc_comments: HashMap::new(),
            },
            STUB_CRATE_NAME,
        );

        assert_eq!(namespaces.len(), 1);
        let root = get_namespace(STUB_CRATE_NAME, &namespaces).unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert_eq!(root.symbols[0].name, STUB_SYMBOL_NAME);
    }

    #[test]
    fn child_namespace_with_doc_comments() {
        let module_name = "text";
        let doc_comment = "Text processing module";
        let mut doc_comments = HashMap::new();
        doc_comments.insert(module_name.to_string(), doc_comment.to_string());
        let resolved_symbols = vec![ResolvedSymbol {
            symbol: stub_symbol(STUB_SYMBOL_NAME),
            modules: vec![module_name.to_string()],
        }];

        let namespaces = construct_namespaces(
            SymbolResolution {
                symbols: resolved_symbols,
                doc_comments,
            },
            STUB_CRATE_NAME,
        );

        let text_namespace = get_namespace(
            &format!("{}::{}", STUB_CRATE_NAME, module_name),
            &namespaces,
        )
        .unwrap();
        assert_eq!(text_namespace.doc_comment.as_deref(), Some(doc_comment));
    }

    #[test]
    fn with_modules() {
        let resolved_symbols = vec![
            ResolvedSymbol {
                symbol: stub_symbol(STUB_SYMBOL_NAME),
                modules: vec![String::new()],
            },
            ResolvedSymbol {
                symbol: stub_symbol(STUB_SYMBOL_NAME),
                modules: vec!["module".to_string()],
            },
        ];

        let namespaces = construct_namespaces(
            SymbolResolution {
                symbols: resolved_symbols,
                doc_comments: HashMap::new(),
            },
            STUB_CRATE_NAME,
        );
        assert_eq!(namespaces.len(), 2);

        let root = get_namespace(STUB_CRATE_NAME, &namespaces).unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert_eq!(root.symbols[0].name, STUB_SYMBOL_NAME);

        let module = get_namespace(&format!("{}::module", STUB_CRATE_NAME), &namespaces).unwrap();
        assert_eq!(module.symbols.len(), 1);
        assert_eq!(module.symbols[0].name, STUB_SYMBOL_NAME);
    }

    #[test]
    fn nested_modules() {
        let resolved_symbols = vec![
            ResolvedSymbol {
                symbol: stub_symbol(STUB_SYMBOL_NAME),
                modules: vec![String::new()],
            },
            ResolvedSymbol {
                symbol: stub_symbol(STUB_SYMBOL_NAME),
                modules: vec!["outer::inner".to_string()],
            },
        ];

        let namespaces = construct_namespaces(
            SymbolResolution {
                symbols: resolved_symbols,
                doc_comments: HashMap::new(),
            },
            STUB_CRATE_NAME,
        );
        assert_eq!(namespaces.len(), 2);

        let root = get_namespace(STUB_CRATE_NAME, &namespaces).unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert_eq!(root.symbols[0].name, STUB_SYMBOL_NAME);

        let nested =
            get_namespace(&format!("{}::outer::inner", STUB_CRATE_NAME), &namespaces).unwrap();
        assert_eq!(nested.symbols.len(), 1);
        assert_eq!(nested.symbols[0].name, STUB_SYMBOL_NAME);
    }

    #[test]
    fn same_symbol_across_hierarchy() {
        let symbol = stub_symbol(STUB_SYMBOL_NAME);
        let resolved_symbols = vec![ResolvedSymbol {
            symbol: symbol.clone(),
            modules: vec!["outer".to_string(), "outer::inner".to_string()],
        }];

        let namespaces = construct_namespaces(
            SymbolResolution {
                symbols: resolved_symbols,
                doc_comments: HashMap::new(),
            },
            STUB_CRATE_NAME,
        );

        assert_eq!(namespaces.len(), 3);
        let outer_namespace =
            get_namespace(&format!("{}::outer", STUB_CRATE_NAME), &namespaces).unwrap();
        let inner_namespace =
            get_namespace(&format!("{}::outer::inner", STUB_CRATE_NAME), &namespaces).unwrap();
        assert_eq!(outer_namespace.symbols[0], symbol);
        assert_eq!(inner_namespace.symbols[0], symbol);
    }

    #[test]
    fn multiple_symbols_per_namespace() {
        let module_name = String::new();
        let resolved_symbols = vec![
            ResolvedSymbol {
                symbol: stub_symbol("first_symbol"),
                modules: vec![module_name.clone()],
            },
            ResolvedSymbol {
                symbol: stub_symbol("second_symbol"),
                modules: vec![module_name.clone()],
            },
        ];

        let namespaces = construct_namespaces(
            SymbolResolution {
                symbols: resolved_symbols,
                doc_comments: HashMap::new(),
            },
            STUB_CRATE_NAME,
        );
        assert_eq!(namespaces.len(), 1);

        let root = get_namespace(STUB_CRATE_NAME, &namespaces).unwrap();
        assert_eq!(root.symbols.len(), 2);
        assert_eq!(root.symbols[0].name, "first_symbol");
        assert_eq!(root.symbols[1].name, "second_symbol");
    }

    #[test]
    fn empty_input() {
        let resolved_symbols = Vec::new();
        let namespaces = construct_namespaces(
            SymbolResolution {
                symbols: resolved_symbols,
                doc_comments: HashMap::new(),
            },
            STUB_CRATE_NAME,
        );
        assert_eq!(namespaces.len(), 1);
        let root = get_namespace(STUB_CRATE_NAME, &namespaces).unwrap();
        assert_eq!(root.symbols.len(), 0);
    }

    #[test]
    fn preserve_source_code() {
        let source_code = "pub struct Config {\n    pub name: String,\n    pub value: i32,\n}";
        let resolved_symbols = vec![ResolvedSymbol {
            symbol: Symbol {
                name: "Config".to_string(),
                source_code: source_code.to_string(),
            },
            modules: vec![String::new()],
        }];

        let namespaces = construct_namespaces(
            SymbolResolution {
                symbols: resolved_symbols,
                doc_comments: HashMap::new(),
            },
            STUB_CRATE_NAME,
        );
        assert_eq!(namespaces.len(), 1);
        let root = get_namespace(STUB_CRATE_NAME, &namespaces).unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert_eq!(root.symbols[0].source_code, source_code);
    }
}
