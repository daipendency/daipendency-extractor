use super::RustApi;
use crate::error::LaibraryError;
use crate::types::SourceFile;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser};
use tree_sitter_rust::LANGUAGE;
use super::public_members::{RustPublicMember, Function, Parameter, TypeParameter, Struct, Enum, Trait};

pub fn extract_public_api(sources: &[SourceFile]) -> Result<RustApi, LaibraryError> {
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

fn extract_public_items(
    node: Node,
    source_code: &str,
    module_path: Option<&str>,
) -> Result<HashMap<String, Vec<RustPublicMember>>, LaibraryError> {
    let mut modules = HashMap::new();
    let mut cursor = node.walk();
    let mut items = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "mod_item" => {
                if is_public(&child, source_code) {
                    let mod_name = extract_mod_name(&child, source_code)?;
                    let new_module_path = match module_path {
                        Some(path) => format!("{}::{}", path, mod_name),
                        None => mod_name.clone(),
                    };
                    
                    // Process inline module contents if present
                    for mod_child in child.children(&mut child.walk()) {
                        if mod_child.kind() == "declaration_list" {
                            let nested_modules = extract_public_items(mod_child, source_code, Some(&new_module_path))?;
                            modules.extend(nested_modules);
                        }
                    }
                    
                    // Always add the module to the map, even if it's empty
                    if !modules.contains_key(&new_module_path) {
                        modules.insert(new_module_path, Vec::new());
                    }
                }
            }
            "use_declaration" => {
                if is_public(&child, source_code) && module_path.is_some() {
                    // TODO: Handle re-exports
                }
            }
            "function_item" => {
                if is_public(&child, source_code) {
                    items.push(extract_function_signature(&child, source_code)?);
                }
            }
            "struct_item" | "enum_item" | "trait_item" => {
                if is_public(&child, source_code) {
                    items.push(extract_item_signature(&child, source_code)?);
                }
            }
            _ => {}
        }
    }

    if let Some(path) = module_path {
        if !items.is_empty() {
            modules.insert(path.to_string(), items);
        }
    } else if !items.is_empty() && node.kind() != "source_file" {
        // Only add items to root if we're not in lib.rs (source_file)
        modules.insert("".to_string(), items);
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

fn extract_item_signature(node: &Node, source_code: &str) -> Result<RustPublicMember, LaibraryError> {
    match node.kind() {
        "function_item" => extract_function_signature(node, source_code),
        "struct_item" => {
            let mut cursor = node.walk();
            let mut name = String::new();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                        name = text.to_string();
                        break;
                    }
                }
            }
            if name.is_empty() {
                return Err(LaibraryError::Parse("Failed to find struct name".to_string()));
            }
            Ok(RustPublicMember::Struct(Struct {
                name,
                fields: vec![],
                doc_comment: None,
                type_parameters: vec![],
            }))
        }
        "enum_item" => {
            let mut cursor = node.walk();
            let mut name = String::new();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                        name = text.to_string();
                        break;
                    }
                }
            }
            if name.is_empty() {
                return Err(LaibraryError::Parse("Failed to find enum name".to_string()));
            }
            Ok(RustPublicMember::Enum(Enum {
                name,
                variants: vec![],
                doc_comment: None,
                type_parameters: vec![],
            }))
        }
        "trait_item" => {
            let mut cursor = node.walk();
            let mut name = String::new();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                        name = text.to_string();
                        break;
                    }
                }
            }
            if name.is_empty() {
                return Err(LaibraryError::Parse("Failed to find trait name".to_string()));
            }
            Ok(RustPublicMember::Trait(Trait {
                name,
                methods: vec![],
                doc_comment: None,
                type_parameters: vec![],
            }))
        }
        _ => {
            let text = node.utf8_text(source_code.as_bytes())?;
            Ok(RustPublicMember::Macro(super::public_members::Macro {
                name: String::new(),
                definition: text.to_string(),
                doc_comment: None,
            }))
        }
    }
}

