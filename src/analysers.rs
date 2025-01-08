use crate::error::LaibraryError;
use crate::types::{Namespace, PackageMetadata, SourceFile};
use std::path::Path;
use tree_sitter::Language;

pub trait Analyser {
    fn get_file_extensions(&self) -> Vec<String>;
    fn get_parser_language(&self) -> Language;
    fn get_package_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError>;
    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<Vec<Namespace>, LaibraryError>;
    fn format_namespace(&self, namespace: &Namespace) -> String;
}
