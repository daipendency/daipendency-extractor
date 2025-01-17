use crate::error::LaibraryError;
use crate::types::Symbol;
use std::collections::HashMap;

use super::symbol_collection::Module;

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
pub fn resolve_symbols(modules: &[Module]) -> Result<SymbolResolution, LaibraryError> {
    let mut symbol_map: HashMap<String, Symbol> = HashMap::new();
    let mut reference_map: HashMap<String, String> = HashMap::new();
    let mut module_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut doc_comments = HashMap::new();
    let mut public_module_paths = std::collections::HashSet::new();

    // First pass: collect all symbol definitions and references
    for module in modules {
        // The root module and public modules are public
        if module.name.is_empty() || module.is_public {
            public_module_paths.insert(module.name.clone());

            if let Some(doc_comment) = &module.doc_comment {
                doc_comments.insert(module.name.clone(), doc_comment.clone());
            }
        }

        for symbol in &module.definitions {
            let qualified_name = if module.name.is_empty() {
                symbol.name.clone()
            } else {
                format!("{}::{}", module.name, symbol.name)
            };
            symbol_map.insert(qualified_name.clone(), symbol.clone());
            let modules = vec![module.name.clone()];
            module_map.entry(qualified_name).or_insert(modules);
        }
        for source_path in &module.references {
            let symbol_name = source_path.split("::").last().unwrap().to_string();
            let qualified_path = if module.name.is_empty() {
                symbol_name.clone()
            } else {
                format!("{}::{}", module.name, source_path)
            };
            reference_map.insert(symbol_name, qualified_path);
        }
    }

    // Second pass: resolve references and collect symbols
    let mut resolved_symbols = Vec::new();
    for module in modules {
        if !module.name.is_empty() && !public_module_paths.contains(&module.name) {
            continue;
        }

        // Resolve and add all references
        for source_path in &module.references {
            let mut visited = Vec::new();
            match resolve_reference(source_path, &reference_map, &symbol_map, &mut visited) {
                Ok(_symbol) => {
                    let current_modules = vec![module.name.clone()];
                    if let Some(modules) = module_map.get_mut(source_path) {
                        *modules = modules
                            .iter()
                            .filter(|m| m.is_empty() || public_module_paths.contains(*m))
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

    mod symbol_definitions {
        use super::*;
        use crate::test_helpers::stub_symbol;

        #[test]
        fn at_root() {
            let modules = vec![Module {
                name: String::new(),
                definitions: vec![Symbol {
                    name: "test".to_string(),
                    source_code: "pub fn test() {}".to_string(),
                }],
                references: Vec::new(),
                is_public: true,
                doc_comment: None,
            }];

            let resolution = resolve_symbols(&modules).unwrap();

            assert_eq!(resolution.symbols.len(), 1);
            assert_eq!(resolution.symbols[0].symbol.name, "test");
            assert_eq!(resolution.symbols[0].modules, vec![String::new()]);
        }

        #[test]
        fn at_submodule() {
            let symbol = stub_symbol();
            let modules = vec![Module {
                name: "outer::inner".to_string(),
                definitions: vec![symbol.clone()],
                references: Vec::new(),
                is_public: true,
                doc_comment: None,
            }];

            let resolution = resolve_symbols(&modules).unwrap();

            assert_eq!(resolution.symbols.len(), 1);
            let resolved_symbol = &resolution.symbols[0];
            assert_eq!(resolved_symbol.modules, vec!["outer::inner"]);
        }
    }

    mod symbol_references {
        use assertables::assert_contains;

        use crate::test_helpers::stub_symbol;

        use super::*;

        #[test]
        fn via_public_module() {
            let symbol = stub_symbol();
            let modules = vec![
                Module {
                    name: String::new(),
                    definitions: Vec::new(),
                    references: vec!["inner::test".to_string()],
                    is_public: true,
                    doc_comment: None,
                },
                Module {
                    name: "inner".to_string(),
                    definitions: vec![symbol.clone()],
                    references: Vec::new(),
                    is_public: true,
                    doc_comment: None,
                },
            ];

            let resolution = resolve_symbols(&modules).unwrap();

            assert_eq!(resolution.symbols.len(), 1);
            let resolved_symbol = &resolution.symbols[0];
            assert_eq!(resolved_symbol.symbol, symbol);
            assert_contains!(&resolved_symbol.modules, &String::new());
            assert_contains!(&resolved_symbol.modules, &"inner".to_string());
        }

        #[test]
        fn via_private_module() {
            let symbol = stub_symbol();
            let modules = vec![
                Module {
                    name: String::new(),
                    definitions: Vec::new(),
                    references: vec!["inner::test".to_string()],
                    is_public: true,
                    doc_comment: None,
                },
                Module {
                    name: "inner".to_string(),
                    definitions: vec![symbol.clone()],
                    references: Vec::new(),
                    is_public: false,
                    doc_comment: None,
                },
            ];

            let resolution = resolve_symbols(&modules).unwrap();

            assert_eq!(resolution.symbols.len(), 1);
            let resolved_symbol = &resolution.symbols[0];
            assert_eq!(resolved_symbol.modules, vec![String::new()]);
            assert_eq!(resolved_symbol.symbol, symbol);
        }

        #[test]
        fn via_nested_public_module() {
            let symbol = stub_symbol();
            let modules = vec![
                Module {
                    name: "foo::bar".to_string(),
                    definitions: Vec::new(),
                    references: vec!["outer::inner::test".to_string()],
                    is_public: true,
                    doc_comment: None,
                },
                Module {
                    name: "outer::inner".to_string(),
                    definitions: vec![symbol.clone()],
                    references: Vec::new(),
                    is_public: true,
                    doc_comment: None,
                },
            ];

            let resolution = resolve_symbols(&modules).unwrap();

            assert_eq!(resolution.symbols.len(), 1);
            let resolved_symbol = &resolution.symbols[0];
            assert_eq!(resolved_symbol.symbol, symbol);
            assert_contains!(&resolved_symbol.modules, &"foo::bar".to_string());
            assert_contains!(&resolved_symbol.modules, &"outer::inner".to_string());
        }

        #[test]
        fn missing_reference() {
            let modules = vec![Module {
                name: String::new(),
                definitions: Vec::new(),
                references: vec!["missing::test".to_string()],
                is_public: true,
                doc_comment: None,
            }];

            let result = resolve_symbols(&modules);

            assert!(matches!(result, Err(LaibraryError::Parse(_))));
        }

        #[test]
        fn circular_reference() {
            let modules = vec![
                Module {
                    name: String::new(),
                    definitions: Vec::new(),
                    references: vec!["a::test".to_string()],
                    is_public: true,
                    doc_comment: None,
                },
                Module {
                    name: "a".to_string(),
                    definitions: Vec::new(),
                    references: vec!["b::test".to_string()],
                    is_public: true,
                    doc_comment: None,
                },
                Module {
                    name: "b".to_string(),
                    definitions: Vec::new(),
                    references: vec!["a::test".to_string()],
                    is_public: true,
                    doc_comment: None,
                },
            ];

            let result = resolve_symbols(&modules);

            assert!(matches!(result, Err(LaibraryError::Parse(_))));
        }
    }

    mod doc_comments {
        use super::*;

        #[test]
        fn namespace_without_doc_comment() {
            let modules = vec![Module {
                name: "text".to_string(),
                definitions: vec![],
                references: vec![],
                is_public: true,
                doc_comment: None,
            }];

            let resolution = resolve_symbols(&modules).unwrap();

            assert!(resolution.doc_comments.is_empty());
        }

        #[test]
        fn namespace_with_doc_comment() {
            let modules = vec![Module {
                name: "text".to_string(),
                definitions: vec![],
                references: vec![],
                is_public: true,
                doc_comment: Some("Module for text processing".to_string()),
            }];

            let resolution = resolve_symbols(&modules).unwrap();
            assert_eq!(resolution.doc_comments.len(), 1);
            assert_eq!(
                resolution.doc_comments.get("text"),
                Some(&"Module for text processing".to_string())
            );
        }
    }
}
