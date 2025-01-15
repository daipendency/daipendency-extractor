use super::types::{RustFile, RustSymbol};
use crate::error::LaibraryError;
use crate::types::Symbol;
use tree_sitter::{Node, Parser};

pub fn parse_rust_file(content: &str, parser: &mut Parser) -> Result<RustFile, LaibraryError> {
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| LaibraryError::Parse("Failed to parse source file".to_string()))?;

    let doc_comment = extract_inner_doc_comments(&tree.root_node(), content)?;
    let symbols = extract_symbols_from_module(tree.root_node(), content)?;
    Ok(RustFile {
        doc_comment,
        symbols,
    })
}

fn extract_symbols_from_module(
    module_node: Node,
    source_code: &str,
) -> Result<Vec<RustSymbol>, LaibraryError> {
    let mut symbols = Vec::new();
    let mut cursor = module_node.walk();

    for child in module_node.children(&mut cursor) {
        match child.kind() {
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "macro_definition" => {
                if !is_public(&child) {
                    continue;
                }
                let name = extract_name(&child, source_code)?;
                symbols.push(RustSymbol::Symbol {
                    symbol: Symbol {
                        name,
                        source_code: get_symbol_source_code(child, source_code)?,
                    },
                });
            }
            "use_declaration" => {
                if !is_public(&child) {
                    continue;
                }
                let mut cursor = child.walk();
                let children: Vec<_> = child.children(&mut cursor).collect();

                if let Some(scoped) = children.iter().find(|c| c.kind() == "scoped_identifier") {
                    let mut scoped_cursor = scoped.walk();
                    let scoped_children: Vec<_> = scoped.children(&mut scoped_cursor).collect();

                    let mut path_parts = Vec::new();
                    for scoped_child in &scoped_children {
                        let text = scoped_child
                            .utf8_text(source_code.as_bytes())
                            .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                        path_parts.push(text);
                    }

                    if let Some(name) = path_parts.last() {
                        let path = path_parts[..path_parts.len() - 1].join("");
                        symbols.push(RustSymbol::SymbolReexport {
                            name: name.to_string(),
                            source_path: format!("{}{}", path, name),
                        });
                    }
                } else if let Some(scoped_list) =
                    children.iter().find(|c| c.kind() == "scoped_use_list")
                {
                    let mut scoped_cursor = scoped_list.walk();
                    let scoped_children: Vec<_> =
                        scoped_list.children(&mut scoped_cursor).collect();

                    let path_prefix = if let Some(path) = scoped_children.first() {
                        path.utf8_text(source_code.as_bytes())
                            .map_err(|e| LaibraryError::Parse(e.to_string()))?
                            .to_string()
                    } else {
                        String::new()
                    };

                    if let Some(use_list) = scoped_children.iter().find(|c| c.kind() == "use_list")
                    {
                        let mut list_cursor = use_list.walk();
                        let list_items: Vec<_> = use_list.children(&mut list_cursor).collect();
                        for list_item in list_items {
                            if list_item.kind() == "identifier" {
                                let name = list_item
                                    .utf8_text(source_code.as_bytes())
                                    .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                                symbols.push(RustSymbol::SymbolReexport {
                                    name: name.to_string(),
                                    source_path: format!("{}::{}", path_prefix, name),
                                });
                            }
                        }
                    }
                } else if let Some(tree) = children.iter().find(|c| c.kind() == "use_tree") {
                    let mut tree_cursor = tree.walk();
                    let tree_children: Vec<_> = tree.children(&mut tree_cursor).collect();

                    if let Some(list) = tree_children.iter().find(|c| c.kind() == "use_tree_list") {
                        let mut list_cursor = list.walk();
                        for list_item in list.children(&mut list_cursor) {
                            if list_item.kind() == "use_tree" {
                                if let Some(name_node) = list_item.child_by_field_name("name") {
                                    let name = extract_name(&name_node, source_code)?;
                                    let path_prefix =
                                        if let Some(path) = tree.child_by_field_name("path") {
                                            let prefix = path
                                                .utf8_text(source_code.as_bytes())
                                                .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                                            format!("{}::", prefix)
                                        } else {
                                            String::new()
                                        };
                                    symbols.push(RustSymbol::SymbolReexport {
                                        name: name.clone(),
                                        source_path: format!("{}{}", path_prefix, name),
                                    });
                                }
                            }
                        }
                    } else if let Some(name_node) = tree.child_by_field_name("name") {
                        let name = extract_name(&name_node, source_code)?;
                        let path_prefix = if let Some(path) = tree.child_by_field_name("path") {
                            let prefix = path
                                .utf8_text(source_code.as_bytes())
                                .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                            format!("{}::", prefix)
                        } else {
                            String::new()
                        };
                        symbols.push(RustSymbol::SymbolReexport {
                            name: name.clone(),
                            source_path: format!("{}{}", path_prefix, name),
                        });
                    }
                }
            }
            "mod_item" => {
                let inner_mod_name = extract_name(&child, source_code)?;
                let is_public = is_public(&child);

                let mut cursor = child.walk();
                let children: Vec<_> = child.children(&mut cursor).collect();
                if let Some(declaration_list) =
                    children.iter().find(|n| n.kind() == "declaration_list")
                {
                    // This is an inline module with content
                    let doc_comment = extract_inner_doc_comments(declaration_list, source_code)?;
                    let inner_mod_symbols =
                        extract_symbols_from_module(*declaration_list, source_code)?;

                    if is_public {
                        symbols.push(RustSymbol::Module {
                            name: inner_mod_name,
                            content: inner_mod_symbols,
                            doc_comment,
                        });
                    }
                } else {
                    // This is a module declaration (mod foo;)
                    symbols.push(RustSymbol::ModuleDeclaration {
                        name: inner_mod_name,
                        is_public,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(symbols)
}

fn extract_attributes(node: &Node, source_code: &str) -> Result<Vec<String>, LaibraryError> {
    let mut current = node.prev_sibling();
    let mut items = Vec::new();

    while let Some(sibling) = current {
        if sibling.kind() != "attribute_item" {
            break;
        }

        let text = sibling
            .utf8_text(source_code.as_bytes())
            .map_err(|e| LaibraryError::Parse(e.to_string()))?;
        items.push(text.to_string());

        current = sibling.prev_sibling();
    }

    items.reverse();
    Ok(items)
}

fn extract_outer_doc_comments(
    node: &Node,
    source_code: &str,
) -> Result<Option<String>, LaibraryError> {
    let mut current = node.prev_sibling();
    let mut items = Vec::new();

    while let Some(sibling) = current {
        match sibling.kind() {
            "line_comment" => {
                let mut cursor = sibling.walk();
                let children: Vec<_> = sibling.children(&mut cursor).collect();

                let has_outer_doc = children
                    .iter()
                    .any(|c| c.kind() == "outer_doc_comment_marker");
                let has_doc_comment = children.iter().any(|child| child.kind() == "doc_comment");

                if has_outer_doc && has_doc_comment {
                    let comment_text = sibling
                        .utf8_text(source_code.as_bytes())
                        .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                    items.push(comment_text.to_string());
                } else {
                    break;
                }
            }
            "block_comment" => {
                let mut cursor = sibling.walk();
                let children: Vec<_> = sibling.children(&mut cursor).collect();

                if children
                    .iter()
                    .any(|c| c.kind() == "outer_doc_comment_marker")
                    && children.iter().any(|child| child.kind() == "doc_comment")
                {
                    let text = sibling
                        .utf8_text(source_code.as_bytes())
                        .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                    items.push(text.to_string() + "\n");
                    break;
                } else {
                    break;
                }
            }
            "attribute_item" => {
                // Ignore attributes as they are handled separately
            }
            _ => break,
        }

        current = sibling.prev_sibling();
    }

    if items.is_empty() {
        Ok(None)
    } else {
        Ok(Some(items.into_iter().rev().collect()))
    }
}

fn extract_inner_doc_comments(
    node: &Node,
    source_code: &str,
) -> Result<Option<String>, LaibraryError> {
    let mut doc_comment = String::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "line_comment" {
            let mut comment_cursor = child.walk();
            let has_inner_doc = child
                .children(&mut comment_cursor)
                .any(|c| c.kind() == "inner_doc_comment_marker");
            if has_inner_doc {
                let comment_text = child
                    .utf8_text(source_code.as_bytes())
                    .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                doc_comment.push_str(comment_text);
            } else {
                break;
            }
        } else if !is_block_delimiter(&child) {
            break;
        }
    }
    Ok(if doc_comment.is_empty() {
        None
    } else {
        Some(doc_comment)
    })
}

fn is_block_delimiter(node: &Node) -> bool {
    matches!(node.kind(), "{" | "}")
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

    let attributes = extract_attributes(&node, source_code)?;
    if !attributes.is_empty() {
        let attributes_str = format!("{}\n", attributes.join("\n"));
        source_code_with_docs.push_str(&attributes_str);
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
    use crate::languages::rust::test_helpers::setup_parser;
    use assertables::{assert_contains, assert_starts_with};

    fn get_inner_module<'a>(path: &str, symbols: &'a [RustSymbol]) -> Option<&'a [RustSymbol]> {
        let parts: Vec<&str> = path.split("::").collect();
        let mut current_symbols = symbols;

        for part in parts {
            match current_symbols.iter().find_map(|symbol| {
                if let RustSymbol::Module { name, content, .. } = symbol {
                    if name == part {
                        Some(content)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                Some(next_symbols) => current_symbols = next_symbols,
                None => return None,
            }
        }

        Some(current_symbols)
    }

    fn get_rust_symbol<'a>(symbols: &'a [RustSymbol], name: &str) -> Option<&'a Symbol> {
        symbols.iter().find_map(|s| {
            if let RustSymbol::Symbol { symbol, .. } = s {
                if symbol.name == name {
                    Some(symbol)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    #[test]
    fn empty_source_file() {
        let source_code = "";
        let mut parser = setup_parser();

        let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

        assert!(rust_file.symbols.is_empty());
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "test_function").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "TestTrait").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(get_rust_symbol(&rust_file.symbols, "private_function").is_none());
        }

        #[test]
        fn crate_visible_symbols() {
            let source_code = r#"
pub(crate) fn crate_function() {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(get_rust_symbol(&rust_file.symbols, "crate_function").is_some());
        }

        #[test]
        fn super_visible_symbols() {
            let source_code = r#"
pub(super) fn super_function() {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(get_rust_symbol(&rust_file.symbols, "super_function").is_some());
        }
    }

    mod file_doc_comments {
        use super::*;

        #[test]
        fn no_doc_comment() {
            let source_code = r#"
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(rust_file.doc_comment.is_none());
        }

        #[test]
        fn single_line_doc_comment() {
            let source_code = r#"
//! This is a file-level doc comment
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(
                rust_file.doc_comment,
                Some("//! This is a file-level doc comment\n".to_string())
            );
        }

        #[test]
        fn multiline_doc_comment() {
            let source_code = r#"
//! This is a file-level doc comment
//! It spans multiple lines

pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(
                rust_file.doc_comment,
                Some(
                    "//! This is a file-level doc comment\n//! It spans multiple lines\n"
                        .to_string()
                )
            );
        }

        #[test]
        fn regular_comment_not_doc_comment() {
            let source_code = r#"
// This is a regular comment
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(rust_file.doc_comment.is_none());
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
            assert_eq!(symbol.source_code, "pub struct Test {}");
        }

        #[test]
        fn single_line() {
            let source_code = r#"
/// A documented item
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
            assert_starts_with!(symbol.source_code, "pub struct Test");
        }

        #[test]
        fn block_doc_comments() {
            let source_code = r#"
/** A block doc comment
 * with multiple lines
 */
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
            assert_starts_with!(
                symbol.source_code,
                "/** A block doc comment\n * with multiple lines\n */\npub struct Test"
            );
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "SecondStruct").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "test_function").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "TestStruct").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "TestEnum").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "TestTrait").unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "TestTrait").unwrap();
            assert_contains!(
                symbol.source_code,
                "/// A documented method\n    pub fn test_method(&self) -> i32;"
            );
        }

        #[test]
        fn doc_comment_with_macro() {
            let source_code = r#"
/// The doc comment
#[derive(Debug)]
pub enum Foo {
    Bar,
    Baz,
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Foo").unwrap();
            assert_starts_with!(
                symbol.source_code,
                "/// The doc comment\n#[derive(Debug)]\npub enum Foo"
            );
        }

        #[test]
        fn doc_comment_with_multiple_attributes() {
            let source_code = r#"
/// The doc comment
#[derive(Debug)]
#[serde(rename = "foo")]
pub enum Foo {
    Bar,
    Baz,
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Foo").unwrap();
            assert_starts_with!(
                symbol.source_code,
                "/// The doc comment\n#[derive(Debug)]\n#[serde(rename = \"foo\")]\npub enum Foo"
            );
        }

        #[test]
        fn macro_without_doc_comment() {
            let source_code = r#"
#[derive(Debug)]
pub enum Foo {
    Bar,
    Baz,
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "Foo").unwrap();
            assert_starts_with!(symbol.source_code, "#[derive(Debug)]\npub enum Foo");
        }
    }

    mod inner_modules {
        use super::*;

        #[test]
        fn module_without_inner_doc_comment() {
            let source_code = r#"
pub mod inner {
    pub fn nested_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let inner_module = match &rust_file.symbols[0] {
                RustSymbol::Module { name, content, .. } => {
                    assert_eq!(name, "inner");
                    content
                }
                _ => panic!("Expected Module variant"),
            };
            let symbol = get_rust_symbol(inner_module, "nested_function").unwrap();
            assert_eq!(symbol.name, "nested_function");
        }

        #[test]
        fn module_with_inner_doc_comment() {
            let source_code = r#"
pub mod inner {
    //! This is the inner doc comment
    //! It spans multiple lines

    pub fn nested_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let inner_module = match &rust_file.symbols[0] {
                RustSymbol::Module {
                    name,
                    content,
                    doc_comment,
                } => {
                    assert_eq!(name, "inner");
                    assert_eq!(
                        doc_comment,
                        &Some(
                            "//! This is the inner doc comment\n//! It spans multiple lines\n"
                                .to_string()
                        )
                    );
                    content
                }
                _ => panic!("Expected Module variant"),
            };
            let symbol = get_rust_symbol(inner_module, "nested_function").unwrap();
            assert_eq!(symbol.name, "nested_function");
        }

        #[test]
        fn public_modules() {
            let source_code = r#"
pub mod inner {
    pub fn nested_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let inner_content = get_inner_module("inner", &rust_file.symbols).unwrap();
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(
                rust_file.symbols.is_empty(),
                "Private modules should be ignored"
            );
        }

        #[test]
        fn empty_modules() {
            let source_code = r#"
pub mod empty {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let empty_content = get_inner_module("empty", &rust_file.symbols).unwrap();
            assert_eq!(rust_file.symbols.len(), 1);
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(rust_file.symbols.len(), 1);
            let inner_content =
                get_inner_module("inner", &rust_file.symbols).expect("Should find inner module");
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

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = match &rust_file.symbols[0] {
                RustSymbol::ModuleDeclaration { name, .. } => name,
                _ => panic!("Expected ModuleDeclaration variant"),
            };
            assert_eq!(symbol, "other");
        }
    }

    mod reexports {
        use super::*;

        #[test]
        fn single_item() {
            let source_code = r#"
pub use inner::Format;
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(rust_file.symbols.len(), 1);
            match &rust_file.symbols[0] {
                RustSymbol::SymbolReexport {
                    name, source_path, ..
                } => {
                    assert_eq!(name, "Format");
                    assert_eq!(source_path, "inner::Format");
                }
                _ => panic!("Expected SymbolReexport variant"),
            }
        }

        #[test]
        fn use_tree_list() {
            let source_code = r#"
pub use inner::{TextFormatter, OtherType};
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(rust_file.symbols.len(), 2);
            let mut found_text_formatter = false;
            let mut found_other_type = false;

            for symbol in &rust_file.symbols {
                match symbol {
                    RustSymbol::SymbolReexport {
                        name, source_path, ..
                    } => match name.as_str() {
                        "TextFormatter" => {
                            assert_eq!(source_path, "inner::TextFormatter");
                            found_text_formatter = true;
                        }
                        "OtherType" => {
                            assert_eq!(source_path, "inner::OtherType");
                            found_other_type = true;
                        }
                        _ => panic!("Unexpected symbol name"),
                    },
                    _ => panic!("Expected SymbolReexport variant"),
                }
            }

            assert!(found_text_formatter, "TextFormatter not found");
            assert!(found_other_type, "OtherType not found");
        }
    }

    mod module_declarations {
        use super::*;

        #[test]
        fn public_module() {
            let source_code = r#"
pub mod test_module;
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(rust_file.symbols.len(), 1);
            match &rust_file.symbols[0] {
                RustSymbol::ModuleDeclaration { name, is_public } => {
                    assert_eq!(name, "test_module");
                    assert!(is_public);
                }
                _ => panic!("Expected ModuleDeclaration variant"),
            }
        }

        #[test]
        fn private_module() {
            let source_code = r#"
mod test_module;
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(rust_file.symbols.len(), 1);
            match &rust_file.symbols[0] {
                RustSymbol::ModuleDeclaration { name, is_public } => {
                    assert_eq!(name, "test_module");
                    assert!(!is_public);
                }
                _ => panic!("Expected ModuleDeclaration variant"),
            }
        }
    }
}
