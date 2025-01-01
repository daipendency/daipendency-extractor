use crate::languages::rust::RustAnalyser;

use crate::error::LaibraryError;
use crate::types::{ApiRepresentation, PackageMetadata, SourceFile};
use std::path::Path;

pub trait LibraryAnalyser {
    type Api: ApiRepresentation;

    fn extract_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError>;
    fn parse_source(&self, path: &Path) -> Result<Vec<SourceFile>, LaibraryError>;
    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<Self::Api, LaibraryError>;
    fn format_documentation(&self, api: &Self::Api) -> Result<String, LaibraryError>;
}

/// Get an analyser for the specified language
pub fn get_analyser(
    language: &str,
) -> Result<Box<dyn LibraryAnalyser<Api = impl ApiRepresentation>>, LaibraryError> {
    match language {
        "rust" => Ok(Box::new(RustAnalyser::new())),
        _ => Err(LaibraryError::UnsupportedLanguage(language.to_string())),
    }
}
