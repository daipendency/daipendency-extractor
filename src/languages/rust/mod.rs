mod impl_extractor;
mod impl_generator;
mod impl_metadata;
mod impl_parser;

use crate::analysers::LibraryAnalyser;
use crate::error::LaibraryError;
use crate::types::{ApiRepresentation, PackageMetadata, SourceFile};
use std::any::Any;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct ApiDefinitions {
    pub functions: Vec<String>,
    pub structs: Vec<String>,
    pub enums: Vec<String>,
    pub traits: Vec<String>,
}

impl ApiRepresentation for ApiDefinitions {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct RustAnalyser;

impl RustAnalyser {
    pub fn new() -> Self {
        Self
    }
}

impl LibraryAnalyser for RustAnalyser {
    type Api = ApiDefinitions;

    fn extract_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError> {
        impl_metadata::extract_metadata(path)
    }

    fn parse_source(&self, path: &Path) -> Result<Vec<SourceFile>, LaibraryError> {
        impl_parser::parse_source(path)
    }

    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<Self::Api, LaibraryError> {
        impl_extractor::extract_public_api(sources)
    }

    fn generate_documentation(
        &self,
        metadata: &PackageMetadata,
        api: &Self::Api,
    ) -> Result<String, LaibraryError> {
        impl_generator::generate_documentation(metadata, api)
    }
}
