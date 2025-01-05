use crate::error::LaibraryError;
use crate::types::{ApiRepresentation, PackageMetadata, SourceFile};
use std::path::Path;
use tree_sitter::Language;

pub trait LibraryAnalyser {
    type Api: ApiRepresentation;

    fn get_parser_language(&self) -> Language;
    fn get_file_extensions(&self) -> Vec<String>;
    fn get_package_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError>;
    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<Self::Api, LaibraryError>;
    fn format_documentation(&self, api: &Self::Api) -> Result<String, LaibraryError>;
}
