use crate::error::LaibraryError;
use crate::types::{Module, PackageMetadata, SourceFile};
use std::path::Path;
use tree_sitter::Language;

pub trait Analyser {
    fn get_file_extensions(&self) -> Vec<String>;
    fn get_parser_language(&self) -> Language;
    fn get_package_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError>;
    fn extract_public_api<'a>(
        &self,
        sources: &'a [SourceFile],
    ) -> Result<Vec<Module<'a>>, LaibraryError>;
    fn format_module(&self, module: &Module) -> Result<String, LaibraryError>;
}
