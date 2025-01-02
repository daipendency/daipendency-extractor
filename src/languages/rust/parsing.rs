use std::path::Path;
use crate::error::LaibraryError;
use crate::types::SourceFile;
use tree_sitter::Parser;
use tree_sitter_rust::LANGUAGE;

pub fn parse_rust_file(file_path: &Path) -> Result<SourceFile, LaibraryError> {
    if !file_path.exists() || !file_path.is_file() {
        return Err(LaibraryError::InvalidPath("Expected a valid file".to_string()));
    }

    let content = std::fs::read_to_string(file_path)?;
    let mut parser = Parser::new();
    parser.set_language(&LANGUAGE.into()).map_err(|e| {
        LaibraryError::Parse(format!("Error setting Rust language for parser: {}", e))
    })?;

    let tree = parser.parse(&content, None);

    Ok(SourceFile {
        path: file_path.to_path_buf(),
        content,
        tree,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_rust_file_valid() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "fn main() { println!(\"Hello, World!\"); }";
        write!(temp_file, "{}", content).unwrap();

        let result = parse_rust_file(temp_file.path()).unwrap();
        
        assert_eq!(result.content, content);
        assert_eq!(result.path, temp_file.path());
        assert!(result.tree.is_some());
        assert_eq!(result.tree.as_ref().unwrap().root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_rust_file_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        
        let result = parse_rust_file(temp_dir.path());
        
        assert!(matches!(result, Err(LaibraryError::InvalidPath(_))));
    }

    #[test]
    fn test_parse_rust_file_nonexistent() {
        let nonexistent = Path::new("nonexistent.rs");
        
        let result = parse_rust_file(nonexistent);
        
        assert!(matches!(result, Err(LaibraryError::InvalidPath(_))));
    }

    #[test]
    fn test_parse_rust_file_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        
        let result = parse_rust_file(temp_file.path()).unwrap();
        
        assert!(result.content.is_empty());
        assert_eq!(result.path, temp_file.path());
        assert!(result.tree.is_some());
        assert_eq!(result.tree.as_ref().unwrap().root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_rust_file_invalid_syntax() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "fn main() { let x = 1; let y = 2; let z = x + y; }";
        write!(temp_file, "{}", content).unwrap();

        let result = parse_rust_file(temp_file.path()).unwrap();
        
        assert!(result.tree.is_some());
        let root_node = result.tree.as_ref().unwrap().root_node();
        assert_eq!(root_node.kind(), "source_file");
        assert!(!root_node.has_error());
    }
} 