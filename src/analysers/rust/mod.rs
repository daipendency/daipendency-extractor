mod impl_extractor;
mod impl_generator;
mod impl_metadata;
mod impl_parser;

use crate::analysers::LibraryAnalyser;
use crate::error::LaibraryError;
use crate::types::{ApiDefinitions, PackageMetadata, SourceFile};
use std::path::Path;

pub struct RustAnalyser;

impl RustAnalyser {
    pub fn new() -> Self {
        Self
    }
}

impl LibraryAnalyser for RustAnalyser {
    fn extract_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError> {
        impl_metadata::extract_metadata(path)
    }

    fn parse_source(&self, path: &Path) -> Result<Vec<SourceFile>, LaibraryError> {
        impl_parser::parse_source(path)
    }

    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<ApiDefinitions, LaibraryError> {
        impl_extractor::extract_public_api(sources)
    }

    fn generate_documentation(
        &self,
        metadata: &PackageMetadata,
        api: &ApiDefinitions,
    ) -> Result<String, LaibraryError> {
        impl_generator::generate_documentation(metadata, api)
    }
}
