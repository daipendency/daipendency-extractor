use crate::error::LaibraryError;
use crate::types::Symbol;
use std::collections::HashMap;

use super::symbol_collection::RawNamespace;

#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    pub symbol: Symbol,
    pub modules: Vec<String>,
}

#[derive(Debug)]
pub struct SymbolResolution {
    pub symbols: Vec<ResolvedSymbol>,
    pub doc_comments: HashMap<String, String>,
}

/// Resolve symbol references by matching them with their corresponding definitions.
pub fn resolve_symbols(raw_namespaces: &[RawNamespace]) -> Result<SymbolResolution, LaibraryError> {
    let mut symbol_map: HashMap<String, Symbol> = HashMap::new();
    let mut reference_map: HashMap<String, String> = HashMap::new();
    let mut module_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut doc_comments = HashMap::new();
    let mut public_namespaces = std::collections::HashSet::new();

    // First pass: collect all symbol definitions and references
    for namespace in raw_namespaces {
        // The root namespace and public modules are public
        if namespace.name.is_empty() || namespace.is_public {
            public_namespaces.insert(namespace.name.clone());
        }

        // Collect doc comments for public namespaces
        if let Some(doc_comment) = &namespace.doc_comment {
            if namespace.is_public {
                doc_comments.insert(namespace.name.clone(), doc_comment.clone());
            }
        }

        for symbol in &namespace.definitions {
            let qualified_name = if namespace.name.is_empty() {
                symbol.name.clone()
            } else {
                format!("{}::{}", namespace.name, symbol.name)
            };
            symbol_map.insert(qualified_name.clone(), symbol.clone());
            let modules = if namespace.name.is_empty() {
                vec![String::new()]
            } else {
                namespace.name.split("::").map(String::from).collect()
            };
            module_map.entry(qualified_name).or_insert(modules);
        }
        for (_name, source_path) in &namespace.references {
            let qualified_name = if namespace.name.is_empty() {
                _name.clone()
            } else {
                format!("{}::{}", namespace.name, _name)
            };
            reference_map.insert(qualified_name, source_path.clone());
        }
    }

    // Second pass: resolve references and collect symbols
    let mut resolved_symbols = Vec::new();
    for namespace in raw_namespaces {
        // Skip private namespaces
        if !namespace.name.is_empty() && !public_namespaces.contains(&namespace.name) {
            continue;
        }

        // Resolve and add all references
        for (_name, source_path) in &namespace.references {
            let mut visited = Vec::new();
            match resolve_reference(source_path, &reference_map, &symbol_map, &mut visited) {
                Ok(_symbol) => {
                    let current_modules = if namespace.name.is_empty() {
                        vec![String::new()]
                    } else {
                        namespace.name.split("::").map(String::from).collect()
                    };
                    if let Some(modules) = module_map.get_mut(&source_path.to_string()) {
                        *modules = modules
                            .iter()
                            .filter(|m| m.is_empty() || public_namespaces.contains(*m))
                            .cloned()
                            .chain(current_modules)
                            .collect::<Vec<_>>();
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    // Third pass: collect all symbols with their modules
    for (qualified_name, symbol) in symbol_map {
        if let Some(modules) = module_map.get(&qualified_name) {
            resolved_symbols.push(ResolvedSymbol {
                symbol,
                modules: modules.clone(),
            });
        }
    }

    Ok(SymbolResolution {
        symbols: resolved_symbols,
        doc_comments,
    })
}

fn resolve_reference<'a>(
    path: &str,
    reference_map: &HashMap<String, String>,
    symbol_map: &'a HashMap<String, Symbol>,
    visited: &mut Vec<String>,
) -> Result<&'a Symbol, LaibraryError> {
    if visited.contains(&path.to_string()) {
        return Err(LaibraryError::Parse(format!(
            "Circular reference detected while resolving '{}'",
            path
        )));
    }
    visited.push(path.to_string());

    if let Some(symbol) = symbol_map.get(path) {
        Ok(symbol)
    } else if let Some(next_path) = reference_map.get(path) {
        resolve_reference(next_path, reference_map, symbol_map, visited)
    } else {
        Err(LaibraryError::Parse(format!(
            "Could not resolve symbol reference '{}'",
            path
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Symbol;

    #[test]
    fn symbol_definition() {
        let raw_namespaces = vec![RawNamespace {
            name: String::new(),
            definitions: vec![Symbol {
                name: "test".to_string(),
                source_code: "pub fn test() {}".to_string(),
            }],
            references: Vec::new(),
            is_public: true,
            doc_comment: None,
        }];

        let resolution = resolve_symbols(&raw_namespaces).unwrap();

        assert_eq!(resolution.symbols.len(), 1);
        assert_eq!(resolution.symbols[0].symbol.name, "test");
        assert_eq!(resolution.symbols[0].modules, vec![String::new()]);
    }

    mod symbol_references {
        use assertables::assert_contains;

        use super::*;

        #[test]
        fn via_public_module() {
            let raw_namespaces = vec![
                RawNamespace {
                    name: String::new(),
                    definitions: Vec::new(),
                    references: vec![("test".to_string(), "inner::test".to_string())],
                    is_public: true,
                    doc_comment: None,
                },
                RawNamespace {
                    name: "inner".to_string(),
                    definitions: vec![Symbol {
                        name: "test".to_string(),
                        source_code: "pub fn test() {}".to_string(),
                    }],
                    references: Vec::new(),
                    is_public: true,
                    doc_comment: None,
                },
            ];

            let resolution = resolve_symbols(&raw_namespaces).unwrap();

            assert_eq!(resolution.symbols.len(), 1);
            let symbol = &resolution.symbols[0];
            assert_eq!(symbol.symbol.name, "test");
            assert_contains!(&symbol.modules, &String::new());
            assert_contains!(&symbol.modules, &"inner".to_string());
        }

        #[test]
        fn via_private_module() {
            let raw_namespaces = vec![
                RawNamespace {
                    name: String::new(),
                    definitions: Vec::new(),
                    references: vec![("test".to_string(), "inner::test".to_string())],
                    is_public: true,
                    doc_comment: None,
                },
                RawNamespace {
                    name: "inner".to_string(),
                    definitions: vec![Symbol {
                        name: "test".to_string(),
                        source_code: "pub fn test() {}".to_string(),
                    }],
                    references: Vec::new(),
                    is_public: false,
                    doc_comment: None,
                },
            ];

            let resolution = resolve_symbols(&raw_namespaces).unwrap();

            assert_eq!(resolution.symbols.len(), 1);
            let symbol = &resolution.symbols[0];
            assert_eq!(symbol.modules, vec![String::new()]);
            assert_eq!(symbol.symbol.name, "test");
        }

        #[test]
        fn missing_reference() {
            let raw_namespaces = vec![RawNamespace {
                name: String::new(),
                definitions: Vec::new(),
                references: vec![("test".to_string(), "missing::test".to_string())],
                is_public: true,
                doc_comment: None,
            }];

            let result = resolve_symbols(&raw_namespaces);

            assert!(matches!(result, Err(LaibraryError::Parse(_))));
        }

        #[test]
        fn circular_reference() {
            let raw_namespaces = vec![
                RawNamespace {
                    name: String::new(),
                    definitions: Vec::new(),
                    references: vec![("test".to_string(), "a::test".to_string())],
                    is_public: true,
                    doc_comment: None,
                },
                RawNamespace {
                    name: "a".to_string(),
                    definitions: Vec::new(),
                    references: vec![("test".to_string(), "b::test".to_string())],
                    is_public: true,
                    doc_comment: None,
                },
                RawNamespace {
                    name: "b".to_string(),
                    definitions: Vec::new(),
                    references: vec![("test".to_string(), "a::test".to_string())],
                    is_public: true,
                    doc_comment: None,
                },
            ];

            let result = resolve_symbols(&raw_namespaces);

            assert!(matches!(result, Err(LaibraryError::Parse(_))));
        }
    }

    mod doc_comments {
        use super::*;

        #[test]
        fn namespace_without_doc_comment() {
            let raw_namespaces = vec![RawNamespace {
                name: "text".to_string(),
                definitions: vec![],
                references: vec![],
                is_public: true,
                doc_comment: None,
            }];

            let resolution = resolve_symbols(&raw_namespaces).unwrap();

            assert!(resolution.doc_comments.is_empty());
        }

        #[test]
        fn namespace_with_doc_comment() {
            let raw_namespaces = vec![RawNamespace {
                name: "text".to_string(),
                definitions: vec![],
                references: vec![],
                is_public: true,
                doc_comment: Some("Module for text processing".to_string()),
            }];

            let resolution = resolve_symbols(&raw_namespaces).unwrap();
            assert_eq!(resolution.doc_comments.len(), 1);
            assert_eq!(
                resolution.doc_comments.get("text"),
                Some(&"Module for text processing".to_string())
            );
        }
    }
}
