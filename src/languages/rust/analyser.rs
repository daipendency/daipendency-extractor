use super::extraction;
use super::formatting;
use super::metadata;
use crate::analysers::Analyser;
use crate::error::LaibraryError;
use crate::types::{Module, PackageMetadata, SourceFile};
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

    fn extract_public_api<'a>(
        &self,
        sources: &'a [SourceFile],
    ) -> Result<Vec<Module<'a>>, LaibraryError> {
        extraction::extract_modules(sources)
    }

    fn format_module(&self, module: &Module) -> Result<String, LaibraryError> {
        formatting::format_module(module)
    }
}
