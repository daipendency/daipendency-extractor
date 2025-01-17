use crate::error::LaibraryError;
use crate::types::Symbol;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

use super::parsing;

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub definitions: Vec<Symbol>,
    pub references: Vec<String>,
    pub is_public: bool,
    pub doc_comment: Option<String>,
}

/// Traverse the source files of the Rust crate and collect all symbols and symbol references (reexports).
pub fn collect_symbols(
    entry_point: &Path,
    parser: &mut Parser,
) -> Result<Vec<Module>, LaibraryError> {
    let mut visited_files = HashMap::new();
    collect_symbols_recursive(entry_point, "", true, parser, &mut visited_files)
}

fn collect_symbols_recursive(
    file_path: &Path,
    namespace_prefix: &str,
    is_public: bool,
    parser: &mut Parser,
    visited_files: &mut HashMap<PathBuf, bool>,
) -> Result<Vec<Module>, LaibraryError> {
    if visited_files.contains_key(&file_path.to_path_buf()) {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(file_path).map_err(|e| {
        LaibraryError::Parse(format!(
            "Failed to read file '{}': {}",
            file_path.display(),
            e
        ))
    })?;

    visited_files.insert(file_path.to_path_buf(), true);
    let rust_file = parsing::parse_rust_file(&content, parser)?;

    let mut namespaces = Vec::new();
    let mut current_namespace = Module {
        name: namespace_prefix.to_string(),
        definitions: Vec::new(),
        references: Vec::new(),
        is_public,
        doc_comment: rust_file.doc_comment,
    };

    for symbol in rust_file.symbols {
        match symbol {
            parsing::RustSymbol::Symbol { symbol } => {
                current_namespace.definitions.push(symbol.clone());
            }
            parsing::RustSymbol::Module {
                name,
                content,
                doc_comment,
            } => {
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
                let mut module_raw_namespace = Module {
                    name: module_namespace.clone(),
                    definitions: Vec::new(),
                    references: Vec::new(),
                    is_public,
                    doc_comment,
                };
                for symbol in content {
                    if let parsing::RustSymbol::Symbol { symbol } = symbol {
                        module_raw_namespace.definitions.push(symbol.clone());
                    }
                }
                namespaces.push(module_raw_namespace);
            }
            parsing::RustSymbol::ModuleDeclaration {
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
                    let mut child_namespaces = collect_symbols_recursive(
                        &module_path,
                        &module_namespace,
                        module_is_public,
                        parser,
                        visited_files,
                    )?;
                    namespaces.append(&mut child_namespaces);
                }
            }
            parsing::RustSymbol::SymbolReexport { source_path } => {
                let source_path = if namespace_prefix.is_empty() {
                    source_path.clone()
                } else {
                    format!("{}::{}", namespace_prefix, source_path)
                };

                current_namespace.references.push(source_path);
            }
        }
    }

    namespaces.push(current_namespace);
    Ok(namespaces)
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
    fn non_existing_file() {
        let path = PathBuf::from("non-existing.rs");
        let mut parser = setup_parser();

        let result = collect_symbols(&path, &mut parser);
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn cyclic_modules() {
        let temp_dir = create_temp_dir();
        let module_a_rs = temp_dir.path().join("src").join("module_a.rs");
        let module_b_rs = temp_dir.path().join("src").join("module_b.rs");
        create_file(
            &module_a_rs,
            r#"
pub mod module_b;
pub fn module_a_function() {}
"#,
        );
        create_file(
            &module_b_rs,
            r#"
pub mod module_a;  // This creates a cycle
pub fn module_b_function() {}
"#,
        );
        let mut parser = setup_parser();

        // This should complete without infinite recursion
        let namespaces = collect_symbols(&module_a_rs, &mut parser).unwrap();

        assert!(!namespaces.is_empty());
    }

    mod exports {
        use super::*;

        #[test]
        fn public_symbol() {
            let temp_dir = create_temp_dir();
            let lib_rs = temp_dir.path().join("src").join("lib.rs");
            create_file(
                &lib_rs,
                r#"
pub fn public_function() {}
"#,
            );
            let mut parser = setup_parser();

            let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

            assert_eq!(namespaces.len(), 1);
            assert_eq!(namespaces[0].name, "");
            assert_eq!(namespaces[0].definitions.len(), 1);

            let definitions = &namespaces[0].definitions;
            assert!(definitions.iter().any(|s| s.name == "public_function"));
        }

        #[test]
        fn private_symbol() {
            let temp_dir = create_temp_dir();
            let lib_rs = temp_dir.path().join("src").join("lib.rs");
            create_file(
                &lib_rs,
                r#"
fn private_function() {}
"#,
            );
            let mut parser = setup_parser();

            let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

            assert_eq!(namespaces.len(), 1);
            assert_eq!(namespaces[0].name, "");
            assert_eq!(namespaces[0].definitions.len(), 0);
        }

        #[test]
        fn public_module() {
            let temp_dir = create_temp_dir();
            let lib_rs = temp_dir.path().join("src").join("lib.rs");
            create_file(
                &lib_rs,
                r#"
pub mod public_module {}
"#,
            );
            let mut parser = setup_parser();

            let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

            assert_eq!(namespaces.len(), 2);
            assert!(namespaces.iter().any(|n| n.name == "public_module"));
        }

        #[test]
        fn private_module() {
            let temp_dir = create_temp_dir();
            let lib_rs = temp_dir.path().join("src").join("lib.rs");
            create_file(
                &lib_rs,
                r#"
mod private_module {}
"#,
            );
            let mut parser = setup_parser();

            let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

            assert_eq!(namespaces.len(), 1);
            assert!(namespaces.iter().any(|n| n.name == ""));
        }
    }

    mod reexports {
        use super::*;

        #[test]
        fn missing_module() {
            let temp_dir = create_temp_dir();
            let lib_rs = temp_dir.path().join("src").join("lib.rs");
            create_file(
                &lib_rs,
                r#"
    pub mod missing;
    "#,
            );
            let mut parser = setup_parser();

            let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

            assert_eq!(namespaces.len(), 1);
            assert_eq!(namespaces[0].name, "");
            assert_eq!(namespaces[0].references.len(), 0);
        }

        #[test]
        fn module_reexport() {
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
            assert_eq!(root.references[0], "module::InnerStruct");

            let module = namespaces.iter().find(|n| n.name == "module").unwrap();
            assert_eq!(module.definitions.len(), 1);
            assert_eq!(module.references.len(), 0);
            assert_eq!(module.definitions[0].name, "InnerStruct");
        }

        #[test]
        fn direct_symbol_reexport() {
            let temp_dir = create_temp_dir();
            let lib_rs = temp_dir.path().join("src").join("lib.rs");
            let formatter_rs = temp_dir.path().join("src").join("formatter.rs");
            create_file(
                &lib_rs,
                r#"
    mod formatter;
    pub use formatter::Format;
    "#,
            );
            create_file(
                &formatter_rs,
                r#"
    pub enum Format {
        Plain,
        Rich,
    }
    "#,
            );
            let mut parser = setup_parser();

            let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

            let formatter = namespaces.iter().find(|n| n.name == "formatter").unwrap();
            assert_eq!(formatter.definitions.len(), 1);
            assert!(formatter.definitions.iter().any(|s| s.name == "Format"));
            assert!(!formatter.is_public);
            // But its symbols should be reexported in the text module
            let text = namespaces.iter().find(|n| n.name == "").unwrap();
            assert_eq!(text.references.len(), 1);
            assert!(text
                .references
                .iter()
                .any(|path| path == "formatter::Format"));
        }

        #[test]
        fn indirect_symbol_reexport() {
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
            assert_eq!(root.references[0], "formatting::Format");

            let formatting = namespaces.iter().find(|n| n.name == "formatting").unwrap();
            assert_eq!(formatting.definitions.len(), 0);
            assert_eq!(formatting.references.len(), 1);
            assert_eq!(formatting.references[0], "formatting::format::Format");

            let format = namespaces
                .iter()
                .find(|n| n.name == "formatting::format")
                .unwrap();
            assert_eq!(format.definitions.len(), 1);
            assert_eq!(format.references.len(), 0);
            assert_eq!(format.definitions[0].name, "Format");
        }
    }

    mod doc_comments {
        use super::*;

        #[test]
        fn file_with_doc_comment() {
            let temp_dir = create_temp_dir();
            let lib_rs = temp_dir.path().join("src").join("lib.rs");
            create_file(
                &lib_rs,
                r#"//! This is a file-level doc comment.
//! It can span multiple lines.

pub struct Test {}
"#,
            );

            let mut parser = setup_parser();
            let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

            assert_eq!(namespaces.len(), 1);
            assert_eq!(namespaces[0].name, "");
            assert_eq!(
                namespaces[0].doc_comment,
                Some(
                    "//! This is a file-level doc comment.\n//! It can span multiple lines.\n"
                        .to_string()
                )
            );
        }

        #[test]
        fn module_with_inner_doc_comment() {
            let temp_dir = create_temp_dir();
            let lib_rs = temp_dir.path().join("src").join("lib.rs");
            create_file(
                &lib_rs,
                r#"
pub mod inner {
    //! This is the inner doc comment
    //! It spans multiple lines

    pub fn nested_function() -> String {}
}
"#,
            );

            let mut parser = setup_parser();
            let namespaces = collect_symbols(&lib_rs, &mut parser).unwrap();

            assert_eq!(namespaces.len(), 2);
            let inner_namespace = namespaces.iter().find(|n| n.name == "inner").unwrap();
            assert_eq!(
                inner_namespace.doc_comment,
                Some(
                    "//! This is the inner doc comment\n//! It spans multiple lines\n".to_string()
                )
            );
        }
    }
}
