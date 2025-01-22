use crate::error::ExtractionError;
use crate::types::{Namespace, PackageMetadata};
use std::path::Path;
use tree_sitter::{Language, Parser};

pub trait Extractor {
    fn get_parser_language(&self) -> Language;
    fn get_package_metadata(&self, path: &Path) -> Result<PackageMetadata, ExtractionError>;
    fn extract_public_api(
        &self,
        metadata: &PackageMetadata,
        parser: &mut Parser,
    ) -> Result<Vec<Namespace>, ExtractionError>;
}
