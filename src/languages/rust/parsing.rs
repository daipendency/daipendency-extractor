use crate::error::LaibraryError;
use crate::types::SourceFile;
use std::fs;
use std::path::Path;

pub(super) fn parse_source(path: &Path) -> Result<Vec<SourceFile>, LaibraryError> {
    let src_path = path.join("src");
    if !src_path.exists() {
        return Err(LaibraryError::Parse(format!(
            "Source directory does not exist: {}",
            src_path.display()
        )));
    }

    collect_rs_files(&src_path)
}

fn collect_rs_files(dir: &Path) -> Result<Vec<SourceFile>, LaibraryError> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)
        .map_err(|e| LaibraryError::Parse(format!("Failed to read directory: {}", e)))?
    {
        let entry = entry
            .map_err(|e| LaibraryError::Parse(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_rs_files(&path)?);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = match fs::read_to_string(&path) {
                Ok(content) if !content.trim().is_empty() => content,
                Ok(_) => continue, // Skip empty files
                Err(e) => {
                    return Err(LaibraryError::Parse(format!(
                        "Failed to read source file {}: {}",
                        path.display(),
                        e
                    )))
                }
            };

            files.push(SourceFile { path, content });
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    mod valid_crates {
        use super::*;

        #[test]
        fn basic_crate_structure() {
            let temp_dir = TempDir::new().unwrap();
            create_test_crate(temp_dir.path(), &[
                ("lib.rs", r#"pub fn test_function() {}"#),
                ("nested/mod.rs", r#"pub fn nested_function() {}"#),
            ]).unwrap();

            let sources = parse_source(temp_dir.path()).unwrap();
            assert_eq!(sources.len(), 2);
            assert!(sources.iter().any(|s| s.path.ends_with("lib.rs")));
            assert!(sources.iter().any(|s| s.path.ends_with("mod.rs")));
        }

        #[test]
        fn multiple_nested_modules() {
            let temp_dir = TempDir::new().unwrap();
            create_test_crate(temp_dir.path(), &[
                ("lib.rs", ""),
                ("a/mod.rs", ""),
                ("a/b/mod.rs", ""),
                ("a/b/c.rs", ""),
            ]).unwrap();

            let sources = parse_source(temp_dir.path()).unwrap();
            assert_eq!(sources.len(), 4);
            assert!(sources.iter().any(|s| s.path.ends_with("c.rs")));
            assert!(sources.iter().any(|s| s.path.ends_with("a/mod.rs")));
            assert!(sources.iter().any(|s| s.path.ends_with("a/b/mod.rs")));
        }

        #[test]
        fn module_with_multiple_files() {
            let temp_dir = TempDir::new().unwrap();
            create_test_crate(temp_dir.path(), &[
                ("lib.rs", ""),
                ("module/mod.rs", ""),
                ("module/one.rs", ""),
                ("module/two.rs", ""),
            ]).unwrap();

            let sources = parse_source(temp_dir.path()).unwrap();
            assert_eq!(sources.len(), 4);
            assert!(sources.iter().any(|s| s.path.ends_with("one.rs")));
            assert!(sources.iter().any(|s| s.path.ends_with("two.rs")));
        }
    }

    mod invalid_crates {
        use super::*;

        #[test]
        fn missing_src_directory() {
            let temp_dir = TempDir::new().unwrap();
            let result = parse_source(temp_dir.path());
            assert!(matches!(result, Err(LaibraryError::Parse(_))));
            assert!(result.unwrap_err().to_string().contains("Source directory does not exist"));
        }

        #[test]
        fn empty_source_file() {
            let temp_dir = TempDir::new().unwrap();
            create_test_crate(temp_dir.path(), &[
                ("empty.rs", ""),
            ]).unwrap();

            let sources = parse_source(temp_dir.path()).unwrap();
            assert_eq!(sources.len(), 0);
        }

        #[test]
        fn invalid_utf8_content() {
            let temp_dir = TempDir::new().unwrap();
            let src_dir = temp_dir.path().join("src");
            fs::create_dir(&src_dir).unwrap();

            let mut file = File::create(src_dir.join("invalid.rs")).unwrap();
            file.write_all(&[0x80]).unwrap(); // Invalid UTF-8 byte

            let result = parse_source(temp_dir.path());
            assert!(matches!(result, Err(LaibraryError::Parse(_))));
            assert!(result.unwrap_err().to_string().contains("Failed to read source file"));
        }

        #[test]
        fn unreadable_source_file() {
            let temp_dir = TempDir::new().unwrap();
            create_test_crate(temp_dir.path(), &[
                ("lib.rs", "pub fn test() {}"),
            ]).unwrap();

            // Make the file unreadable
            let file_path = temp_dir.path().join("src").join("lib.rs");
            std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o000)).unwrap();

            let result = parse_source(temp_dir.path());
            assert!(matches!(result, Err(LaibraryError::Parse(_))));
            assert!(result.unwrap_err().to_string().contains("Failed to read source file"));

            // Restore permissions for cleanup
            std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn only_comments() {
            let temp_dir = TempDir::new().unwrap();
            create_test_crate(temp_dir.path(), &[
                ("lib.rs", "// Just a comment\n/* Block comment */"),
            ]).unwrap();

            let sources = parse_source(temp_dir.path()).unwrap();
            assert_eq!(sources.len(), 0);
        }

        #[test]
        fn only_whitespace() {
            let temp_dir = TempDir::new().unwrap();
            create_test_crate(temp_dir.path(), &[
                ("lib.rs", "   \n\t\n  "),
            ]).unwrap();

            let sources = parse_source(temp_dir.path()).unwrap();
            assert_eq!(sources.len(), 0);
        }

        #[test]
        fn non_rs_files() {
            let temp_dir = TempDir::new().unwrap();
            let src_dir = temp_dir.path().join("src");
            fs::create_dir(&src_dir).unwrap();
            fs::write(src_dir.join("file.txt"), "text content").unwrap();
            fs::write(src_dir.join("lib.rs"), "pub fn test() {}").unwrap();

            let sources = parse_source(temp_dir.path()).unwrap();
            assert_eq!(sources.len(), 1);
            assert!(sources[0].path.ends_with("lib.rs"));
        }
    }

    // Helper function to create test crate structure
    fn create_test_crate(root: &Path, files: &[(&str, &str)]) -> Result<(), std::io::Error> {
        let src_dir = root.join("src");
        fs::create_dir(&src_dir)?;

        for (file_path, content) in files {
            let full_path = src_dir.join(file_path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full_path, content)?;
        }

        Ok(())
    }
}
