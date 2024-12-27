use super::Parser as ParserTrait;
use crate::error::LaibraryError;
use crate::types::LibraryInfo;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;
use tree_sitter::{Node, Parser};
use tree_sitter_rust::LANGUAGE;

/// Rust-specific parser implementation
pub struct RustParser;

impl ParserTrait for RustParser {
    fn parse(&self, path: &Path) -> Result<LibraryInfo, LaibraryError> {
        // Parse Cargo.toml for name and version
        let cargo_toml_path = path.join("Cargo.toml");
        let cargo_toml_content = fs::read_to_string(&cargo_toml_path)
            .map_err(|e| LaibraryError::Parse(format!("Failed to read Cargo.toml: {}", e)))?;
        let cargo_toml_value: Value = cargo_toml_content
            .parse()
            .map_err(|e| LaibraryError::Parse(format!("Failed to parse Cargo.toml: {}", e)))?;

        let package = cargo_toml_value.get("package").ok_or_else(|| {
            LaibraryError::Parse("Missing [package] section in Cargo.toml".to_string())
        })?;
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| LaibraryError::Parse("Missing 'name' in [package] section".to_string()))?
            .to_string();
        let version = package
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                LaibraryError::Parse("Missing 'version' in [package] section".to_string())
            })?
            .to_string();

        // Read README.md for documentation - don't panic if missing
        let readme_path = path.join("README.md");
        let documentation = fs::read_to_string(&readme_path).unwrap_or_default();

        // Initialize Tree-sitter parser
        let mut parser = Parser::new();
        parser.set_language(&LANGUAGE.into()).map_err(|e| {
            LaibraryError::Parse(format!("Error setting Rust language for parser: {}", e))
        })?;

        // Collect and parse Rust source files
        let src_path = path.join("src");
        let mut api_signatures = String::new();

        let rs_files = collect_rs_files(&src_path)?;

        for file_path in rs_files {
            let source_code = fs::read_to_string(&file_path)
                .map_err(|e| LaibraryError::Parse(format!("Failed to read source file: {}", e)))?;

            // Skip empty files
            if source_code.trim().is_empty() {
                continue;
            }

            match parser.parse(&source_code, None) {
                Some(tree) => {
                    let root_node = tree.root_node();
                    if let Ok(signatures) = extract_public_api(root_node, &source_code) {
                        api_signatures.push_str(&signatures);
                    }
                }
                None => {
                    return Err(LaibraryError::Parse(format!(
                        "Failed to parse source file: {}",
                        file_path.display()
                    )));
                }
            }
        }

        Ok(LibraryInfo {
            name,
            version,
            documentation,
            api: api_signatures,
        })
    }
}

/// Recursively collect all .rs files in a directory
fn collect_rs_files(dir: &Path) -> Result<Vec<PathBuf>, LaibraryError> {
    if !dir.exists() {
        return Err(LaibraryError::Parse(format!(
            "Directory does not exist: {}",
            dir.display()
        )));
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(dir)
        .map_err(|e| LaibraryError::Parse(format!("Failed to read directory: {}", e)))?
    {
        let entry = entry
            .map_err(|e| LaibraryError::Parse(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_rs_files(&path)?);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(files)
}

/// Extract public API signatures from a syntax tree node
fn extract_public_api(node: Node, source_code: &str) -> Result<String, LaibraryError> {
    let mut api = String::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "mod_item" => {
                if is_public(&child, source_code) {
                    if child.kind() == "function_item" {
                        // For functions, only extract the signature
                        let mut sig_cursor = child.walk();
                        let mut signature = String::new();
                        let mut in_return_type = false;

                        // Build the signature
                        for part in child.children(&mut sig_cursor) {
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
                                "type_parameters" => {
                                    if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                                        signature.push_str(text);
                                    }
                                }
                                "parameters" => {
                                    if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                                        signature.push_str(text);
                                    }
                                }
                                "->" => {
                                    in_return_type = true;
                                    signature.push_str(" -> ");
                                }
                                "block" => {
                                    // Stop when we hit the function body
                                    break;
                                }
                                "where" => {
                                    signature.push_str("where");
                                }
                                _ if in_return_type => {
                                    // For return type, include everything after the arrow until the block
                                    if let Ok(text) = part.utf8_text(source_code.as_bytes()) {
                                        let text = text.trim_end_matches(',');
                                        signature.push_str(text);
                                    }
                                }
                                _ => {}
                            }
                        }

                        if !signature.is_empty() {
                            if !in_return_type {
                                signature.push_str(" -> ()");
                            }
                            // Fix spacing around where clause
                            signature = signature.replace("where", " where");
                            signature = signature.replace("  where", " where");
                            signature.push_str(";\n\n");
                            api.push_str(&signature);
                        }
                    } else {
                        // For other items, include the full definition
                        if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                            api.push_str(text);
                            api.push_str("\n\n");
                        }
                    }
                }
            }
            _ => {
                // Recursively process child nodes
                if let Ok(child_api) = extract_public_api(child, source_code) {
                    api.push_str(&child_api);
                }
            }
        }
    }
    Ok(api)
}

