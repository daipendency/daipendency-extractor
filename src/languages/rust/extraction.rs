use crate::error::LaibraryError;
use crate::types::{Namespace, SourceFile, Symbol};
use std::path::Path;
use tree_sitter::Node;

fn extract_modules_from_module<'a>(
    module_node: Node<'a>,
    source_code: &str,
    module_path: String,
) -> Result<Vec<Namespace<'a>>, LaibraryError> {
    let mut modules = Vec::new();
    let mut symbols = Vec::new();
    let mut cursor = module_node.walk();

    for child in module_node.children(&mut cursor) {
        if !is_public(&child, source_code) {
            continue;
        }

        match child.kind() {
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "macro_definition" => {
                if let Some(name) = extract_name(&child, source_code) {
                    symbols.push(Symbol {
                        name,
                        node: child,
                        source_code: child
                            .utf8_text(source_code.as_bytes())
                            .map_err(|e| LaibraryError::Parse(e.to_string()))?
                            .to_string(),
                    });
                }
            }
            "mod_item" => {
                let mod_name = extract_name(&child, source_code)
                    .ok_or_else(|| LaibraryError::Parse("Invalid module name".to_string()))?;
                let new_module_path = if module_path.is_empty() {
                    mod_name.clone()
                } else {
                    format!("{}::{}", module_path, mod_name)
                };

                // Look for the declaration_list node
                let mut cursor = child.walk();
                let children: Vec<_> = child.children(&mut cursor).collect();
                if let Some(declaration_node) = children
                    .iter()
                    .find(|mod_child| mod_child.kind() == "declaration_list")
                {
                    let mut extracted = extract_modules_from_module(
                        *declaration_node,
                        source_code,
                        new_module_path,
                    )?;
                    modules.append(&mut extracted);
                } else {
                    // Add module declaration as a symbol
                    symbols.push(Symbol {
                        name: mod_name,
                        node: child,
                        source_code: child
                            .utf8_text(source_code.as_bytes())
                            .map_err(|e| LaibraryError::Parse(e.to_string()))?
                            .to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    modules.push(Namespace {
        name: module_path,
        symbols,
    });

    Ok(modules)
}

pub fn extract_modules(sources: &[SourceFile]) -> Result<Vec<Namespace<'_>>, LaibraryError> {
    let mut modules = Vec::new();

    for source in sources {
        let root_node = source.tree.root_node();
        let module_path = determine_module_path(&source.path)?;
        let module_path = module_path.unwrap_or_default();

        let mut source_modules =
            extract_modules_from_module(root_node, &source.content, module_path)?;
        modules.append(&mut source_modules);
    }

    Ok(modules)
}

fn determine_module_path(file_path: &Path) -> Result<Option<String>, LaibraryError> {
    let file_name = file_path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| LaibraryError::Parse("Invalid file name".to_string()))?;

    if file_name == "lib.rs" {
        return Ok(None);
    }

    let mut path_components = Vec::new();

    // Get all path components after "src"
    let mut found_src = false;
    for component in file_path.parent().unwrap_or(Path::new("")).components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                if found_src {
                    path_components.push(name_str.to_string());
                } else if name_str == "src" {
                    found_src = true;
                }
            }
        }
    }

    // Add the file name without extension if it's not mod.rs
    if file_name != "mod.rs" {
        if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
            path_components.push(stem.to_string());
        }
    }

    // If we found no components after src, this is a root file
    if path_components.is_empty() && file_name != "mod.rs" {
        Ok(None)
    } else {
        Ok(Some(path_components.join("::")))
    }
}

fn is_public(node: &Node, source_code: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                return text == "pub";
            }
        }
    }
    false
}

