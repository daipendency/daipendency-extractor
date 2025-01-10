use crate::error::LaibraryError;
use crate::languages::rust::{parsing, types::RustSymbol};
use crate::types::Namespace;
use std::path::Path;
use tree_sitter::Parser;

pub fn build_public_api(
    path: &Path,
    crate_name: &str,
    parser: &mut Parser,
) -> Result<Vec<Namespace>, LaibraryError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        LaibraryError::Parse(format!("Failed to read file '{}': {}", path.display(), e))
    })?;
    let module_path = determine_module_path(path)?;
    let module_path = module_path.unwrap_or_default();

    let prefixed_module_path = if module_path.is_empty() {
        crate_name.to_string()
    } else {
        format!("{}::{}", crate_name, module_path)
    };

    let file_symbols = parsing::parse_rust_file(&content, parser)?;
    extract_modules(file_symbols, &prefixed_module_path)
}

fn extract_modules(
    symbols: Vec<RustSymbol>,
    prefix: &str,
) -> Result<Vec<Namespace>, LaibraryError> {
    let mut namespaces = Vec::new();
    let mut current_namespace = Namespace {
        name: prefix.to_string(),
        symbols: Vec::new(),
    };

    for symbol in symbols {
        match symbol {
            RustSymbol::Symbol(symbol) => {
                current_namespace.symbols.push(symbol);
            }
            RustSymbol::Module { name, content } => {
                let module_path = format!("{}::{}", prefix, name);
                let mut nested = extract_modules(content, &module_path)?;
                namespaces.append(&mut nested);
            }
        }
    }

    namespaces.push(current_namespace);
    Ok(namespaces)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::test_helpers::setup_parser;
    use crate::languages::test_helpers::{create_file, create_temp_dir};
    use std::path::PathBuf;

    const STUB_CRATE_NAME: &str = "test_crate";

    #[test]
    fn nonexistent_file() {
        let mut parser = setup_parser();
        let path = PathBuf::from("nonexistent.rs");

        let err = build_public_api(&path, STUB_CRATE_NAME, &mut parser).unwrap_err();

        assert!(matches!(err, LaibraryError::Parse(_)));
        assert!(err
            .to_string()
            .contains("Failed to read file 'nonexistent.rs'"));
    }

    #[test]
    fn lib_rs_has_no_module_path() {
        let temp_dir = create_temp_dir();
        let path = temp_dir.path().join("src").join("lib.rs");
        create_file(&path, "");

        let module_path = determine_module_path(&path).unwrap();

        assert!(module_path.is_none());
    }

    #[test]
    fn direct_module_file_has_single_segment_path() {
        let temp_dir = create_temp_dir();
        let path = temp_dir.path().join("src").join("text.rs");
        create_file(&path, "");

        let module_path = determine_module_path(&path).unwrap();

        assert_eq!(module_path.unwrap(), "text".to_string());
    }

    #[test]
    fn mod_rs_has_directory_name_path() {
        let temp_dir = create_temp_dir();
        let path = temp_dir.path().join("src").join("text").join("mod.rs");
        create_file(&path, "");

        let module_path = determine_module_path(&path).unwrap();

        assert_eq!(module_path.unwrap(), "text".to_string());
    }

    #[test]
    fn nested_module_has_multi_segment_path() {
        let temp_dir = create_temp_dir();
        let path = temp_dir
            .path()
            .join("src")
            .join("text")
            .join("formatter.rs");
        create_file(&path, "");

        let module_path = determine_module_path(&path).unwrap();

        assert_eq!(module_path.unwrap(), "text::formatter".to_string());
    }

    #[test]
    fn extract_modules_handles_nested_structure() {
        let source_code = r#"
pub fn root_fn() {}

pub mod submod {
    pub fn nested_fn() {}
}
"#;
        let temp_dir = create_temp_dir();
        let path = temp_dir.path().join("src").join("lib.rs");
        create_file(&path, source_code);
        let mut parser = setup_parser();

        let namespaces = build_public_api(&path, STUB_CRATE_NAME, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 2);

        let root = namespaces
            .iter()
            .find(|n| n.name == STUB_CRATE_NAME)
            .unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert_eq!(root.symbols[0].name, "root_fn");

        let submod = namespaces
            .iter()
            .find(|n| n.name == format!("{}::submod", STUB_CRATE_NAME))
            .unwrap();
        assert_eq!(submod.symbols.len(), 1);
        assert_eq!(submod.symbols[0].name, "nested_fn");
    }
}