/// Check if a node is marked as public
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_crate(dir: &Path) -> Result<(), std::io::Error> {
        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test-crate"
version = "0.1.0"
"#;
        fs::write(dir.join("Cargo.toml"), cargo_toml)?;

        // Create src directory
        fs::create_dir(dir.join("src"))?;

        // Create lib.rs with proper newlines
        let lib_rs = r#"
pub struct TestStruct {
    field: String,
}

pub fn test_function() {}
"#;
        fs::write(dir.join("src").join("lib.rs"), lib_rs.trim())?;

        // Create README.md
        fs::write(dir.join("README.md"), "Test crate")?;

        Ok(())
    }

    #[test]
    fn test_parse_valid_crate() {
        let temp_dir = TempDir::new().unwrap();
        create_test_crate(temp_dir.path()).unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.name, "test-crate");
        assert_eq!(info.version, "0.1.0");
        assert!(info.api.contains("pub struct TestStruct"));
        assert!(info.api.contains("pub fn test_function"));
    }

    #[test]
    fn test_missing_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_invalid_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "invalid toml content").unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_missing_package_section() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[dependencies]\nfoo = \"1.0\"",
        )
        .unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_missing_readme() {
        let temp_dir = TempDir::new().unwrap();
        create_test_crate(temp_dir.path()).unwrap();
        fs::remove_file(temp_dir.path().join("README.md")).unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().documentation, "");
    }

    #[test]
    fn test_invalid_utf8_in_source() {
        let temp_dir = TempDir::new().unwrap();
        create_test_crate(temp_dir.path()).unwrap();

        let mut file = File::create(temp_dir.path().join("src").join("invalid.rs")).unwrap();
        file.write_all(b"pub fn test() -> String {\n    let bytes = b\"\\xFF\\xFF\";\n    String::from_utf8_lossy(bytes).to_string()\n}\n").unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_src_directory() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-crate"
version = "0.1.0"
"#,
        )
        .unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_empty_source_file() {
        let temp_dir = TempDir::new().unwrap();
        create_test_crate(temp_dir.path()).unwrap();
        fs::write(temp_dir.path().join("src").join("empty.rs"), "").unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_signature_only() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create Cargo.toml
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-crate"
version = "0.1.0"
"#,
        )
        .unwrap();

        // Create lib.rs with a function that has a body
        fs::write(
            src_dir.join("lib.rs"),
            r#"
pub fn test_function(input: &str) -> String {
    input.to_uppercase()
}
"#,
        )
        .unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(result.is_ok());

        let info = result.unwrap();
        println!("Generated API:\n{}", info.api); // Debug output
        assert!(info
            .api
            .contains("pub fn test_function(input: &str) -> String;"));
        assert!(!info.api.contains("input.to_uppercase()"));
    }

    #[test]
    fn test_complex_function_signature() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create Cargo.toml
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-crate"
version = "0.1.0"
"#,
        )
        .unwrap();

        // Create lib.rs with a function that has multiple parameters
        fs::write(
            src_dir.join("lib.rs"),
            r#"
pub fn complex_function(x: i32, y: &str, z: Option<Vec<String>>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{}{:?}", y, z))
}
"#,
        ).unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path());
        assert!(result.is_ok());

        let info = result.unwrap();
        println!("Generated API:\n{}", info.api);
        assert!(info.api.contains("pub fn complex_function(x: i32, y: &str, z: Option<Vec<String>>) -> Result<String, Box<dyn std::error::Error>>;"));
        assert!(!info.api.contains("Ok(format!"));
    }

    #[test]
    fn test_where_clause_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-crate"
version = "0.1.0"
"#,
        ).unwrap();

        fs::write(
            src_dir.join("lib.rs"),
            r#"
pub fn generic_function<T>(data: T) -> Vec<String> where
    T: AsRef<str> {
    vec![data.as_ref().to_string()]
}
"#,
        ).unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path()).unwrap();
        println!("Actual API output:\n{}", result.api);
        assert!(result.api.contains("pub fn generic_function<T>(data: T) -> Vec<String> where\n    T: AsRef<str>;"));
    }

    #[test]
    fn test_no_trailing_comma() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-crate"
version = "0.1.0"
"#,
        ).unwrap();

        fs::write(
            src_dir.join("lib.rs"),
            r#"
pub fn stream_function<'a, I>(iter: I) -> Vec<String> where
    I: Iterator<Item = &'a str>, {
    vec![]
}
"#,
        ).unwrap();

        let parser = RustParser;
        let result = parser.parse(temp_dir.path()).unwrap();
        assert!(result.api.contains("pub fn stream_function<'a, I>(iter: I) -> Vec<String> where\n    I: Iterator<Item = &'a str>;"));
    }
}
