use crate::error::LaibraryError;
use crate::types::Symbol;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

use super::parsing;

#[derive(Debug, Clone)]
pub struct RawNamespace {
    pub name: String,
    pub definitions: Vec<Symbol>,
    pub references: Vec<(String, String)>, // (name, source_path)
    pub is_public: bool,
}

/// Traverse the source files of the Rust crate and collect all symbols and symbol references (reexports).
pub fn collect_symbols(
    entry_point: &Path,
    parser: &mut Parser,
) -> Result<Vec<RawNamespace>, LaibraryError> {
    let mut visited_files = HashMap::new();
    let mut namespaces = Vec::new();
    collect_symbols_recursive(
        entry_point,
        "",
        true,
        parser,
        &mut visited_files,
        &mut namespaces,
    )?;
    Ok(namespaces)
}

fn collect_symbols_recursive(
    file_path: &Path,
    namespace_prefix: &str,
    is_public: bool,
    parser: &mut Parser,
    visited_files: &mut HashMap<PathBuf, bool>,
    namespaces: &mut Vec<RawNamespace>,
) -> Result<(), LaibraryError> {
    if visited_files.contains_key(&file_path.to_path_buf()) {
        return Ok(());
    }

    let content = std::fs::read_to_string(file_path).map_err(|e| {
        LaibraryError::Parse(format!(
            "Failed to read file '{}': {}",
            file_path.display(),
            e
        ))
    })?;

    visited_files.insert(file_path.to_path_buf(), true);
    let file_symbols = parsing::parse_rust_file(&content, parser)?;

    let mut current_namespace = RawNamespace {
        name: namespace_prefix.to_string(),
        definitions: Vec::new(),
        references: Vec::new(),
        is_public,
    };

    for symbol in file_symbols {
        match symbol {
            super::types::RustSymbol::Symbol { symbol, .. } => {
                current_namespace.definitions.push(symbol);
            }
            super::types::RustSymbol::Module { name, content } => {
                let module_namespace = format!(
                    "{}{}{}",
                    namespace_prefix,
                    if namespace_prefix.is_empty() {
                        ""
                    } else {
                        "::"
                    },
                    name
                );
                let mut module_raw_namespace = RawNamespace {
                    name: module_namespace.clone(),
                    definitions: Vec::new(),
                    references: Vec::new(),
                    is_public,
                };
                for symbol in content {
                    if let super::types::RustSymbol::Symbol { symbol, .. } = symbol {
                        module_raw_namespace.definitions.push(symbol);
                    }
                }
                namespaces.push(module_raw_namespace);
            }
            super::types::RustSymbol::ModuleDeclaration {
                name,
                is_public: module_is_public,
                ..
            } => {
                if let Ok(module_path) = resolve_module_path(file_path, &name) {
                    let module_namespace = format!(
                        "{}{}{}",
                        namespace_prefix,
                        if namespace_prefix.is_empty() {
                            ""
                        } else {
                            "::"
                        },
                        name
                    );
                    collect_symbols_recursive(
                        &module_path,
                        &module_namespace,
                        module_is_public,
                        parser,
                        visited_files,
                        namespaces,
                    )?;
                }
            }
            super::types::RustSymbol::SymbolReexport {
                name, source_path, ..
            } => {
                let source_path = if source_path.starts_with("self::") {
                    source_path[6..].to_string()
                } else if source_path.contains("::") {
                    if namespace_prefix.is_empty() {
                        source_path.clone()
                    } else {
                        format!("{}::{}", namespace_prefix, source_path)
                    }
                } else {
                    if namespace_prefix.is_empty() {
                        source_path.clone()
                    } else {
                        format!("{}::{}", namespace_prefix, source_path)
                    }
                };

                current_namespace.references.push((name, source_path));
            }
        }
    }

    namespaces.push(current_namespace);
    Ok(())
}

