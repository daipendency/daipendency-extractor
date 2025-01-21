use crate::error::LaibraryError;
use std::path::Path;
use walkdir::WalkDir;

pub fn get_source_file_paths(
    directory_path: String,
    extensions: Vec<String>,
) -> Result<Vec<String>, LaibraryError> {
    let path = Path::new(&directory_path);
    if path.is_file() {
        return Err(LaibraryError::InvalidPath(
            "Expected a directory".to_string(),
        ));
    }

    let paths: Vec<String> = WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map_or(false, |ext| extensions.iter().any(|e| e == ext))
        })
        .filter_map(|e| e.path().to_str().map(String::from))
        .collect();

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn get_source_file_paths_errors_on_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let result = get_source_file_paths(
            file_path.to_str().unwrap().to_string(),
            vec!["txt".to_string()],
        );

        assert!(matches!(result, Err(LaibraryError::InvalidPath(_))));
    }

    #[test]
    fn get_source_file_paths_finds_matching_extensions() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        File::create(temp_dir.path().join("test1.rs")).unwrap();
        File::create(temp_dir.path().join("test2.rs")).unwrap();
        File::create(temp_dir.path().join("test.txt")).unwrap();

        // Create a subdirectory with more files
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        File::create(sub_dir.join("test3.rs")).unwrap();

        let paths = get_source_file_paths(
            temp_dir.path().to_str().unwrap().to_string(),
            vec!["rs".to_string()],
        )
        .unwrap();

        assert_eq!(paths.len(), 3);
        assert!(paths.iter().all(|p| p.ends_with(".rs")));
    }

    #[test]
    fn get_source_file_paths_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let paths = get_source_file_paths(
            temp_dir.path().to_str().unwrap().to_string(),
            vec!["rs".to_string()],
        )
        .unwrap();

        assert!(paths.is_empty());
    }

    #[test]
    fn get_source_file_paths_multiple_extensions() {
        let temp_dir = TempDir::new().unwrap();

        File::create(temp_dir.path().join("test1.rs")).unwrap();
        File::create(temp_dir.path().join("test2.txt")).unwrap();

        let paths = get_source_file_paths(
            temp_dir.path().to_str().unwrap().to_string(),
            vec!["rs".to_string(), "txt".to_string()],
        )
        .unwrap();

        assert_eq!(paths.len(), 2);
    }
}
