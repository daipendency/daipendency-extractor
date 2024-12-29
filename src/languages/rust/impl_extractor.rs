use super::RustApi;
use crate::error::LaibraryError;
use crate::types::SourceFile;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser};
use tree_sitter_rust::LANGUAGE;

pub(super) fn extract_public_api(sources: &[SourceFile]) -> Result<RustApi, LaibraryError> {
    let mut parser = Parser::new();
    parser.set_language(&LANGUAGE.into()).map_err(|e| {
        LaibraryError::Parse(format!("Error setting Rust language for parser: {}", e))
    })?;

    let mut all_modules = HashMap::new();

    for source in sources {
        if let Some(tree) = parser.parse(&source.content, None) {
            let root_node = tree.root_node();
            let module_path = determine_module_path(&source.path)?;
            let modules = extract_public_items(root_node, &source.content, module_path.as_deref())?;
            all_modules.extend(modules);
        } else {
            return Err(LaibraryError::Parse(format!(
                "Failed to parse source file: {}",
                source.path.display()
            )));
        }
    }

    Ok(RustApi { modules: all_modules })
}

fn determine_module_path(file_path: &Path) -> Result<Option<String>, LaibraryError> {
    let file_name = file_path.file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| LaibraryError::Parse("Invalid file name".to_string()))?;

    if file_name == "lib.rs" {
        return Ok(None);
    }

    let mut path_components = Vec::new();
    path_components.push("rust_crate".to_string());

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
    if path_components.len() == 1 && file_name != "mod.rs" {
        Ok(None)
    } else {
        Ok(Some(path_components.join("::")))
    }
}

fn extract_public_items(
    node: Node,
    source_code: &str,
    module_path: Option<&str>,
) -> Result<HashMap<String, Vec<String>>, LaibraryError> {
    let mut modules = HashMap::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "mod_item" => {
                if is_public(&child, source_code) {
                    let mod_name = extract_mod_name(&child, source_code)?;
                    let new_module_path = match module_path {
                        Some(path) => format!("{}::{}", path, mod_name),
                        None => format!("rust_crate::{}", mod_name),
                    };
                    let nested_modules = extract_public_items(child, source_code, Some(&new_module_path))?;
                    modules.extend(nested_modules);
                }
            }
            "use_declaration" => {
                if is_public(&child, source_code) && module_path.is_some() {
                    // TODO: Handle re-exports
                }
            }
            "function_item" | "struct_item" | "enum_item" | "trait_item" => {
                if is_public(&child, source_code) && module_path.is_some() {
                    let item_text = extract_item_signature(&child, source_code)?;
                    modules
                        .entry(module_path.unwrap().to_string())
                        .or_default()
                        .push(item_text);
                }
            }
            _ => {
                let nested_modules = extract_public_items(child, source_code, module_path)?;
                for (path, items) in nested_modules {
                    modules
                        .entry(path)
                        .or_default()
                        .extend(items);
                }
            }
        }
    }
    Ok(modules)
}

