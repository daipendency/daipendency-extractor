use crate::error::LaibraryError;
use crate::types::Symbol;
use std::collections::HashMap;

use super::symbol_collection::RawNamespace;

#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    pub symbol: Symbol,
    pub modules: Vec<String>,
}

/// Resolve symbol references by matching them with their corresponding definitions.
pub fn resolve_symbols(
    raw_namespaces: &[RawNamespace],
) -> Result<Vec<ResolvedSymbol>, LaibraryError> {
    let mut symbol_map: HashMap<String, Symbol> = HashMap::new();
    let mut reference_map: HashMap<String, String> = HashMap::new();
    let mut resolved_symbols = Vec::new();
    let mut public_namespaces = std::collections::HashSet::new();

    // First pass: collect all symbol definitions and references
    for namespace in raw_namespaces {
        // The root namespace and public modules are public
        if namespace.name.is_empty() || namespace.is_public {
            public_namespaces.insert(namespace.name.clone());
        }

        for symbol in &namespace.definitions {
            let qualified_name = if namespace.name.is_empty() {
                symbol.name.clone()
            } else {
                format!("{}::{}", namespace.name, symbol.name)
            };
            symbol_map.insert(qualified_name, symbol.clone());
        }
        for (name, source_path) in &namespace.references {
            let qualified_name = if namespace.name.is_empty() {
                name.clone()
            } else {
                format!("{}::{}", namespace.name, name)
            };
            reference_map.insert(qualified_name, source_path.clone());
        }
    }

    // Third pass: resolve references and collect symbols
    for namespace in raw_namespaces {
        // Skip private namespaces
        if !namespace.name.is_empty() && !public_namespaces.contains(&namespace.name) {
            continue;
        }

        // Add all definitions
        for symbol in &namespace.definitions {
            let modules = if namespace.name.is_empty() {
                Vec::new()
            } else {
                namespace.name.split("::").map(String::from).collect()
            };
            resolved_symbols.push(ResolvedSymbol {
                symbol: symbol.clone(),
                modules,
            });
        }

        // Resolve and add all references
        for (name, source_path) in &namespace.references {
            let mut visited = Vec::new();
            match resolve_reference(source_path, &reference_map, &symbol_map, &mut visited) {
                Ok(symbol) => {
                    let modules = if namespace.name.is_empty() {
                        Vec::new()
                    } else {
                        namespace.name.split("::").map(String::from).collect()
                    };
                    resolved_symbols.push(ResolvedSymbol {
                        symbol: Symbol {
                            name: name.clone(),
                            source_code: symbol.source_code.clone(),
                        },
                        modules,
                    });
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    Ok(resolved_symbols)
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
    fn test_resolve_symbols_simple() {
        let raw_namespaces = vec![RawNamespace {
            name: String::new(),
            definitions: vec![Symbol {
                name: "test".to_string(),
                source_code: "pub fn test() {}".to_string(),
            }],
            references: Vec::new(),
            is_public: true,
        }];

        let resolved = resolve_symbols(&raw_namespaces).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].symbol.name, "test");
        assert_eq!(resolved[0].modules.len(), 0);
    }

    #[test]
    fn test_resolve_symbols_with_reference() {
        let raw_namespaces = vec![
            RawNamespace {
                name: String::new(),
                definitions: Vec::new(),
                references: vec![("test".to_string(), "inner::test".to_string())],
                is_public: true,
            },
            RawNamespace {
                name: "inner".to_string(),
                definitions: vec![Symbol {
                    name: "test".to_string(),
                    source_code: "pub fn test() {}".to_string(),
                }],
                references: Vec::new(),
                is_public: true,
            },
        ];

        let resolved = resolve_symbols(&raw_namespaces).unwrap();
        assert_eq!(resolved.len(), 2);

        let root_symbol = resolved.iter().find(|s| s.modules.is_empty()).unwrap();
        assert_eq!(root_symbol.symbol.name, "test");

        let inner_symbol = resolved
            .iter()
            .find(|s| s.modules == vec!["inner"])
            .unwrap();
        assert_eq!(inner_symbol.symbol.name, "test");
    }

    #[test]
    fn test_resolve_symbols_missing_reference() {
        let raw_namespaces = vec![RawNamespace {
            name: String::new(),
            definitions: Vec::new(),
            references: vec![("test".to_string(), "missing::test".to_string())],
            is_public: true,
        }];

        let result = resolve_symbols(&raw_namespaces);
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_resolve_symbols_reexport_chain() {
        let raw_namespaces = vec![
            RawNamespace {
                name: String::new(),
                definitions: Vec::new(),
                references: vec![("Format".to_string(), "formatting::Format".to_string())],
                is_public: true,
            },
            RawNamespace {
                name: "formatting".to_string(),
                definitions: Vec::new(),
                references: vec![(
                    "Format".to_string(),
                    "formatting::format::Format".to_string(),
                )],
                is_public: true,
            },
            RawNamespace {
                name: "formatting::format".to_string(),
                definitions: vec![Symbol {
                    name: "Format".to_string(),
                    source_code: "pub enum Format { Markdown, Html, Plain }".to_string(),
                }],
                references: Vec::new(),
                is_public: true,
            },
        ];

        let resolved = resolve_symbols(&raw_namespaces).unwrap();
        assert_eq!(resolved.len(), 3);

        let root_symbol = resolved.iter().find(|s| s.modules.is_empty()).unwrap();
        assert_eq!(root_symbol.symbol.name, "Format");
        assert!(root_symbol
            .symbol
            .source_code
            .contains("pub enum Format { Markdown, Html, Plain }"));

        let formatting_symbol = resolved
            .iter()
            .find(|s| s.modules == vec!["formatting"])
            .unwrap();
        assert_eq!(formatting_symbol.symbol.name, "Format");
        assert!(formatting_symbol
            .symbol
            .source_code
            .contains("pub enum Format { Markdown, Html, Plain }"));

        let format_symbol = resolved
            .iter()
            .find(|s| s.modules == vec!["formatting", "format"])
            .unwrap();
        assert_eq!(format_symbol.symbol.name, "Format");
        assert!(format_symbol
            .symbol
            .source_code
            .contains("pub enum Format { Markdown, Html, Plain }"));
    }

    #[test]
    fn test_resolve_symbols_multiple_reexports() {
        let raw_namespaces = vec![
            RawNamespace {
                name: String::new(),
                definitions: Vec::new(),
                references: vec![
                    ("Format".to_string(), "text::Format".to_string()),
                    (
                        "TextFormatter".to_string(),
                        "text::TextFormatter".to_string(),
                    ),
                ],
                is_public: true,
            },
            RawNamespace {
                name: "text".to_string(),
                definitions: Vec::new(),
                references: vec![
                    ("Format".to_string(), "text::formatter::Format".to_string()),
                    (
                        "TextFormatter".to_string(),
                        "text::formatter::TextFormatter".to_string(),
                    ),
                ],
                is_public: true,
            },
            RawNamespace {
                name: "text::formatter".to_string(),
                definitions: vec![
                    Symbol {
                        name: "Format".to_string(),
                        source_code: "pub enum Format { Plain, Rich }".to_string(),
                    },
                    Symbol {
                        name: "TextFormatter".to_string(),
                        source_code: "pub struct TextFormatter { format: Format }".to_string(),
                    },
                ],
                references: Vec::new(),
                is_public: true,
            },
        ];

        let resolved = resolve_symbols(&raw_namespaces).unwrap();
        assert_eq!(resolved.len(), 6);

        // Root namespace
        let root_symbols: Vec<_> = resolved.iter().filter(|s| s.modules.is_empty()).collect();
        assert_eq!(root_symbols.len(), 2);
        assert!(
            root_symbols
                .iter()
                .any(|s| s.symbol.name == "Format"
                    && s.symbol.source_code.contains("pub enum Format"))
        );
        assert!(root_symbols.iter().any(|s| s.symbol.name == "TextFormatter"
            && s.symbol.source_code.contains("pub struct TextFormatter")));

        // Text namespace
        let text_symbols: Vec<_> = resolved
            .iter()
            .filter(|s| s.modules == vec!["text"])
            .collect();
        assert_eq!(text_symbols.len(), 2);
        assert!(
            text_symbols
                .iter()
                .any(|s| s.symbol.name == "Format"
                    && s.symbol.source_code.contains("pub enum Format"))
        );
        assert!(text_symbols.iter().any(|s| s.symbol.name == "TextFormatter"
            && s.symbol.source_code.contains("pub struct TextFormatter")));

        // Formatter namespace
        let formatter_symbols: Vec<_> = resolved
            .iter()
            .filter(|s| s.modules == vec!["text", "formatter"])
            .collect();
        assert_eq!(formatter_symbols.len(), 2);
        assert!(
            formatter_symbols
                .iter()
                .any(|s| s.symbol.name == "Format"
                    && s.symbol.source_code.contains("pub enum Format"))
        );
        assert!(formatter_symbols
            .iter()
            .any(|s| s.symbol.name == "TextFormatter"
                && s.symbol.source_code.contains("pub struct TextFormatter")));
    }

    #[test]
    fn test_resolve_symbols_circular_reference() {
        let raw_namespaces = vec![
            RawNamespace {
                name: String::new(),
                definitions: Vec::new(),
                references: vec![("test".to_string(), "a::test".to_string())],
                is_public: true,
            },
            RawNamespace {
                name: "a".to_string(),
                definitions: Vec::new(),
                references: vec![("test".to_string(), "b::test".to_string())],
                is_public: true,
            },
            RawNamespace {
                name: "b".to_string(),
                definitions: Vec::new(),
                references: vec![("test".to_string(), "a::test".to_string())],
                is_public: true,
            },
        ];

        let result = resolve_symbols(&raw_namespaces);
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_resolve_symbols_through_private_module() {
        let raw_namespaces = vec![
            RawNamespace {
                name: String::new(),
                definitions: Vec::new(),
                references: Vec::new(),
                is_public: true,
            },
            RawNamespace {
                name: "text".to_string(),
                definitions: Vec::new(),
                references: vec![
                    ("Format".to_string(), "text::formatter::Format".to_string()),
                    (
                        "TextFormatter".to_string(),
                        "text::formatter::TextFormatter".to_string(),
                    ),
                ],
                is_public: true,
            },
            RawNamespace {
                name: "text::formatter".to_string(),
                definitions: vec![
                    Symbol {
                        name: "Format".to_string(),
                        source_code: "pub enum Format { Plain, Rich }".to_string(),
                    },
                    Symbol {
                        name: "TextFormatter".to_string(),
                        source_code: "pub struct TextFormatter { format: Format }".to_string(),
                    },
                ],
                references: Vec::new(),
                is_public: false,
            },
        ];

        let resolved = resolve_symbols(&raw_namespaces).unwrap();

        // We should have symbols in both the root and text namespaces
        let text_symbols: Vec<_> = resolved
            .iter()
            .filter(|s| s.modules == vec!["text"])
            .collect();
        assert_eq!(text_symbols.len(), 2);
        assert!(text_symbols.iter().any(|s| s.symbol.name == "Format"));
        assert!(text_symbols
            .iter()
            .any(|s| s.symbol.name == "TextFormatter"));

        // The formatter module's symbols should not appear in their original location
        assert!(!resolved
            .iter()
            .any(|s| s.modules == vec!["text", "formatter"]));
    }
}
