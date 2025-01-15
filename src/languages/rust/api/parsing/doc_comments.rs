use crate::error::LaibraryError;
use tree_sitter::Node;

pub fn extract_outer_doc_comments(
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

pub fn extract_inner_doc_comments(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::test_helpers::setup_parser;
    use crate::treesitter_test_helpers::{find_item_node, find_item_nodes};

    mod inner_doc_comments {
        use super::*;

        #[test]
        fn no_doc_comment() {
            let source_code = r#"
pub struct Test {}
"#;
            let mut parser = setup_parser();
            let tree = parser.parse(source_code, None).unwrap();

            let result = extract_inner_doc_comments(&tree.root_node(), source_code).unwrap();

            assert!(result.is_none());
        }

        #[test]
        fn single_line_doc_comment() {
            let source_code = r#"
//! This is a file-level doc comment
pub struct Test {}
"#;
            let mut parser = setup_parser();
            let tree = parser.parse(source_code, None).unwrap();

            let result = extract_inner_doc_comments(&tree.root_node(), source_code).unwrap();

            assert_eq!(
                result,
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
            let tree = parser.parse(source_code, None).unwrap();

            let result = extract_inner_doc_comments(&tree.root_node(), source_code).unwrap();

            assert_eq!(
                result,
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
            let tree = parser.parse(source_code, None).unwrap();

            let result = extract_inner_doc_comments(&tree.root_node(), source_code).unwrap();

            assert!(result.is_none());
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert!(result.is_none());
        }

        #[test]
        fn single_line() {
            let source_code = r#"
/// A documented item
pub struct Test {}
"#;
            let mut parser = setup_parser();
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// A documented item\n".to_string()));
        }

        #[test]
        fn multiple_line() {
            let source_code = r#"
/// First line
/// Second line
pub struct Test {}
"#;
            let mut parser = setup_parser();
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(
                result,
                Some("/// First line\n/// Second line\n".to_string())
            );
        }

        #[test]
        fn inner_doc_comments() {
            let source_code = r#"
//! Inner doc
/// Outer doc
pub struct Test {}
"#;
            let mut parser = setup_parser();
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// Outer doc\n".to_string()));
        }

        #[test]
        fn regular_comments() {
            let source_code = r#"
// Regular comment
/// Doc comment
pub struct Test {}
"#;
            let mut parser = setup_parser();
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// Doc comment\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(
                result,
                Some("/** A block doc comment\n * with multiple lines\n */\n".to_string())
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// This is the struct's doc\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let nodes = find_item_nodes(tree.root_node(), "struct_item");
            let node = &nodes[1];

            let result = extract_outer_doc_comments(node, source_code).unwrap();

            assert_eq!(result, Some("/// Second struct's doc\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(
                result,
                Some("/** This block comment\n * should be returned\n */\n".to_string())
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "enum_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// The doc comment\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "enum_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// The doc comment\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "enum_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert!(result.is_none());
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "function_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// A documented function\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "struct_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// A documented struct\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "enum_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// A documented enum\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let node = find_item_node(tree.root_node(), "trait_item");

            let result = extract_outer_doc_comments(&node, source_code).unwrap();

            assert_eq!(result, Some("/// A documented trait\n".to_string()));
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
            let tree = parser.parse(source_code, None).unwrap();
            let trait_node = find_item_node(tree.root_node(), "trait_item");
            let decl_list = find_item_node(trait_node, "declaration_list");
            let method_node = find_item_node(decl_list, "function_item");

            let result = extract_outer_doc_comments(&method_node, source_code).unwrap();

            assert_eq!(result, Some("/// A documented method\n".to_string()));
        }
    }
}