fn extract_mod_name(node: &Node, source_code: &str) -> Result<String, LaibraryError> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return child
                .utf8_text(source_code.as_bytes())
                .map(|s| s.to_string())
                .map_err(|e| LaibraryError::Parse(format!("Failed to extract module name: {}", e)));
        }
    }
    Err(LaibraryError::Parse("Failed to find module name".to_string()))
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
    let mut in_where_clause = false;

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
                in_where_clause = true;
                signature.push_str(" where");
            }
            _ if in_where_clause => {
                if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                    signature.push_str(text);
                }
            }
            _ if in_return_type => {
                if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                    let text = text.trim_end_matches(',');
                    signature.push_str(text);
                    if !text.ends_with(' ') {
                        signature.push(' ');
                    }
                }
            }
            _ => {}
        }
    }

    if !signature.contains("->") {
        signature.push_str(" -> ()");
    }

    signature = signature.trim_end().to_string();
    signature.push(';');
    Ok(signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_determine_module_path() {
        // lib.rs should have no module path
        assert_eq!(
            determine_module_path(&PathBuf::from("src/lib.rs")).unwrap(),
            None
        );

        // Direct module file
        assert_eq!(
            determine_module_path(&PathBuf::from("src/text.rs")).unwrap(),
            Some("rust_crate::text".to_string())
        );

        // Module in mod.rs
        assert_eq!(
            determine_module_path(&PathBuf::from("src/text/mod.rs")).unwrap(),
            Some("rust_crate::text".to_string())
        );

        // Nested module
        assert_eq!(
            determine_module_path(&PathBuf::from("src/text/formatter.rs")).unwrap(),
            Some("rust_crate::text::formatter".to_string())
        );
    }

    #[test]
    fn test_extract_nested_modules() {
        let source = SourceFile {
            path: PathBuf::from("src/text/mod.rs"),
            content: r#"
pub mod inner {
    pub fn nested_function() -> String {}
}
pub fn outer_function() -> i32 {}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        let modules = result.modules;

        assert!(modules.contains_key("rust_crate::text"));
        assert!(modules.contains_key("rust_crate::text::inner"));
        assert_eq!(
            modules.get("rust_crate::text").unwrap(),
            &vec!["pub fn outer_function() -> i32;"]
        );
        assert_eq!(
            modules.get("rust_crate::text::inner").unwrap(),
            &vec!["pub fn nested_function() -> String;"]
        );
    }

    #[test]
    fn test_extract_mixed_items() {
        let source = SourceFile {
            path: PathBuf::from("src/text/mod.rs"),
            content: r#"
pub struct TestStruct {
    pub field: String,
}
pub enum TestEnum {
    A,
    B(i32),
}
pub fn test_function() -> i32 {}
pub trait TestTrait {
    fn method(&self);
}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        let modules = result.modules;

        let test_module_items = modules.get("rust_crate::text").unwrap();
        assert_eq!(test_module_items.len(), 4);
        assert!(test_module_items
            .iter()
            .any(|item| item.contains("pub struct TestStruct")));
        assert!(test_module_items
            .iter()
            .any(|item| item.contains("pub enum TestEnum")));
        assert!(test_module_items
            .iter()
            .any(|item| item.contains("pub fn test_function() -> i32;")));
        assert!(test_module_items
            .iter()
            .any(|item| item.contains("pub trait TestTrait")));
    }

    #[test]
    fn test_private_items_ignored() {
        let source = SourceFile {
            path: PathBuf::from("src/text/mod.rs"),
            content: r#"
struct PrivateStruct {}
fn private_function() {}
pub fn public_function() -> () {}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        let modules = result.modules;

        let test_module_items = modules.get("rust_crate::text").unwrap();
        assert_eq!(test_module_items.len(), 1);
        assert_eq!(test_module_items[0], "pub fn public_function() -> ();");
    }

    #[test]
    fn test_empty_source() {
        let source = SourceFile {
            path: PathBuf::from("src/empty.rs"),
            content: String::new(),
        };

        let result = extract_public_api(&[source]).unwrap();
        assert!(result.modules.is_empty());
    }

    #[test]
    fn test_lib_items_ignored() {
        let source = SourceFile {
            path: PathBuf::from("src/lib.rs"),
            content: r#"
pub fn root_function() -> () {}
pub struct RootStruct {}

pub mod text {
    pub fn text_function() -> () {}
}
"#
            .to_string(),
        };

        let result = extract_public_api(&[source]).unwrap();
        let modules = result.modules;

        // Root items should be ignored
        assert!(!modules.contains_key("rust_crate"));
        
        // Module items should be included
        let text_module = modules.get("rust_crate::text").unwrap();
        assert_eq!(text_module.len(), 1);
        assert_eq!(text_module[0], "pub fn text_function() -> ();");
    }
}
