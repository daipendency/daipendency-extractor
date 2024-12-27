//! Module for parsing library documentation from various languages.

mod rust_parser;
use crate::error::LaibraryError;
use crate::types::LibraryInfo;
use std::path::Path;

/// Trait defining the interface for language parsers
pub trait Parser {
    /// Parse the library documentation from the given path
    fn parse(&self, path: &Path) -> Result<LibraryInfo, LaibraryError>;
}

/// Get a parser for the specified language
///
/// # Arguments
///
/// * `language` - The language to get a parser for
///
/// # Returns
///
/// Returns a boxed parser or an error if the language is unsupported
pub fn get_parser(language: &str) -> Result<Box<dyn Parser>, LaibraryError> {
    match language {
        "rust" => Ok(Box::new(rust_parser::RustParser)),
        _ => Err(LaibraryError::UnsupportedLanguage(language.to_string())),
    }
}