fn extract_function_signature(node: &Node, source_code: &str) -> Result<RustPublicMember, LaibraryError> {
    let mut cursor = node.walk();
    let mut name = String::new();
    let mut parameters = Vec::new();
    let mut return_type = None;
    let mut type_parameters = Vec::new();
    let mut where_clause = None;

    // First pass: get the basic structure
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                    name = text.to_string();
                }
            }
            "type_parameters" => {
                type_parameters = extract_type_parameters(&child, source_code)?;
            }
            "parameters" => {
                parameters = extract_parameters(&child, source_code)?;
            }
            "return_type" => {
                let mut child_cursor = child.walk();
                for grandchild in child.children(&mut child_cursor) {
                    match grandchild.kind() {
                        "type" => {
                            let type_str = extract_type(&grandchild, source_code)?;
                            return_type = Some(type_str);
                            break;
                        }
                        "generic_type" => {
                            let type_str = extract_generic_type(&grandchild, source_code)?;
                            return_type = Some(type_str);
                            break;
                        }
                        "type_identifier" => {
                            if let Ok(text) = grandchild.utf8_text(source_code.as_bytes()) {
                                return_type = Some(text.to_string());
                                break;
                            }
                        }
                        "primitive_type" => {
                            if let Ok(text) = grandchild.utf8_text(source_code.as_bytes()) {
                                return_type = Some(text.to_string());
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            "generic_type" => {
                // This handles cases where the generic_type is directly under the function node
                let type_str = extract_generic_type(&child, source_code)?;
                return_type = Some(type_str);
            }
            "type_identifier" => {
                // This handles cases where the type_identifier is directly under the function node
                if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                    return_type = Some(text.to_string());
                }
            }
            "primitive_type" => {
                // This handles cases where the primitive_type is directly under the function node
                if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                    return_type = Some(text.to_string());
                }
            }
            "where_clause" => {
                if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                    where_clause = Some(text.trim_start_matches("where").trim().to_string());
                }
            }
            _ => {}
        }
    }

    // If no explicit return type is specified, it's unit ()
    if return_type.is_none() {
        return_type = Some("()".to_string());
    }

    Ok(RustPublicMember::Function(Function {
        name,
        parameters,
        return_type,
        doc_comment: None,
        type_parameters,
        where_clause,
    }))
}

fn extract_type_parameters(node: &Node, source_code: &str) -> Result<Vec<TypeParameter>, LaibraryError> {
    let mut type_parameters = Vec::new();

    for child in node.children(&mut node.walk()) {
        match child.kind() {
            "type_parameter" | "constrained_type_parameter" => {
                let mut name = String::new();
                let mut bounds = Vec::new();

                for param_child in child.children(&mut child.walk()) {
                    match param_child.kind() {
                        "type_identifier" => {
                            if let Ok(text) = param_child.utf8_text(source_code.as_bytes()) {
                                name = text.to_string();
                            }
                        }
                        "trait_bounds" => {
                            if let Ok(text) = param_child.utf8_text(source_code.as_bytes()) {
                                let bound = text.trim_start_matches(":").trim().to_string();
                                if !bound.is_empty() {
                                    bounds.push(bound);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if !name.is_empty() {
                    type_parameters.push(TypeParameter { name, bounds });
                }
            }
            _ => {}
        }
    }

    Ok(type_parameters)
}

fn extract_parameters(node: &Node, source_code: &str) -> Result<Vec<Parameter>, LaibraryError> {
    let mut parameters = Vec::new();
    let mut cursor = node.walk();

    for param in node.children(&mut cursor) {
        if param.kind() == "parameter" {
            let mut param_cursor = param.walk();
            let mut name = String::new();
            let mut type_name = String::new();

            // First pass: get the parameter name
            for child in param.children(&mut param_cursor) {
                if child.kind() == "identifier" {
                    if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                        name = text.to_string();
                    }
                }
            }

            // Second pass: get the parameter type
            let mut param_cursor = param.walk();
            for child in param.children(&mut param_cursor) {
                match child.kind() {
                    "primitive_type" | "type_identifier" => {
                        if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                            type_name = text.to_string();
                        }
                    }
                    _ => {}
                }
            }

            parameters.push(Parameter { name, type_name });
        }
    }

    Ok(parameters)
}

fn extract_type(node: &Node, source_code: &str) -> Result<String, LaibraryError> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "generic_type" => {
                return extract_generic_type(&child, source_code);
            }
            "type_identifier" | "primitive_type" => {
                if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                    return Ok(text.to_string());
                }
            }
            "qualified_type" | "scoped_type_identifier" => {
                if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                    return Ok(text.to_string());
                }
            }
            "dynamic_type" => {
                let mut type_str = String::new();
                let mut found_dyn = false;
                for dyn_child in child.children(&mut child.walk()) {
                    if let Ok(text) = dyn_child.utf8_text(source_code.as_bytes()) {
                        if text == "dyn" {
                            found_dyn = true;
                        }
                        type_str.push_str(text);
                        type_str.push_str(" ");
                    }
                }
                if !found_dyn {
                    type_str.insert_str(0, "dyn ");
                }
                type_str.pop(); // Remove trailing space
                return Ok(type_str);
            }
            _ => continue
        }
    }
    
    Ok("".to_string())
}

