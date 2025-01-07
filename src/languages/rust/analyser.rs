use super::extraction;
use super::formatting;
use super::metadata;
use crate::analysers::Analyser;
use crate::error::LaibraryError;
use crate::types::{Namespace, PackageMetadata, SourceFile};
use std::path::Path;
use tree_sitter::Language;

pub struct RustAnalyser;

impl RustAnalyser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustAnalyser {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyser for RustAnalyser {
    fn get_file_extensions(&self) -> Vec<String> {
        vec!["rs".to_string()]
    }

    fn get_parser_language(&self) -> Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn get_package_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError> {
        metadata::extract_metadata(path)
    }

    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<Vec<Namespace>, LaibraryError> {
        extraction::extract_modules(sources)
    }

    fn format_namespace(&self, module: &Namespace) -> Result<String, LaibraryError> {
        formatting::format_module(module)
    }
}
