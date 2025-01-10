use crate::error::LaibraryError;
use crate::languages::rust::types::RustSymbol;
use crate::types::Symbol;
use tree_sitter::{Node, Parser};

pub fn parse_rust_file(
    content: &str,
    parser: &mut Parser,
) -> Result<Vec<RustSymbol>, LaibraryError> {
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| LaibraryError::Parse("Failed to parse source file".to_string()))?;

    extract_symbols_from_module(tree.root_node(), content)
}

fn extract_symbols_from_module(
    module_node: Node,
    source_code: &str,
) -> Result<Vec<RustSymbol>, LaibraryError> {
    let mut symbols = Vec::new();
    let mut cursor = module_node.walk();

    for child in module_node.children(&mut cursor) {
        if !is_public(&child) {
            continue;
        }

        match child.kind() {
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "macro_definition" => {
                let name = extract_name(&child, source_code)?;
                symbols.push(RustSymbol::Symbol(Symbol {
                    name,
                    source_code: get_symbol_source_code(child, source_code)?,
                }));
            }
            "mod_item" => {
                let inner_mod_name = extract_name(&child, source_code)?;

                let mut mod_cursor = child.walk();
                let mod_children: Vec<_> = child.children(&mut mod_cursor).collect();
                if let Some(declaration_node) = mod_children
                    .iter()
                    .find(|mod_child| mod_child.kind() == "declaration_list")
                {
                    let inner_mod_symbols =
                        extract_symbols_from_module(*declaration_node, source_code)?;

                    symbols.push(RustSymbol::Module {
                        name: inner_mod_name,
                        content: inner_mod_symbols,
                    });
                } else {
                    // It's a module re-export, not a module block
                    symbols.push(RustSymbol::Symbol(Symbol {
                        name: inner_mod_name,
                        source_code: child
                            .utf8_text(source_code.as_bytes())
                            .map_err(|e| LaibraryError::Parse(e.to_string()))?
                            .to_string(),
                    }));
                }
            }
            _ => {}
        }
    }

    Ok(symbols)
}

fn extract_outer_doc_comments(
    node: &Node,
    source_code: &str,
) -> Result<Option<String>, LaibraryError> {
    let sibling = match node.prev_sibling() {
        Some(s) => s,
        None => return Ok(None),
    };

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
                let has_doc_comment = children.iter().any(|child| child.kind() == "doc_comment");

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
}

fn is_public(node: &Node) -> bool {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    children
        .iter()
        .any(|child| child.kind() == "visibility_modifier")
}