fn extract_generic_type(node: &Node, source_code: &str) -> Result<String, LaibraryError> {
    let mut type_str = String::new();
    let mut cursor = node.walk();
    
    // Get the base type (e.g., "Result")
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                type_str.push_str(text);
                break;
            }
        }
    }

    // Get the type arguments
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_arguments" {
            type_str.push('<');
            let mut first = true;
            let mut args_cursor = child.walk();
            for type_arg in child.children(&mut args_cursor) {
                match type_arg.kind() {
                    "type" => {
                        if !first {
                            type_str.push_str(", ");
                        }
                        first = false;
                        let arg_type = extract_type(&type_arg, source_code)?;
                        type_str.push_str(&arg_type);
                    }
                    "unit_type" => {
                        if !first {
                            type_str.push_str(", ");
                        }
                        first = false;
                        type_str.push_str("()");
                    }
                    "generic_type" => {
                        if !first {
                            type_str.push_str(", ");
                        }
                        first = false;
                        let nested_type = extract_generic_type(&type_arg, source_code)?;
                        type_str.push_str(&nested_type);
                    }
                    "type_identifier" => {
                        if !first {
                            type_str.push_str(", ");
                        }
                        first = false;
                        if let Ok(text) = type_arg.utf8_text(source_code.as_bytes()) {
                            type_str.push_str(text);
                        }
                    }
                    "qualified_type" => {
                        if !first {
                            type_str.push_str(", ");
                        }
                        first = false;
                        if let Ok(text) = type_arg.utf8_text(source_code.as_bytes()) {
                            type_str.push_str(text);
                        }
                    }
                    "scoped_type_identifier" => {
                        if !first {
                            type_str.push_str(", ");
                        }
                        first = false;
                        if let Ok(text) = type_arg.utf8_text(source_code.as_bytes()) {
                            type_str.push_str(text);
                        }
                    }
                    "dynamic_type" => {
                        if !first {
                            type_str.push_str(", ");
                        }
                        first = false;
                        let mut found_dyn = false;
                        for dyn_child in type_arg.children(&mut type_arg.walk()) {
                            if let Ok(text) = dyn_child.utf8_text(source_code.as_bytes()) {
                                if text == "dyn" {
                                    found_dyn = true;
                                }
                                type_str.push_str(text);
                                type_str.push_str(" ");
                            }
                        }
                        if !found_dyn {
                            type_str.insert_str(type_str.len() - 1, "dyn ");
                        }
                        type_str.pop(); // Remove trailing space
                    }
                    "," => continue,
                    "<" | ">" => continue,
                    _ => {}
                }
            }
            type_str.push('>');
            break;
        }
    }

    Ok(type_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::languages::rust::public_members::{Function, Parameter, TypeParameter};

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

            assert!(modules.contains_key("text"));
            assert!(modules.contains_key("text::inner"));
            assert_eq!(
                modules.get("text").unwrap(),
                &vec![RustPublicMember::Function(Function {
                    name: "outer_function".to_string(),
                    parameters: vec![],
                    return_type: Some("i32".to_string()),
                    doc_comment: None,
                    type_parameters: vec![],
                    where_clause: None,
                })]
            );
            assert_eq!(
                modules.get("text::inner").unwrap(),
                &vec![RustPublicMember::Function(Function {
                    name: "nested_function".to_string(),
                    parameters: vec![],
                    return_type: Some("String".to_string()),
                    doc_comment: None,
                    type_parameters: vec![],
                    where_clause: None,
                })]
            );
        }

        #[test]
        fn private_modules_are_ignored() {
            let source = SourceFile {
                path: PathBuf::from("src/text/mod.rs"),
                content: r#"
mod private {
    pub fn private_function() -> String {}
}
pub fn public_function() -> i32 {}
"#
                .to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            let modules = result.modules;

            assert!(modules.contains_key("text"));
            assert!(!modules.contains_key("text::private"));
            assert_eq!(modules.get("text").unwrap().len(), 1);
        }

        #[test]
        fn empty_modules_are_included() {
            let source = SourceFile {
                path: PathBuf::from("src/text/mod.rs"),
                content: r#"
pub mod empty {}
"#
                .to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            let modules = result.modules;

            assert!(modules.contains_key("text::empty"));
            assert!(modules.get("text::empty").unwrap().is_empty());
        }
    }

    mod functions {
        use super::*;

        #[test]
        fn basic_function_with_no_params() {
            let source = SourceFile {
                path: PathBuf::from("src/text.rs"),
                content: r#"pub fn test_no_params() -> () {}"#.to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            let func = get_first_function(&result.modules, "text");

            assert_eq!(func.name, "test_no_params");
            assert!(func.parameters.is_empty());
            assert_eq!(func.return_type, Some("()".to_string()));
            assert!(func.type_parameters.is_empty());
            assert!(func.where_clause.is_none());
        }

        #[test]
        fn function_with_implicit_unit_return() {
            let source = SourceFile {
                path: PathBuf::from("src/text.rs"),
                content: r#"pub fn test_no_return() {}"#.to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            let func = get_first_function(&result.modules, "text");

            assert_eq!(func.name, "test_no_return");
            assert!(func.parameters.is_empty());
            assert_eq!(func.return_type, Some("()".to_string()));
        }

        #[test]
        fn function_with_parameters() {
            let source = SourceFile {
                path: PathBuf::from("src/text.rs"),
                content: r#"pub fn test_params(a: i32, b: String) -> bool {}"#.to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            let func = get_first_function(&result.modules, "text");

            assert_eq!(func.name, "test_params");
            assert_eq!(func.parameters, vec![
                Parameter { name: "a".to_string(), type_name: "i32".to_string() },
                Parameter { name: "b".to_string(), type_name: "String".to_string() },
            ]);
            assert_eq!(func.return_type, Some("bool".to_string()));
        }

        #[test]
        fn function_with_generic_parameters() {
            let source = SourceFile {
                path: PathBuf::from("src/text.rs"),
                content: r#"pub fn test_generics<T: std::fmt::Display>(a: T) -> T {}"#.to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            let func = get_first_function(&result.modules, "text");

            assert_eq!(func.name, "test_generics");
            assert_eq!(func.parameters, vec![
                Parameter { name: "a".to_string(), type_name: "T".to_string() },
            ]);
            assert_eq!(func.return_type, Some("T".to_string()));
            assert_eq!(func.type_parameters, vec![
                TypeParameter { name: "T".to_string(), bounds: vec!["std::fmt::Display".to_string()] }
            ]);
        }

        #[test]
        fn function_with_where_clause() {
            let source = SourceFile {
                path: PathBuf::from("src/text.rs"),
                content: r#"pub fn test_where<T>(a: T) -> T where T: std::fmt::Display {}"#.to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            let func = get_first_function(&result.modules, "text");

            assert_eq!(func.name, "test_where");
            assert_eq!(func.parameters, vec![
                Parameter { name: "a".to_string(), type_name: "T".to_string() },
            ]);
            assert_eq!(func.return_type, Some("T".to_string()));
            assert_eq!(func.where_clause, Some("T: std::fmt::Display".to_string()));
        }

        #[test]
        fn function_with_complex_return_type() {
            let source = SourceFile {
                path: PathBuf::from("src/text.rs"),
                content: r#"pub fn test_complex() -> Result<Vec<String>, Box<dyn std::error::Error>> {}"#.to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            let func = get_first_function(&result.modules, "text");

            assert_eq!(func.name, "test_complex");
            assert_eq!(func.return_type, Some("Result<Vec<String>, Box<dyn std::error::Error>>".to_string()));
        }

        fn get_first_function<'a>(modules: &'a HashMap<String, Vec<RustPublicMember>>, module_name: &str) -> &'a Function {
            match &modules.get(module_name).unwrap()[0] {
                RustPublicMember::Function(f) => f,
                _ => panic!("Expected a function"),
            }
        }
    }

    mod visibility {
        use super::*;

        #[test]
        fn private_items_are_ignored() {
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

            let test_module_items = modules.get("text").unwrap();
            assert_eq!(test_module_items.len(), 1);
            assert!(matches!(test_module_items[0], RustPublicMember::Function(_)));
            if let RustPublicMember::Function(func) = &test_module_items[0] {
                assert_eq!(func.name, "public_function");
            }
        }

        #[test]
        fn lib_items_are_ignored() {
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

            assert!(!modules.contains_key(""));
            
            if let Some(text_module) = modules.get("text") {
                assert_eq!(text_module.len(), 1);
                assert!(matches!(text_module[0], RustPublicMember::Function(_)));
                if let RustPublicMember::Function(func) = &text_module[0] {
                    assert_eq!(func.name, "text_function");
                }
            } else {
                panic!("Expected text module to be present");
            }
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn empty_source_file() {
            let source = SourceFile {
                path: PathBuf::from("src/empty.rs"),
                content: String::new(),
            };

            let result = extract_public_api(&[source]).unwrap();
            assert!(result.modules.is_empty());
        }

        #[test]
        fn whitespace_only_source() {
            let source = SourceFile {
                path: PathBuf::from("src/whitespace.rs"),
                content: "    \n\t\n    ".to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            assert!(result.modules.is_empty());
        }

        #[test]
        fn comments_only_source() {
            let source = SourceFile {
                path: PathBuf::from("src/comments.rs"),
                content: "// Just a comment\n/* Another comment */".to_string(),
            };

            let result = extract_public_api(&[source]).unwrap();
            assert!(result.modules.is_empty());
        }
    }
}

