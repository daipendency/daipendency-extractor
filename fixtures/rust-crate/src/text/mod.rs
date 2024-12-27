//! Provides text formatting functionality through the TextFormatter type.

mod formatter;

pub use formatter::{Format, FormatterError, TextFormatter};

/// Represents text processing capabilities.
///
/// This module provides functionality for text formatting and manipulation.
/// See [`TextFormatter`] for the main formatting implementation.
pub mod prelude {
    pub use super::formatter::{Format, TextFormatter};
}
