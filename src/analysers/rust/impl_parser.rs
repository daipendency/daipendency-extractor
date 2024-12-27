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

    fn create_test_source(dir: &Path) -> Result<(), std::io::Error> {
        let src_dir = dir.join("src");
        fs::create_dir(&src_dir)?;

        // Create lib.rs
        fs::write(
            src_dir.join("lib.rs"),
            r#"
pub struct TestStruct {
    field: String,
}

pub fn test_function() {}
"#,
        )?;

        // Create a nested module
        let nested_dir = src_dir.join("nested");
        fs::create_dir(&nested_dir)?;
        fs::write(
            nested_dir.join("mod.rs"),
            r#"
pub fn nested_function() {}
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_parse_source_valid_crate() {
        let temp_dir = TempDir::new().unwrap();
        create_test_source(temp_dir.path()).unwrap();

        let result = parse_source(temp_dir.path());
        assert!(result.is_ok());

        let sources = result.unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|s| s.path.ends_with("lib.rs")));
        assert!(sources.iter().any(|s| s.path.ends_with("mod.rs")));
    }

    #[test]
    fn test_missing_src_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = parse_source(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_empty_source_file() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("empty.rs"), "").unwrap();

        let result = parse_source(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_invalid_utf8_in_source() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Write an invalid UTF-8 sequence (0x80 is an invalid start byte)
        let mut file = File::create(src_dir.join("invalid.rs")).unwrap();
        file.write_all(&[0x80]).unwrap();

        let result = parse_source(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }
}
