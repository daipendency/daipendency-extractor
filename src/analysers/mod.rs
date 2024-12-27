pub mod rust;

use crate::error::LaibraryError;
use crate::types::{ApiDefinitions, PackageMetadata, SourceFile};
use std::path::Path;

pub trait LibraryAnalyser {
    fn extract_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError>;
    fn parse_source(&self, path: &Path) -> Result<Vec<SourceFile>, LaibraryError>;
    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<ApiDefinitions, LaibraryError>;
    fn generate_documentation(
        &self,
        metadata: &PackageMetadata,
        api: &ApiDefinitions,
    ) -> Result<String, LaibraryError>;
}

/// Get an analyser for the specified language
pub fn get_analyser(language: &str) -> Result<Box<dyn LibraryAnalyser>, LaibraryError> {
    match language {
        "rust" => Ok(Box::new(rust::RustAnalyser::new())),
        _ => Err(LaibraryError::UnsupportedLanguage(language.to_string())),
    }
}
