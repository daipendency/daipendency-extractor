//! Main library module for generating library API documentation.

mod analysers;
pub mod error;
mod formatting;
mod languages;
mod listing;
mod parsing;
mod types;

use crate::error::LaibraryError;
use crate::formatting::format_library_context;
use crate::languages::get_analyser;
use crate::listing::get_source_file_paths;
use crate::parsing::parse_source_files;
use std::path::Path;

/// Generate API documentation for a library in the specified language.
///
/// # Arguments
///
/// * `language` - The programming language of the library
/// * `path` - Path to the library's root directory
///
/// # Returns
///
/// Returns a Result containing the generated documentation as a string, or an error if something went wrong.
pub fn generate_documentation(language: &str, path: &Path) -> Result<String, LaibraryError> {
    let analyser = get_analyser(language)?;

    let metadata = analyser.get_package_metadata(path)?;
    let file_paths = get_source_file_paths(
        path.to_string_lossy().into_owned(),
        analyser.get_file_extensions(),
    )?;
    let sources = parse_source_files(&file_paths, &analyser.get_parser_language())?;
    let modules = analyser.extract_public_api(&sources)?;

    format_library_context(&metadata, &modules, analyser.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_unsupported_language() {
        let result = generate_documentation("unsupported", &PathBuf::new());
        assert!(matches!(result, Err(LaibraryError::UnsupportedLanguage(_))));
    }
}
