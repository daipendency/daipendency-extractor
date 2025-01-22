use crate::error::ExtractionError;
use crate::library_metadata::{LibraryMetadata, LibraryMetadataError};
use crate::types::Namespace;
use std::path::Path;
use tree_sitter::{Language, Parser};

pub trait Extractor {
    fn get_parser_language(&self) -> Language;
    fn get_library_metadata(&self, path: &Path) -> Result<LibraryMetadata, LibraryMetadataError>;
    fn extract_public_api(
        &self,
        metadata: &LibraryMetadata,
        parser: &mut Parser,
    ) -> Result<Vec<Namespace>, ExtractionError>;
}