fn extract_name(node: &Node, source_code: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(child.kind(), "identifier" | "type_identifier") {
            if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                return Some(text.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysers::Analyser;
    use crate::languages::rust::analyser::RustAnalyser;
    use std::path::PathBuf;
    use tree_sitter::Parser;

    fn create_source_file(path: &str, content: &str) -> SourceFile {
        let mut parser = Parser::new();
        let analyser = RustAnalyser::new();
        parser
            .set_language(&analyser.get_parser_language())
            .unwrap();
        let tree = parser
            .parse(content, None)
            .expect("Failed to parse test source file");
        SourceFile {
            path: PathBuf::from(path),
            content: content.to_string(),
            tree,
        }
    }

    mod module_path {
        use super::*;

        #[test]
        fn lib_rs_has_no_module_path() {
            assert_eq!(
                determine_module_path(&PathBuf::from("src/lib.rs")).unwrap(),
                None
            );
        }

        #[test]
        fn direct_module_file_has_single_segment_path() {
            assert_eq!(
                determine_module_path(&PathBuf::from("src/text.rs")).unwrap(),
                Some("text".to_string())
            );
        }

        #[test]
        fn mod_rs_has_directory_name_path() {
            assert_eq!(
                determine_module_path(&PathBuf::from("src/text/mod.rs")).unwrap(),
                Some("text".to_string())
            );
        }

        #[test]
        fn nested_module_has_multi_segment_path() {
            assert_eq!(
                determine_module_path(&PathBuf::from("src/text/formatter.rs")).unwrap(),
                Some("text::formatter".to_string())
            );
        }
    }

    mod modules {
        use super::*;

        #[test]
        fn nested_modules_are_extracted() {
            let source = create_source_file(
                "src/text/mod.rs",
                r#"
pub mod inner {
    pub fn nested_function() -> String {}
}
pub fn outer_function() -> i32 {}
"#,
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();

            assert!(modules.iter().any(|m| m.name == "text"));
            assert!(modules.iter().any(|m| m.name == "text::inner"));

            let text_module = modules.iter().find(|m| m.name == "text").unwrap();
            assert_eq!(text_module.symbols.len(), 1);
            assert_eq!(text_module.symbols[0].name, "outer_function");

            let inner_module = modules.iter().find(|m| m.name == "text::inner").unwrap();
            assert_eq!(inner_module.symbols.len(), 1);
            assert_eq!(inner_module.symbols[0].name, "nested_function");
        }

        #[test]
        fn private_modules_are_ignored() {
            let source = create_source_file(
                "src/text/mod.rs",
                r#"
mod private {
    pub fn private_function() -> String {}
}
pub fn public_function() -> i32 {}
"#,
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();

            assert_eq!(modules.len(), 1);
            assert_eq!(modules[0].name, "text");
            assert_eq!(modules[0].symbols.len(), 1);
            assert_eq!(modules[0].symbols[0].name, "public_function");
        }

        #[test]
        fn empty_modules_are_included() {
            let source = create_source_file(
                "src/text/mod.rs",
                r#"
pub mod empty {}
"#,
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();

            assert_eq!(modules.len(), 2); // text and text::empty

            let text_module = modules.iter().find(|m| m.name == "text").unwrap();
            assert!(text_module.symbols.is_empty()); // The empty module is not a symbol

            let empty_module = modules.iter().find(|m| m.name == "text::empty").unwrap();
            assert!(empty_module.symbols.is_empty());
        }

        #[test]
        fn nested_module_symbols_are_extracted() {
            let source = create_source_file(
                "src/text/mod.rs",
                r#"
pub mod inner {
    pub struct InnerStruct {}
    pub fn inner_function() {}
    
    pub mod deeper {
        pub enum DeeperEnum {
            A, B
        }
    }
}
"#,
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();

            // Find the inner module
            let inner_module = modules
                .iter()
                .find(|m| m.name == "text::inner")
                .expect("inner module should exist");

            assert_eq!(
                inner_module.symbols.len(),
                2,
                "inner module should have two symbols"
            );
            assert!(
                inner_module.symbols.iter().any(|s| s.name == "InnerStruct"),
                "inner module should contain InnerStruct"
            );
            assert!(
                inner_module
                    .symbols
                    .iter()
                    .any(|s| s.name == "inner_function"),
                "inner module should contain inner_function"
            );

            // Find the deeper module
            let deeper_module = modules
                .iter()
                .find(|m| m.name == "text::inner::deeper")
                .expect("deeper module should exist");

            assert_eq!(
                deeper_module.symbols.len(),
                1,
                "deeper module should have one symbol"
            );
            assert!(
                deeper_module.symbols.iter().any(|s| s.name == "DeeperEnum"),
                "deeper module should contain DeeperEnum"
            );
        }

        #[test]
        fn module_symbols_are_extracted_with_declarations() {
            let source = create_source_file(
                "src/text/mod.rs",
                r#"
pub mod inner {
    pub struct InnerStruct {}
    pub fn inner_function() {}
}
"#,
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();

            // The module should appear exactly once
            let inner_modules: Vec<_> =
                modules.iter().filter(|m| m.name == "text::inner").collect();

            assert_eq!(
                inner_modules.len(),
                1,
                "inner module should appear exactly once"
            );

            let inner_module = &inner_modules[0];
            assert_eq!(
                inner_module.symbols.len(),
                2,
                "inner module should have exactly two symbols"
            );
        }

        #[test]
        fn module_declarations_are_added_as_symbols() {
            let source = create_source_file(
                "src/text/mod.rs",
                r#"
pub mod other;  // This should be added as a symbol
pub mod inner {  // This should be processed as a module
    pub fn inner_function() {}
}
"#,
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();

            assert_eq!(modules.len(), 2); // text and text::inner

            let text_module = modules.iter().find(|m| m.name == "text").unwrap();
            assert_eq!(text_module.symbols.len(), 1);
            assert_eq!(text_module.symbols[0].name, "other"); // The module declaration becomes a symbol

            let inner_module = modules.iter().find(|m| m.name == "text::inner").unwrap();
            assert_eq!(inner_module.symbols.len(), 1);
            assert_eq!(inner_module.symbols[0].name, "inner_function");
        }
    }

    mod visibility {
        use super::*;

        #[test]
        fn private_items_are_ignored() {
            let source = create_source_file(
                "src/text/mod.rs",
                r#"
struct PrivateStruct {}
fn private_function() {}
pub fn public_function() -> () {}
"#,
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();

            assert_eq!(modules.len(), 1);
            let module = &modules[0];
            assert_eq!(module.name, "text");
            assert_eq!(module.symbols.len(), 1);
            assert_eq!(module.symbols[0].name, "public_function");
        }

        #[test]
        fn lib_items_are_processed() {
            let source = create_source_file(
                "src/lib.rs",
                r#"
pub fn root_function() -> () {}
pub struct RootStruct {}

pub mod text {
    pub fn text_function() -> () {}
}
"#,
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();

            assert_eq!(modules.len(), 2);

            let root_module = modules.iter().find(|m| m.name.is_empty()).unwrap();
            assert_eq!(root_module.symbols.len(), 2);
            assert!(root_module
                .symbols
                .iter()
                .any(|s| s.name == "root_function"));
            assert!(root_module.symbols.iter().any(|s| s.name == "RootStruct"));

            let text_module = modules.iter().find(|m| m.name == "text").unwrap();
            assert_eq!(text_module.symbols.len(), 1);
            assert_eq!(text_module.symbols[0].name, "text_function");
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn empty_source_file() {
            let source = create_source_file("src/empty.rs", "");

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();
            assert_eq!(modules.len(), 1); // An empty file still creates a module
            assert_eq!(modules[0].name, "empty");
            assert!(modules[0].symbols.is_empty()); // But it has no symbols
        }

        #[test]
        fn whitespace_only_source() {
            let source = create_source_file("src/whitespace.rs", "  \n\t  \n");

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();
            assert_eq!(modules.len(), 1); // A whitespace-only file still creates a module
            assert_eq!(modules[0].name, "whitespace");
            assert!(modules[0].symbols.is_empty()); // But it has no symbols
        }

        #[test]
        fn comments_only_source() {
            let source = create_source_file(
                "src/comments.rs",
                "// Just a comment\n/* Another comment */",
            );

            let sources = vec![source];
            let modules = extract_modules(&sources).unwrap();
            assert_eq!(modules.len(), 1); // A file with only comments still creates a module
            assert_eq!(modules[0].name, "comments");
            assert!(modules[0].symbols.is_empty()); // But it has no symbols
        }
    }
}
