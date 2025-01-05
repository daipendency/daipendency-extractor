use crate::analysers::LibraryAnalyser;
use crate::error::LaibraryError;
use crate::types::{PackageMetadata, SourceFile};
use std::path::Path;
use tree_sitter::Language;
use tree_sitter_rust::LANGUAGE;

use super::extraction;
use super::formatting;
use super::metadata;
use super::RustApi;

pub struct RustAnalyser;

impl Default for RustAnalyser {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalyser {
    pub fn new() -> Self {
        RustAnalyser
    }
}

impl LibraryAnalyser for RustAnalyser {
    type Api = RustApi;

    fn get_parser_language(&self) -> Language {
        LANGUAGE.into()
    }

    fn get_file_extensions(&self) -> Vec<String> {
        vec!["rs".to_string()]
    }

    fn extract_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError> {
        metadata::extract_metadata(path)
    }

    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<Self::Api, LaibraryError> {
        extraction::extract_public_api(sources)
    }

    fn format_documentation(&self, api: &Self::Api) -> Result<String, LaibraryError> {
        let mut all_members = Vec::new();
        for members in api.modules.values() {
            all_members.extend(members.iter().cloned());
        }
        formatting::format_documentation(&all_members)
    }
}
