use std::error::Error;
use std::fmt;

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

impl fmt::Display for LaibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaibraryError::Io(e) => write!(f, "I/O Error: {}", e),
            LaibraryError::Parse(msg) => write!(f, "Parse Error: {}", msg),
            LaibraryError::UnsupportedLanguage(lang) => write!(f, "Unsupported Language: {}", lang),
            LaibraryError::InvalidPath(path) => write!(f, "Invalid Path: {}", path),
        }
    }
}

impl Error for LaibraryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LaibraryError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for LaibraryError {
    fn from(error: std::io::Error) -> Self {
        LaibraryError::Io(error)
    }
}

impl From<std::str::Utf8Error> for LaibraryError {
    fn from(error: std::str::Utf8Error) -> Self {
        LaibraryError::Parse(format!("UTF-8 error: {}", error))
    }
}
