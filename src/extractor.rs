use crate::error::{DependencyResolutionError, ExtractionError};
use crate::library_metadata::{LibraryMetadata, LibraryMetadataError};
use crate::types::Namespace;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser};

/// Extract metadata and public API information from a library.
pub trait Extractor<EntryPoint> {
    /// Provide the TreeSitter language
    fn get_parser_language(&self) -> Language;

    /// Provide the library metadata
    fn get_library_metadata(
        &self,
        path: &Path,
    ) -> Result<LibraryMetadata<EntryPoint>, LibraryMetadataError>;

    /// Extract the public API
    fn extract_public_api(
        &self,
        metadata: &LibraryMetadata<EntryPoint>,
        parser: &mut Parser,
    ) -> Result<Vec<Namespace>, ExtractionError>;

    /// Resolve the path to a dependency
    fn resolve_dependency_path(
        &self,
        dependency_name: &str,
        dependant_path: &Path,
    ) -> Result<PathBuf, DependencyResolutionError>;
}
