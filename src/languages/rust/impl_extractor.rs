use crate::error::LaibraryError;
use crate::types::{ApiDefinitions, SourceFile};
use tree_sitter::{Node, Parser};
use tree_sitter_rust::LANGUAGE;

pub(super) fn extract_public_api(sources: &[SourceFile]) -> Result<ApiDefinitions, LaibraryError> {
    let mut parser = Parser::new();
    parser.set_language(&LANGUAGE.into()).map_err(|e| {
        LaibraryError::Parse(format!("Error setting Rust language for parser: {}", e))
    })?;

    let mut api = ApiDefinitions {
        functions: Vec::new(),
        structs: Vec::new(),
        enums: Vec::new(),
        traits: Vec::new(),
    };

    for source in sources {
        if let Some(tree) = parser.parse(&source.content, None) {
            let root_node = tree.root_node();
            extract_public_items(root_node, &source.content, &mut api)?;
        } else {
            return Err(LaibraryError::Parse(format!(
                "Failed to parse source file: {}",
                source.path.display()
            )));
        }
    }

    Ok(api)
}

fn extract_public_items(
    node: Node,
    source_code: &str,
    api: &mut ApiDefinitions,
) -> Result<(), LaibraryError> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" | "struct_item" | "enum_item" | "trait_item" => {
                if is_public(&child, source_code) {
                    let item_text = extract_item_signature(&child, source_code)?;
                    match child.kind() {
                        "function_item" => api.functions.push(item_text),
                        "struct_item" => api.structs.push(item_text),
                        "enum_item" => api.enums.push(item_text),
                        "trait_item" => api.traits.push(item_text),
                        _ => unreachable!(),
                    }
                }
            }
            _ => {
                // Recursively process child nodes
                extract_public_items(child, source_code, api)?;
            }
        }
    }
    Ok(())
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

fn extract_item_signature(node: &Node, source_code: &str) -> Result<String, LaibraryError> {
    match node.kind() {
        "function_item" => extract_function_signature(node, source_code),
        _ => {
            // For other items, include the full definition
            node.utf8_text(source_code.as_bytes())
                .map(|s| s.to_string())
                .map_err(|e| LaibraryError::Parse(format!("Failed to extract item text: {}", e)))
        }
    }
}

fn extract_function_signature(node: &Node, source_code: &str) -> Result<String, LaibraryError> {
    let mut cursor = node.walk();
    let mut signature = String::new();
    let mut in_return_type = false;

    for part in node.children(&mut cursor) {
        match part.kind() {
            "visibility_modifier" => {
                if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                    signature.push_str(text);
                    signature.push(' ');
                }
            }
            "fn" => {
                signature.push_str("fn ");
            }
            "identifier" => {
                if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                    signature.push_str(text);
                }
            }
            "type_parameters" | "parameters" => {
                if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                    signature.push_str(text);
                }
            }
            "->" => {
                in_return_type = true;
                signature.push_str(" -> ");
            }
            "block" => break,
            "where" => {
                signature.push_str(" where");
            }
            _ if in_return_type => {
                if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                    let text = text.trim_end_matches(',');
                    signature.push_str(text);
                }
            }
            _ => {}
        }
    }

    if !signature.contains("->") {
        signature.push_str(" -> ()");
    }

    signature.push(';');
    Ok(signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_function() {
        let source = SourceFile {
            path: PathBuf::from("test.rs"),
            content: r#"
pub fn simple_function() {}
pub fn function_with_params(x: i32, y: &str) -> String {}
pub fn generic_function<T: Display>(data: T) -> String {}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        assert_eq!(result.functions.len(), 3);
        assert!(result
            .functions
            .contains(&"pub fn simple_function() -> ();".to_string()));
        assert!(result
            .functions
            .contains(&"pub fn function_with_params(x: i32, y: &str) -> String;".to_string()));
        assert!(result
            .functions
            .contains(&"pub fn generic_function<T: Display>(data: T) -> String;".to_string()));
    }

    #[test]
    fn test_extract_struct() {
        let source = SourceFile {
            path: PathBuf::from("test.rs"),
            content: r#"
pub struct TestStruct {
    pub field: String,
    private_field: i32,
}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        assert_eq!(result.structs.len(), 1);
        assert!(result.structs[0].contains("pub struct TestStruct"));
    }

    #[test]
    fn test_extract_enum() {
        let source = SourceFile {
            path: PathBuf::from("test.rs"),
            content: r#"
pub enum TestEnum {
    Variant1,
    Variant2(String),
    Variant3 { field: i32 },
}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        assert_eq!(result.enums.len(), 1);
        assert!(result.enums[0].contains("pub enum TestEnum"));
    }

    #[test]
    fn test_extract_trait() {
        let source = SourceFile {
            path: PathBuf::from("test.rs"),
            content: r#"
pub trait TestTrait {
    fn required_method(&self);
    fn optional_method(&self) {}
}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        assert_eq!(result.traits.len(), 1);
        assert!(result.traits[0].contains("pub trait TestTrait"));
    }

    #[test]
    fn test_private_items_ignored() {
        let source = SourceFile {
            path: PathBuf::from("test.rs"),
            content: r#"
fn private_function() {}
struct PrivateStruct {}
enum PrivateEnum {}
trait PrivateTrait {}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        assert!(result.functions.is_empty());
        assert!(result.structs.is_empty());
        assert!(result.enums.is_empty());
        assert!(result.traits.is_empty());
    }

    #[test]
    fn test_where_clause() {
        let source = SourceFile {
            path: PathBuf::from("test.rs"),
            content: r#"
pub fn complex_function<T>(data: T) -> Vec<String> where
    T: Display + Clone {
    vec![]
}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert!(result.functions[0].contains("where"));
        assert!(result.functions[0].contains("T: Display + Clone"));
    }
}
