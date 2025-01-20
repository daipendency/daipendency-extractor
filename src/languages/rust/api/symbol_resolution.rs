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
    let mut resolved_symbols: HashMap<String, ResolvedSymbol> = HashMap::new();
    let mut reference_map: HashMap<String, String> = HashMap::new();

    // First pass: collect all symbol definitions and references from ALL modules
    for module in modules {
        for symbol in &module.definitions {
            let symbol_path = get_symbol_path(&symbol.name, module);
            resolved_symbols.insert(
                symbol_path.clone(),
                ResolvedSymbol {
                    symbol: symbol.clone(),
                    modules: vec![module.name.clone()],
                },
            );
        }
        for source_path in &module.references {
            let symbol_name = source_path.split("::").last().unwrap();
            let symbol_path = get_symbol_path(symbol_name, module);
            reference_map.insert(symbol_name.to_string(), symbol_path);
        }
    }

    let public_modules: Vec<&Module> = modules
        .iter()
        .filter(|m| m.name.is_empty() || m.is_public)
        .collect();

    // Second pass: resolve references and collect symbols
    for module in &public_modules {
        // Resolve and add all references
        for source_path in &module.references {
            let mut visited = Vec::new();
            match resolve_reference(source_path, &reference_map, &resolved_symbols, &mut visited) {
                Ok(_symbol) => {
                    let current_modules = vec![module.name.clone()];
                    if let Some(resolved) = resolved_symbols.get_mut(source_path) {
                        resolved.modules = resolved
                            .modules
                            .iter()
                            .filter(|m| public_modules.iter().any(|pm| &pm.name == *m))
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

    let doc_comments = public_modules
        .iter()
        .filter_map(|module| {
            module
                .doc_comment
                .as_ref()
                .map(|doc| (module.name.clone(), doc.clone()))
        })
        .collect();

    Ok(SymbolResolution {
        symbols: resolved_symbols.into_values().collect(),
        doc_comments,
    })
}

fn get_symbol_path(symbol_name: &str, module: &Module) -> String {
    if module.name.is_empty() {
        symbol_name.to_string()
    } else {
        format!("{}::{}", module.name, symbol_name)
    }
}

fn resolve_reference<'a>(
    path: &str,
    reference_map: &HashMap<String, String>,
    resolved_symbols: &'a HashMap<String, ResolvedSymbol>,
    visited: &mut Vec<String>,
) -> Result<&'a Symbol, LaibraryError> {
    if visited.contains(&path.to_string()) {
        return Err(LaibraryError::Parse(format!(
            "Circular reference detected while resolving '{}'",
            path
        )));
    }
    visited.push(path.to_string());

    if let Some(resolved) = resolved_symbols.get(path) {
        Ok(&resolved.symbol)
    } else if let Some(next_path) = reference_map.get(path) {
        resolve_reference(next_path, reference_map, resolved_symbols, visited)
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

    mod symbol_definitions {
        use super::*;
        use crate::test_helpers::stub_symbol;

        #[test]
        fn at_root() {
            let symbol = stub_symbol();
            let modules = vec![Module {
                name: String::new(),
                definitions: vec![symbol.clone()],
                references: Vec::new(),
                is_public: true,
                doc_comment: None,
            }];

            let resolution = resolve_symbols(&modules).unwrap();

            assert_eq!(resolution.symbols.len(), 1);
            assert_eq!(resolution.symbols[0].symbol, symbol);
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

    mod reexports {
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

            assert!(matches!(
                result,
                Err(LaibraryError::Parse(msg)) if msg == "Could not resolve symbol reference 'missing::test'"
            ));
        }

        #[test]
        fn self_referential_symbol() {
            let modules = vec![Module {
                name: String::new(),
                definitions: Vec::new(),
                references: vec!["test".to_string()],
                is_public: true,
                doc_comment: None,
            }];

            let result = resolve_symbols(&modules);

            assert!(matches!(
                result,
                Err(LaibraryError::Parse(msg)) if msg == "Circular reference detected while resolving 'test'"
            ));
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
