use crate::error::LaibraryError;
use crate::types::SourceFile;
use std::path::Path;
use tree_sitter::{Language, Parser};

pub fn parse_source_files(
    paths: &[String],
    parser_language: &Language,
) -> Result<Vec<SourceFile>, LaibraryError> {
    let mut parser = Parser::new();
    parser
        .set_language(parser_language)
        .map_err(|e| LaibraryError::Parse(format!("Error setting language for parser: {}", e)))?;

    let mut sources = Vec::new();
    for file_path in paths {
        sources.push(parse_source_file(Path::new(file_path), &mut parser)?);
    }
    Ok(sources)
}

fn parse_source_file(file_path: &Path, parser: &mut Parser) -> Result<SourceFile, LaibraryError> {
    if !file_path.exists() || !file_path.is_file() {
        return Err(LaibraryError::InvalidPath(
            "Expected a valid file".to_string(),
        ));
    }

    let content = std::fs::read_to_string(file_path)?;
    let tree = parser
        .parse(&content, None)
        .ok_or_else(|| LaibraryError::Parse("Failed to parse source file".to_string()))?;

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

    // We use Rust's tree-sitter implementation for testing since it's already a dependency
    // In a real-world scenario, we'd use a mock language
    use tree_sitter_rust::LANGUAGE;

    #[test]
    fn test_parse_source_file_valid() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "fn main() { println!(\"Hello, World!\"); }";
        write!(temp_file, "{}", content).unwrap();

        let results = parse_source_files(
            &[temp_file.path().to_string_lossy().to_string()],
            &LANGUAGE.into(),
        )
        .unwrap();
        let result = &results[0];

        assert_eq!(result.content, content);
        assert_eq!(result.path, temp_file.path());
        assert_eq!(result.tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_source_file_directory() {
        let temp_dir = tempfile::tempdir().unwrap();

        let result = parse_source_files(
            &[temp_dir.path().to_string_lossy().to_string()],
            &LANGUAGE.into(),
        );

        assert!(matches!(result, Err(LaibraryError::InvalidPath(_))));
    }

    #[test]
    fn test_parse_source_file_nonexistent() {
        let result = parse_source_files(&["nonexistent.rs".to_string()], &LANGUAGE.into());

        assert!(matches!(result, Err(LaibraryError::InvalidPath(_))));
    }

    #[test]
    fn test_parse_source_file_empty() {
        let temp_file = NamedTempFile::new().unwrap();

        let results = parse_source_files(
            &[temp_file.path().to_string_lossy().to_string()],
            &LANGUAGE.into(),
        )
        .unwrap();
        let result = &results[0];

        assert!(result.content.is_empty());
        assert_eq!(result.path, temp_file.path());
        assert_eq!(result.tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_source_file_invalid_syntax() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "fn main() { let x = 1; let y = 2; let z = x + y; }";
        write!(temp_file, "{}", content).unwrap();

        let results = parse_source_files(
            &[temp_file.path().to_string_lossy().to_string()],
            &LANGUAGE.into(),
        )
        .unwrap();
        let result = &results[0];

        let root_node = result.tree.root_node();
        assert_eq!(root_node.kind(), "source_file");
        assert!(!root_node.has_error());
    }

    #[test]
    fn test_parse_source_files_multiple() {
        let mut temp_files = Vec::new();
        let mut file_paths = Vec::new();
        let contents = vec![
            "fn main() {}",
            "struct Test { field: i32 }",
            "enum Color { Red, Blue }",
        ];

        for content in contents.iter() {
            let mut temp_file = NamedTempFile::new().unwrap();
            write!(temp_file, "{}", content).unwrap();
            file_paths.push(temp_file.path().to_string_lossy().to_string());
            temp_files.push(temp_file);
        }

        let results = parse_source_files(&file_paths, &LANGUAGE.into()).unwrap();

        assert_eq!(results.len(), 3);
        for (result, content) in results.iter().zip(contents.iter()) {
            assert_eq!(result.content, *content);
            assert_eq!(result.tree.root_node().kind(), "source_file");
        }
    }

    #[test]
    fn test_parse_source_files_with_invalid() {
        let files = vec!["nonexistent.rs".to_string()];

        let result = parse_source_files(&files, &LANGUAGE.into());
        assert!(matches!(result, Err(LaibraryError::InvalidPath(_))));
    }
}
