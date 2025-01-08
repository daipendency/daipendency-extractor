use crate::error::LaibraryError;
use crate::types::{Namespace, SourceFile, Symbol};
use std::path::Path;
use tree_sitter::Node;

pub fn extract_modules_from_file(source: &SourceFile) -> Result<Vec<Namespace>, LaibraryError> {
    let module_path = determine_module_path(&source.path)?;
    let module_path = module_path.unwrap_or_default();

    extract_modules_from_module(source.tree.root_node(), &source.content, module_path)
}

fn extract_modules_from_module(
    module_node: Node,
    source_code: &str,
    module_path: String,
) -> Result<Vec<Namespace>, LaibraryError> {
    let mut modules = Vec::new();
    let mut symbols = Vec::new();
    let mut cursor = module_node.walk();

    for child in module_node.children(&mut cursor) {
        if !is_public(&child) {
            continue;
        }

        match child.kind() {
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "macro_definition" => {
                let name = extract_name(&child, source_code)?;
                let mut source_code_with_docs = String::new();

                if let Some(doc_comment) = extract_outer_doc_comments(&child, source_code)? {
                    source_code_with_docs.push_str(&doc_comment);
                }

                source_code_with_docs.push_str(&get_symbol_source_code(child, source_code)?);

                symbols.push(Symbol {
                    name,
                    source_code: source_code_with_docs,
                });
            }
            "mod_item" => {
                let mod_name = extract_name(&child, source_code)?;
                let new_module_path = format!("{}::{}", module_path, mod_name);

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

fn determine_module_path(file_path: &Path) -> Result<Option<String>, LaibraryError> {
    let file_name = file_path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| LaibraryError::Parse("Invalid file name".to_string()))?;

    if file_name == "lib.rs" {
        return Ok(None);
    }

    let mut path_components = Vec::new();

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

fn extract_outer_doc_comments(
    node: &Node,
    source_code: &str,
) -> Result<Option<String>, LaibraryError> {
    if let Some(sibling) = node.prev_sibling() {
        let mut cursor = sibling.walk();
        let children: Vec<_> = sibling.children(&mut cursor).collect();

        let is_outer_doc_comment = children
            .iter()
            .any(|c| c.kind() == "outer_doc_comment_marker");

        if !is_outer_doc_comment {
            return Ok(None);
        }

        match sibling.kind() {
            "block_comment" => {
                if children.iter().any(|child| child.kind() == "doc_comment") {
                    let text = sibling
                        .utf8_text(source_code.as_bytes())
                        .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                    Ok(Some(text.to_string() + "\n"))
                } else {
                    Ok(None)
                }
            }
            "line_comment" => {
                let mut current = Some(sibling);
                let mut doc_comments = Vec::new();

                while let Some(comment) = current {
                    let mut cursor = comment.walk();
                    let children: Vec<_> = comment.children(&mut cursor).collect();

                    let has_outer_doc = children
                        .iter()
                        .any(|c| c.kind() == "outer_doc_comment_marker");
                    let has_doc_comment =
                        children.iter().any(|child| child.kind() == "doc_comment");

                    if has_outer_doc && has_doc_comment {
                        let comment_text = comment
                            .utf8_text(source_code.as_bytes())
                            .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                        doc_comments.push(comment_text.to_string());
                    } else {
                        break;
                    }

                    current = comment.prev_sibling();
                    if let Some(next) = current {
                        if next.kind() != "line_comment" {
                            break;
                        }
                    }
                }

                if doc_comments.is_empty() {
                    return Ok(None);
                }

                Ok(Some(
                    doc_comments.into_iter().rev().collect::<Vec<_>>().join(""),
                ))
            }
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn is_public(node: &Node) -> bool {
    let mut cursor = node.walk();
    let mut children = node.children(&mut cursor);
    children.any(|child| child.kind() == "visibility_modifier")
}

fn extract_name(node: &Node, source_code: &str) -> Result<String, LaibraryError> {
    let mut cursor = node.walk();
    let mut children = node.children(&mut cursor);
    children
        .find(|child| matches!(child.kind(), "identifier" | "type_identifier"))
        .and_then(|child| {
            child
                .utf8_text(source_code.as_bytes())
                .map(|s| s.to_string())
                .ok()
        })
        .ok_or_else(|| LaibraryError::Parse("Failed to extract name".to_string()))
}

fn get_symbol_source_code(node: Node, source_code: &str) -> Result<String, LaibraryError> {
    match node.kind() {
        "function_item" => {
            let mut cursor = node.walk();
            let block_node = node
                .children(&mut cursor)
                .find(|n| n.kind() == "block")
                .unwrap();
            Ok(format!(
                "{};",
                &source_code[node.start_byte()..block_node.start_byte()].trim_end()
            ))
        }
        _ => node
            .utf8_text(source_code.as_bytes())
            .map(|s| s.to_string())
            .map_err(|e| LaibraryError::Parse(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysers::Analyser;
    use crate::languages::rust::analyser::RustAnalyser;
    use crate::types::Symbol;
    use helpers::*;
    use std::path::PathBuf;
    use tree_sitter::Parser;

    mod helpers {
        use super::*;

        pub fn create_source_file(path: &str, content: &str) -> SourceFile {
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

        pub fn get_namespace<'a>(modules: &'a [Namespace], name: &str) -> Option<&'a Namespace> {
            modules.iter().find(|m| m.name == name)
        }

        pub fn extract_symbol(source_code: &str, symbol_name: &str) -> Symbol {
            let source = create_source_file("src/lib.rs", source_code);
            let modules = extract_modules_from_file(&source).unwrap();
            let root_module = get_namespace(&modules, "").unwrap();
            root_module.get_symbol(symbol_name).unwrap().clone()
        }

        pub fn extract_modules_from_source(path: &str, content: &str) -> Vec<Namespace> {
            let source = create_source_file(path, content);
            extract_modules_from_file(&source).unwrap()
        }
    }

    #[test]
    fn empty_source_file() {
        let source_code = "";

        let modules = extract_modules_from_source("src/empty.rs", source_code);

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].name, "empty");
        assert!(modules[0].symbols.is_empty());
    }

    #[test]
    fn function_body_is_omitted() {
        let source_code = r#"
pub fn test_function() -> i32 {
    return 42;
}
"#;

        let modules = extract_modules_from_source("src/lib.rs", source_code);

        assert_eq!(modules.len(), 1);
        let module = &modules[0];
        assert_eq!(module.symbols.len(), 1);
        assert_eq!(
            module.symbols[0].source_code,
            "pub fn test_function() -> i32;"
        );
    }

    mod visibility {
        use super::helpers::*;

        #[test]
        fn private_symbols() {
            let source_code = r#"
fn private_function() {}
pub fn public_function() -> () {}
"#;

            let modules = extract_modules_from_source("src/lib.rs", source_code);

            assert_eq!(modules.len(), 1);
            let module = &modules[0];
            assert_eq!(module.symbols.len(), 1);
            assert_eq!(module.symbols[0].name, "public_function");
        }

        #[test]
        fn crate_visible_symbols() {
            let source_code = r#"
pub(crate) fn crate_function() {}
"#;

            let modules = extract_modules_from_source("src/lib.rs", source_code);

            assert_eq!(modules.len(), 1);
            let module = &modules[0];
            assert_eq!(module.symbols.len(), 1);
            assert_eq!(module.symbols[0].name, "crate_function");
        }

        #[test]
        fn super_visible_symbols() {
            let source_code = r#"
pub(super) fn super_function() {}
"#;

            let modules = extract_modules_from_source("src/lib.rs", source_code);

            assert_eq!(modules.len(), 1);
            let module = &modules[0];
            assert_eq!(module.symbols.len(), 1);
            assert_eq!(module.symbols[0].name, "super_function");
        }
    }

    mod outer_doc_comments {
        use super::helpers::*;
        use assertables::assert_starts_with;

        #[test]
        fn no_doc_comments() {
            let source_code = r#"
pub struct Test {}
"#;

            let symbol = extract_symbol(source_code, "Test");
            assert_eq!(symbol.source_code, "pub struct Test {}");
        }

        #[test]
        fn single_line() {
            let source_code = r#"
/// A documented item
pub struct Test {}
"#;

            let symbol = extract_symbol(source_code, "Test");
            assert_eq!(
                symbol.source_code,
                "/// A documented item\npub struct Test {}"
            );
        }

        #[test]
        fn multiple_line() {
            let source_code = r#"
/// First line
/// Second line
pub struct Test {}
"#;

            let symbol = extract_symbol(source_code, "Test");
            assert_starts_with!(symbol.source_code, "/// First line\n/// Second line\n");
        }

        #[test]
        fn inner_doc_comments() {
            let source_code = r#"
/// Outer doc
//! Inner doc
pub struct Test {}
"#;

            let symbol = extract_symbol(source_code, "Test");
            assert_starts_with!(symbol.source_code, "pub struct Test");
        }

        #[test]
        fn regular_comments() {
            let source_code = r#"
/// Doc comment
// Regular comment
pub struct Test {}
"#;

            let symbol = extract_symbol(source_code, "Test");
            assert_starts_with!(symbol.source_code, "pub struct Test");
        }

        #[test]
        fn block_doc_comments() {
            let source_code = r#"
/** A block doc comment
 * with multiple lines
 * and some indentation
 */
pub struct Test {}
"#;

            let symbol = extract_symbol(source_code, "Test");
            assert_starts_with!(symbol.source_code, "/** A block doc comment\n * with multiple lines\n * and some indentation\n */\npub struct Test");
        }

        #[test]
        fn file_level_doc_comments() {
            let source_code = r#"
//! File-level documentation
//! More file-level docs

/// This is the struct's doc
pub struct Test {}
"#;

            let symbol = extract_symbol(source_code, "Test");
            assert_starts_with!(symbol.source_code, "/// This is the struct's doc\n");
        }

        #[test]
        fn preceding_symbol() {
            let source_code = r#"
/// First struct's doc
pub struct FirstStruct {}

/// Second struct's doc
pub struct SecondStruct {}
"#;

            let symbol = extract_symbol(source_code, "SecondStruct");
            assert_starts_with!(symbol.source_code, "/// Second struct's doc\n");
        }

        #[test]
        fn block_comment_preceded_by_line_comment() {
            let source_code = r#"
/// This line should be ignored
/** This block comment
 * should be returned
 */
pub struct Test {}
"#;

            let symbol = extract_symbol(source_code, "Test");
            assert_starts_with!(
                symbol.source_code,
                "/** This block comment\n * should be returned\n */\n"
            );
        }

        #[test]
        fn function_with_doc_comment() {
            let source_code = r#"
/// A documented function
pub fn test_function() -> i32 {
    42
}
"#;

            let symbol = extract_symbol(source_code, "test_function");
            assert_starts_with!(symbol.source_code, "/// A documented function\n");
        }

        #[test]
        fn struct_with_doc_comment() {
            let source_code = r#"
/// A documented struct
pub struct TestStruct {
    field: i32
}
"#;

            let symbol = extract_symbol(source_code, "TestStruct");
            assert_starts_with!(symbol.source_code, "/// A documented struct\n");
        }

        #[test]
        fn enum_with_doc_comment() {
            let source_code = r#"
/// A documented enum
pub enum TestEnum {
    A,
    B
}
"#;

            let symbol = extract_symbol(source_code, "TestEnum");
            assert_starts_with!(symbol.source_code, "/// A documented enum\n");
        }

        #[test]
        fn trait_with_doc_comment() {
            let source_code = r#"
/// A documented trait
pub trait TestTrait {
    fn test_method(&self);
}
"#;

            let symbol = extract_symbol(source_code, "TestTrait");
            assert_starts_with!(symbol.source_code, "/// A documented trait\n");
        }
    }

    mod module_path {
        use super::helpers::*;

        #[test]
        fn lib_rs_has_no_module_path() {
            let source_code = "";

            let modules = extract_modules_from_source("src/lib.rs", source_code);

            assert_eq!(modules.len(), 1);
            assert_eq!(modules[0].name, "");
        }

        #[test]
        fn direct_module_file_has_single_segment_path() {
            let source_code = "";

            let modules = extract_modules_from_source("src/text.rs", source_code);

            assert_eq!(modules.len(), 1);
            assert_eq!(modules[0].name, "text");
        }

        #[test]
        fn mod_rs_has_directory_name_path() {
            let source_code = "";

            let modules = extract_modules_from_source("src/text/mod.rs", source_code);

            assert_eq!(modules.len(), 1);
            assert_eq!(modules[0].name, "text");
        }

        #[test]
        fn nested_module_has_multi_segment_path() {
            let source_code = "";

            let modules = extract_modules_from_source("src/text/formatter.rs", source_code);

            assert_eq!(modules.len(), 1);
            assert_eq!(modules[0].name, "text::formatter");
        }
    }

    mod inner_modules {
        use super::helpers::*;

        #[test]
        fn public_modules() {
            let source_code = r#"
pub mod inner {
    pub fn nested_function() -> String {}
}
"#;

            let modules = extract_modules_from_source("src/text/mod.rs", source_code);

            let inner_module = modules.iter().find(|m| m.name == "text::inner").unwrap();
            assert_eq!(inner_module.symbols.len(), 1);
            assert_eq!(inner_module.symbols[0].name, "nested_function");
        }

        #[test]
        fn private_modules() {
            let source_code = r#"
mod private {
    pub fn private_function() -> String {}
}
"#;

            let modules = extract_modules_from_source("src/text/mod.rs", source_code);

            assert_eq!(modules.len(), 1);
            assert_eq!(modules[0].name, "text");
        }

        #[test]
        fn empty_modules() {
            let source_code = r#"
pub mod empty {}
"#;

            let modules = extract_modules_from_source("src/text/mod.rs", source_code);

            assert_eq!(modules.len(), 2);

            let empty_module = modules.iter().find(|m| m.name == "text::empty").unwrap();

            assert!(empty_module.symbols.is_empty());
        }

        #[test]
        fn inner_module_symbols() {
            let source_code = r#"
pub mod inner {
    pub struct InnerStruct {}

    pub mod deeper {
        pub enum DeeperEnum {
            A, B
        }
    }
}
"#;

            let modules = extract_modules_from_source("src/text/mod.rs", source_code);

            let inner_module = modules
                .iter()
                .find(|m| m.name == "text::inner")
                .expect("inner module should exist");

            let deeper_module = modules
                .iter()
                .find(|m| m.name == "text::inner::deeper")
                .expect("deeper module should exist");

            assert_eq!(inner_module.symbols.len(), 1);
            assert!(inner_module.symbols.iter().any(|s| s.name == "InnerStruct"));

            assert_eq!(deeper_module.symbols.len(), 1);
            assert!(deeper_module.symbols.iter().any(|s| s.name == "DeeperEnum"));
        }

        #[test]
        fn module_declarations() {
            let source_code = r#"
pub mod other;
"#;

            let modules = extract_modules_from_source("src/text/mod.rs", source_code);

            assert_eq!(modules.len(), 1);

            let text_module = modules.iter().find(|m| m.name == "text").unwrap();

            assert_eq!(text_module.symbols.len(), 1);
            let symbol = &text_module.symbols[0];
            assert_eq!(symbol.name, "other");
            assert_eq!(symbol.source_code, "pub mod other;");
        }
    }
}
