use super::{api, metadata};
use crate::analysers::Analyser;
use crate::error::LaibraryError;
use crate::types::{Namespace, PackageMetadata};
use std::path::Path;
use tree_sitter::{Language, Parser};

pub struct RustAnalyser;

impl RustAnalyser {
    pub fn new() -> Self {
        Self
    }
}

impl Analyser for RustAnalyser {
    fn get_parser_language(&self) -> Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn get_package_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError> {
        metadata::extract_metadata(path)
    }

    fn extract_public_api(
        &self,
        metadata: &PackageMetadata,
        parser: &mut Parser,
    ) -> Result<Vec<Namespace>, LaibraryError> {
        api::build_public_api(&metadata.entry_point, &metadata.name, parser)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::test_helpers::setup_parser;
    use crate::test_helpers::create_temp_dir;

    #[test]
    fn get_package_metadata() {
        let temp_dir = create_temp_dir();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"[package]
name = "test_crate"
version = "0.1.0"
"#,
        )
        .unwrap();

        let analyser = RustAnalyser::new();
        let metadata = analyser.get_package_metadata(temp_dir.path()).unwrap();

        assert_eq!(metadata.name, "test_crate");
    }

    #[test]
    fn extract_public_api() {
        let temp_dir = create_temp_dir();
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        let lib_rs = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs,
            r#"
pub fn test_function() -> i32 {
    42
}
"#,
        )
        .unwrap();

        let analyser = RustAnalyser::new();
        let metadata = PackageMetadata {
            name: "test_crate".to_string(),
            version: Some("0.1.0".to_string()),
            documentation: String::new(),
            entry_point: lib_rs,
        };
        let mut parser = setup_parser();

        let namespaces = analyser.extract_public_api(&metadata, &mut parser).unwrap();

        assert_eq!(namespaces.len(), 1);
        let root = namespaces.iter().find(|n| n.name == "test_crate").unwrap();
        assert_eq!(root.symbols.len(), 1);
        assert_eq!(root.symbols[0].name, "test_function");
    }
}
