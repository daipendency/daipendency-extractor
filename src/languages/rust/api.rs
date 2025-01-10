use crate::error::LaibraryError;
use crate::languages::rust::{parsing, types::RustSymbol};
use crate::types::{Namespace, SourceFile};
use std::path::Path;

pub fn build_public_api(
    source: &SourceFile,
    crate_name: &str,
) -> Result<Vec<Namespace>, LaibraryError> {
    let module_path = determine_module_path(&source.path)?;
    let module_path = module_path.unwrap_or_default();

    let prefixed_module_path = if module_path.is_empty() {
        crate_name.to_string()
    } else {
        format!("{}::{}", crate_name, module_path)
    };

    let file_symbols = parsing::parse_rust_file(source)?;
    extract_modules(file_symbols, &prefixed_module_path)
}

pub fn extract_modules(
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
    use crate::types::Symbol;

    const STUB_CRATE_NAME: &str = "test_crate";

    #[test]
    fn lib_rs_has_no_module_path() {
        let path = Path::new("src/lib.rs");
        assert!(determine_module_path(path).unwrap().is_none());
    }

    #[test]
    fn direct_module_file_has_single_segment_path() {
        let path = Path::new("src/text.rs");
        assert_eq!(
            determine_module_path(path).unwrap().unwrap(),
            "text".to_string()
        );
    }

    #[test]
    fn mod_rs_has_directory_name_path() {
        let path = Path::new("src/text/mod.rs");
        assert_eq!(
            determine_module_path(path).unwrap().unwrap(),
            "text".to_string()
        );
    }

    #[test]
    fn nested_module_has_multi_segment_path() {
        let path = Path::new("src/text/formatter.rs");
        assert_eq!(
            determine_module_path(path).unwrap().unwrap(),
            "text::formatter".to_string()
        );
    }

    #[test]
    fn extract_modules_handles_nested_structure() {
        let symbols = vec![
            RustSymbol::Symbol(Symbol {
                name: "root_fn".to_string(),
                source_code: "pub fn root_fn() {}".to_string(),
            }),
            RustSymbol::Module {
                name: "submod".to_string(),
                content: vec![RustSymbol::Symbol(Symbol {
                    name: "nested_fn".to_string(),
                    source_code: "pub fn nested_fn() {}".to_string(),
                })],
            },
        ];

        let namespaces = extract_modules(symbols, STUB_CRATE_NAME).unwrap();

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