fn extract_name(node: &Node, source_code: &str) -> Result<String, LaibraryError> {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    children
        .iter()
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
    let mut source_code_with_docs = String::new();

    if let Some(doc_comment) = extract_outer_doc_comments(&node, source_code)? {
        source_code_with_docs.push_str(&doc_comment);
    }

    let symbol_source = match node.kind() {
        "function_item" | "function_signature_item" => {
            let mut cursor = node.walk();
            let block_node = node
                .children(&mut cursor)
                .find(|n| n.kind() == "block")
                .ok_or_else(|| LaibraryError::Parse("Failed to find function block".to_string()))?;
            format!(
                "{};",
                &source_code[node.start_byte()..block_node.start_byte()].trim_end()
            )
        }
        "trait_item" => {
            let mut cursor = node.walk();
            let declaration_list = node
                .children(&mut cursor)
                .find(|n| n.kind() == "declaration_list")
                .ok_or_else(|| {
                    LaibraryError::Parse("Failed to find trait declaration list".to_string())
                })?;

            let mut trait_source = String::new();
            trait_source.push_str(&source_code[node.start_byte()..declaration_list.start_byte()]);
            trait_source.push_str(" {\n");

            let mut method_cursor = declaration_list.walk();
            for method in declaration_list.children(&mut method_cursor) {
                if method.kind() == "function_item" {
                    let method_source = get_symbol_source_code(method, source_code)?;
                    for line in method_source.lines() {
                        trait_source.push_str("    ");
                        trait_source.push_str(line);
                        trait_source.push('\n');
                    }
                }
            }

            trait_source.push('}');
            trait_source
        }
        _ => node
            .utf8_text(source_code.as_bytes())
            .map(|s| s.to_string())
            .map_err(|e| LaibraryError::Parse(e.to_string()))?,
    };

    source_code_with_docs.push_str(&symbol_source);
    Ok(source_code_with_docs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::test_helpers::{get_inner_module, get_rust_symbol, setup_parser};
    use assertables::{assert_contains, assert_starts_with};

    #[test]
    fn empty_source_file() {
        let source_code = "";
        let mut parser = setup_parser();

        let symbols = parse_rust_file(source_code, &mut parser).unwrap();

        assert!(symbols.is_empty());
    }

    #[test]
    fn invalid_syntax() {
        let source_code = "fn main() { let x = 1; let y = 2; let z = x + y; }";
        let mut parser = setup_parser();

        let result = parse_rust_file(source_code, &mut parser);

        assert!(result.is_ok());
    }

    mod function_body {
        use super::*;

        #[test]
        fn function_declaration() {
            let source_code = r#"
pub fn test_function() -> i32 {
    return 42;
}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "test_function").unwrap();
            assert_eq!(symbol.source_code, "pub fn test_function() -> i32;");
        }

        #[test]
        fn trait_method_declaration() {
            let source_code = r#"
pub trait TestTrait {
    pub fn test_method(&self) -> i32 {
        42
    }
}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "TestTrait").unwrap();
            assert_contains!(symbol.source_code, "pub fn test_method(&self) -> i32;");
        }
    }

    mod visibility {
        use super::*;

        #[test]
        fn private_symbols() {
            let source_code = r#"
fn private_function() {}
pub fn public_function() -> () {}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(get_rust_symbol(&symbols, "private_function").is_none());
        }

        #[test]
        fn crate_visible_symbols() {
            let source_code = r#"
pub(crate) fn crate_function() {}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(get_rust_symbol(&symbols, "crate_function").is_some());
        }

        #[test]
        fn super_visible_symbols() {
            let source_code = r#"
pub(super) fn super_function() {}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(get_rust_symbol(&symbols, "super_function").is_some());
        }
    }

    mod outer_doc_comments {
        use super::*;

        #[test]
        fn no_doc_comments() {
            let source_code = r#"
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "Test").unwrap();
            assert_eq!(symbol.source_code, "pub struct Test {}");
        }

        #[test]
        fn single_line() {
            let source_code = r#"
/// A documented item
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "Test").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "Test").unwrap();
            assert_starts_with!(symbol.source_code, "/// First line\n/// Second line\n");
        }

        #[test]
        fn inner_doc_comments() {
            let source_code = r#"
/// Outer doc
//! Inner doc
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "Test").unwrap();
            assert_starts_with!(symbol.source_code, "pub struct Test");
        }

        #[test]
        fn regular_comments() {
            let source_code = r#"
/// Doc comment
// Regular comment
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "Test").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "Test").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "Test").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "SecondStruct").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "Test").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "test_function").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "TestStruct").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "TestEnum").unwrap();
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "TestTrait").unwrap();
            assert_starts_with!(symbol.source_code, "/// A documented trait\n");
        }

        #[test]
        fn trait_method_doc_comments() {
            let source_code = r#"
pub trait TestTrait {
    /// A documented method
    pub fn test_method(&self) -> i32 {
        42
    }
}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "TestTrait").unwrap();
            assert_contains!(
                symbol.source_code,
                "/// A documented method\n    pub fn test_method(&self) -> i32;"
            );
        }
    }

    mod inner_modules {
        use super::*;

        #[test]
        fn public_modules() {
            let source_code = r#"
pub mod inner {
    pub fn nested_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let inner_content = get_inner_module("inner", &symbols).unwrap();
            let symbol = get_rust_symbol(inner_content, "nested_function").unwrap();
            assert_eq!(symbol.name, "nested_function");
        }

        #[test]
        fn private_modules() {
            let source_code = r#"
mod private {
    pub fn private_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(symbols.is_empty(), "Private modules should be ignored");
        }

        #[test]
        fn empty_modules() {
            let source_code = r#"
pub mod empty {}
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let empty_content = get_inner_module("empty", &symbols).unwrap();
            assert_eq!(symbols.len(), 1);
            assert!(empty_content.is_empty());
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
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(symbols.len(), 1);
            let inner_content =
                get_inner_module("inner", &symbols).expect("Should find inner module");
            assert_eq!(inner_content.len(), 2);
            let inner_struct =
                get_rust_symbol(inner_content, "InnerStruct").expect("Should find InnerStruct");
            assert_eq!(inner_struct.name, "InnerStruct");
            let deeper_content =
                get_inner_module("deeper", inner_content).expect("Should find deeper module");
            assert_eq!(deeper_content.len(), 1);
            let deeper_enum =
                get_rust_symbol(deeper_content, "DeeperEnum").expect("Should find DeeperEnum");
            assert_eq!(deeper_enum.name, "DeeperEnum");
        }

        #[test]
        fn module_declarations() {
            let source_code = r#"
pub mod other;
"#;
            let mut parser = setup_parser();

            let symbols = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&symbols, "other").unwrap();
            assert_eq!(symbol.name, "other");
            assert_eq!(symbol.source_code, "pub mod other;");
        }
    }
}
