//! Main library module for generating library API documentation.

pub mod error;
pub mod generator;
pub mod parser;
pub mod types;

use error::LaibraryError;
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
    let parser = parser::get_parser("rust")?;
    let library_info = parser.parse(crate_path)?;
    Ok(generator::generate_output(&library_info))
}
