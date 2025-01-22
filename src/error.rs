/// Custom error type for Daipendency extraction operations
#[derive(Debug)]
pub enum ExtractionError {
    /// Parsing related errors
    Parse(String),
}
