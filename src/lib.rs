//! Main library module for generating library API documentation.

use crate::analysers::get_analyser;
use crate::error::LaibraryError;
use crate::formatting::format_library_context;
use crate::types::ApiRepresentation;
use std::path::Path;

pub mod analysers;
pub mod error;
pub mod formatting;
pub mod languages;
pub mod types;
pub mod listing;

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
    let metadata = analyser.extract_metadata(path)?;
    let sources = analyser.parse_source(path)?;
    let api = analyser.extract_public_api(&sources)?;
    let modules = api.modules();

    format_library_context(&metadata, &modules)
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
