/// Custom error type for laibrary operations
#[derive(Debug)]
pub enum LaibraryError {
    /// I/O related errors
    Io(std::io::Error),
    /// Parsing related errors
    Parse(String),
    /// Unsupported language errors
    UnsupportedLanguage(String),
    /// Invalid path errors
    InvalidPath(String),
}
