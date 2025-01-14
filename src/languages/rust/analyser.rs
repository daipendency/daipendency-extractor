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

impl Default for RustAnalyser {
    fn default() -> Self {
        Self::new()
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
