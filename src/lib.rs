//! Main library module for generating library API documentation.

pub mod analysers;
pub mod error;
pub mod formatting;
mod languages;
pub mod types;

use error::LaibraryError;
use formatting::format_library_context;
use std::path::Path;

/// Generate library API documentation for a given Rust crate path.
///
/// # Arguments
///
/// * `crate_path` - Path to the Rust crate directory
///
/// # Returns
///
/// Returns a Result containing the generated pseudo-XML string or an error
pub fn generate_library_api(crate_path: &Path) -> Result<String, LaibraryError> {
    let analyser = analysers::get_analyser("rust")?;
    let metadata = analyser.extract_metadata(crate_path)?;
    let sources = analyser.parse_source(crate_path)?;
    let api = analyser.extract_public_api(&sources)?;
    let api_content = analyser.generate_documentation(&api)?;
    format_library_context(&metadata, &api_content)
}