fn resolve_module_path(current_file: &Path, module_name: &str) -> Result<PathBuf, LaibraryError> {
    let parent = current_file.parent().ok_or_else(|| {
        LaibraryError::Parse(format!(
            "Failed to get parent directory of {}",
            current_file.display()
        ))
    })?;

    let mod_rs_path = parent.join(module_name).join("mod.rs");
    if mod_rs_path.exists() {
        return Ok(mod_rs_path);
    }

    let rs_path = parent.join(format!("{}.rs", module_name));
    if rs_path.exists() {
        return Ok(rs_path);
    }

    Err(LaibraryError::Parse(format!(
        "Could not find module {} from {}",
        module_name,
        current_file.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::test_helpers::setup_parser;
    use crate::test_helpers::{create_file, create_temp_dir};

    #[test]
    fn test_collect_symbols_single_file() {
        let temp_dir = create_temp_dir();
        let lib_rs = temp_dir.path().join("src").join("lib.rs");
        create_file(
            &lib_rs,
            r#"
pub fn public_function() {}
fn private_function() {}
"#,
        );

        let mut parser = setup_parser();
        let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 1);
        assert_eq!(namespaces[0].name, "");
        assert_eq!(
            namespaces[0].definitions.len(),
            1,
            "Should only collect public functions"
        );
        assert_eq!(namespaces[0].references.len(), 0);

        let definitions = &namespaces[0].definitions;
        assert!(
            definitions.iter().any(|s| s.name == "public_function"),
            "Should collect public function"
        );
    }

    #[test]
    fn test_collect_symbols_with_module() {
        let temp_dir = create_temp_dir();
        let lib_rs = temp_dir.path().join("src").join("lib.rs");
        let module_rs = temp_dir.path().join("src").join("module.rs");

        create_file(
            &lib_rs,
            r#"
pub mod module;
pub fn root_function() {}
"#,
        );
        create_file(
            &module_rs,
            r#"
pub fn module_function() {}
"#,
        );

        let mut parser = setup_parser();
        let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 2);
        let root = namespaces.iter().find(|n| n.name.is_empty()).unwrap();
        assert_eq!(root.definitions.len(), 1);
        assert_eq!(root.references.len(), 0);

        let module = namespaces.iter().find(|n| n.name == "module").unwrap();
        assert_eq!(module.definitions.len(), 1);
        assert_eq!(module.references.len(), 0);
    }

    #[test]
    fn test_collect_symbols_nested_module() {
        let temp_dir = create_temp_dir();
        let lib_rs = temp_dir.path().join("src").join("lib.rs");
        let module_dir = temp_dir.path().join("src").join("module");
        let mod_rs = module_dir.join("mod.rs");

        create_file(
            &lib_rs,
            r#"
pub mod module;
pub fn root_function() {}
"#,
        );
        create_file(
            &mod_rs,
            r#"
pub fn module_function() {}
"#,
        );

        let mut parser = setup_parser();
        let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 2);
        let root = namespaces.iter().find(|n| n.name.is_empty()).unwrap();
        assert_eq!(root.definitions.len(), 1);
        assert_eq!(root.references.len(), 0);

        let module = namespaces.iter().find(|n| n.name == "module").unwrap();
        assert_eq!(module.definitions.len(), 1);
        assert_eq!(module.references.len(), 0);
    }

    #[test]
    fn test_collect_symbols_missing_module() {
        let temp_dir = create_temp_dir();
        let lib_rs = temp_dir.path().join("src").join("lib.rs");
        create_file(
            &lib_rs,
            r#"
pub mod missing;
pub fn root_function() {}
"#,
        );

        let mut parser = setup_parser();
        let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 1);
        assert_eq!(namespaces[0].name, "");
        assert_eq!(namespaces[0].definitions.len(), 1);
        assert_eq!(namespaces[0].references.len(), 0);
    }

    #[test]
    fn test_collect_symbols_module_reexport() {
        let temp_dir = create_temp_dir();
        let lib_rs = temp_dir.path().join("src").join("lib.rs");
        let module_rs = temp_dir.path().join("src").join("module.rs");

        create_file(
            &lib_rs,
            r#"
pub mod module;
pub use module::InnerStruct;
"#,
        );
        create_file(
            &module_rs,
            r#"
pub struct InnerStruct;
"#,
        );

        let mut parser = setup_parser();
        let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 2);
        let root = namespaces.iter().find(|n| n.name.is_empty()).unwrap();
        assert_eq!(root.definitions.len(), 0);
        assert_eq!(root.references.len(), 1);
        assert_eq!(root.references[0].0, "InnerStruct");
        assert_eq!(root.references[0].1, "module::InnerStruct");

        let module = namespaces.iter().find(|n| n.name == "module").unwrap();
        assert_eq!(module.definitions.len(), 1);
        assert_eq!(module.references.len(), 0);
        assert_eq!(module.definitions[0].name, "InnerStruct");
    }

    #[test]
    fn test_collect_symbols_module_internal_reexport() {
        let temp_dir = create_temp_dir();
        let lib_rs = temp_dir.path().join("src").join("lib.rs");
        let text_dir = temp_dir.path().join("src").join("text");
        let text_mod_rs = text_dir.join("mod.rs");
        let formatter_rs = text_dir.join("formatter.rs");

        create_file(
            &lib_rs,
            r#"
pub mod text;
"#,
        );
        create_file(
            &text_mod_rs,
            r#"
mod formatter;
pub use formatter::{Format, TextFormatter};
"#,
        );
        create_file(
            &formatter_rs,
            r#"
pub struct TextFormatter {
    format: Format,
}

pub enum Format {
    Plain,
    Rich,
}
"#,
        );

        let mut parser = setup_parser();
        let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 3);
        let root = namespaces.iter().find(|n| n.name.is_empty()).unwrap();
        assert_eq!(root.definitions.len(), 0);
        assert_eq!(root.references.len(), 0);

        let text = namespaces.iter().find(|n| n.name == "text").unwrap();
        assert_eq!(text.definitions.len(), 0);
        assert_eq!(text.references.len(), 2);
        assert!(text.references.iter().any(|(name, source_path)| {
            name == "Format" && source_path == "text::formatter::Format"
        }));
        assert!(text.references.iter().any(|(name, source_path)| {
            name == "TextFormatter" && source_path == "text::formatter::TextFormatter"
        }));

        let formatter = namespaces
            .iter()
            .find(|n| n.name == "text::formatter")
            .unwrap();
        assert_eq!(formatter.definitions.len(), 2);
        assert_eq!(formatter.references.len(), 0);
        assert!(formatter.definitions.iter().any(|s| s.name == "Format"));
        assert!(formatter
            .definitions
            .iter()
            .any(|s| s.name == "TextFormatter"));
    }

    #[test]
    fn test_collect_symbols_private_module() {
        let temp_dir = create_temp_dir();
        let lib_rs = temp_dir.path().join("src").join("lib.rs");
        let text_dir = temp_dir.path().join("src").join("text");
        let text_mod_rs = text_dir.join("mod.rs");
        let formatter_rs = text_dir.join("formatter.rs");

        create_file(
            &lib_rs,
            r#"
pub mod text;
"#,
        );
        create_file(
            &text_mod_rs,
            r#"
mod formatter;
pub use formatter::{Format, TextFormatter};
"#,
        );
        create_file(
            &formatter_rs,
            r#"
pub struct TextFormatter {
    format: Format,
}

pub enum Format {
    Plain,
    Rich,
}
"#,
        );

        let mut parser = setup_parser();
        let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();
        let formatter = namespaces
            .iter()
            .find(|n| n.name == "text::formatter")
            .unwrap();
        assert_eq!(formatter.definitions.len(), 2);
        assert!(formatter.definitions.iter().any(|s| s.name == "Format"));
        assert!(formatter
            .definitions
            .iter()
            .any(|s| s.name == "TextFormatter"));
        assert!(!formatter.is_public);

        // But its symbols should be reexported in the text module
        let text = namespaces.iter().find(|n| n.name == "text").unwrap();
        assert_eq!(text.references.len(), 2);
        assert!(text.references.iter().any(|(name, _)| name == "Format"));
        assert!(text
            .references
            .iter()
            .any(|(name, _)| name == "TextFormatter"));
    }

    #[test]
    fn test_collect_symbols_reexport_chain() {
        let temp_dir = create_temp_dir();
        let lib_rs = temp_dir.path().join("src").join("lib.rs");
        let formatting_dir = temp_dir.path().join("src").join("formatting");
        let formatting_mod_rs = formatting_dir.join("mod.rs");
        let format_rs = formatting_dir.join("format.rs");

        create_file(
            &lib_rs,
            r#"
mod formatting;
pub use formatting::Format;
"#,
        );
        create_file(
            &formatting_mod_rs,
            r#"
mod format;
pub use format::Format;
"#,
        );
        create_file(
            &format_rs,
            r#"
pub enum Format {
    Markdown,
    Html,
    Plain,
}
"#,
        );

        let mut parser = setup_parser();
        let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 3);
        let root = namespaces.iter().find(|n| n.name.is_empty()).unwrap();
        assert_eq!(root.definitions.len(), 0);
        assert_eq!(root.references.len(), 1);
        assert_eq!(root.references[0].0, "Format");
        assert_eq!(root.references[0].1, "formatting::Format");

        let formatting = namespaces.iter().find(|n| n.name == "formatting").unwrap();
        assert_eq!(formatting.definitions.len(), 0);
        assert_eq!(formatting.references.len(), 1);
        assert_eq!(formatting.references[0].0, "Format");
        assert_eq!(formatting.references[0].1, "formatting::format::Format");

        let format = namespaces
            .iter()
            .find(|n| n.name == "formatting::format")
            .unwrap();
        assert_eq!(format.definitions.len(), 1);
        assert_eq!(format.references.len(), 0);
        assert_eq!(format.definitions[0].name, "Format");
    }

    #[test]
    fn test_collect_symbols_nonexistent_file() {
        let path = PathBuf::from("nonexistent.rs");
        let mut parser = setup_parser();

        let result = collect_symbols(&path, &mut parser);
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }
}
