use crate::error::LaibraryError;
use crate::types::PackageMetadata;
use std::fs;
use std::path::Path;
use toml::Value;

const DEFAULT_LIB_PATH: &str = "src/lib.rs";
const README_PATH: &str = "README.md";

pub fn extract_metadata(path: &Path) -> Result<PackageMetadata, LaibraryError> {
    let cargo_toml_path = path.join("Cargo.toml");
    let cargo_toml_content = fs::read_to_string(&cargo_toml_path)
        .map_err(|e| LaibraryError::Parse(format!("Failed to read Cargo.toml: {}", e)))?;

    let cargo_toml_value: Value = cargo_toml_content
        .parse()
        .map_err(|e| LaibraryError::Parse(format!("Failed to parse Cargo.toml: {}", e)))?;

    let package = cargo_toml_value.get("package").ok_or_else(|| {
        LaibraryError::Parse("Missing [package] section in Cargo.toml".to_string())
    })?;

    let name = package
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| LaibraryError::Parse("Missing 'name' in [package] section".to_string()))?
        .to_string();

    let version = package
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| LaibraryError::Parse("Missing 'version' in [package] section".to_string()))?
        .to_string();

    // Read README.md for documentation - don't panic if missing
    let readme_path = path.join(README_PATH);
    let documentation = fs::read_to_string(&readme_path).unwrap_or_default();

    let entry_point = if let Some(lib) = cargo_toml_value.get("lib") {
        if let Some(path_str) = lib.get("path").and_then(Value::as_str) {
            path.join(Path::new(path_str))
        } else {
            path.join(DEFAULT_LIB_PATH)
        }
    } else {
        path.join(DEFAULT_LIB_PATH)
    };

    Ok(PackageMetadata {
        name,
        version,
        documentation,
        entry_point,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_crate(dir: &Path, custom_lib: Option<String>) -> Result<(), std::io::Error> {
        let mut cargo_toml = String::from(
            r#"
[package]
name = "test-crate"
version = "0.1.0"
"#,
        );

        if let Some(lib_path) = custom_lib {
            cargo_toml.push_str(&format!(
                r#"
[lib]
path = "{}"
"#,
                lib_path
            ));
        }

        fs::write(dir.join("Cargo.toml"), cargo_toml)?;

        fs::write(dir.join(README_PATH), "Test crate")?;

        Ok(())
    }

    #[test]
    fn test_extract_metadata_valid_crate() {
        let temp_dir = TempDir::new().unwrap();
        create_test_crate(temp_dir.path(), None).unwrap();

        let result = extract_metadata(temp_dir.path());
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.name, "test-crate");
        assert_eq!(metadata.version, "0.1.0");
        assert_eq!(metadata.documentation, "Test crate");
    }

    #[test]
    fn test_missing_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let result = extract_metadata(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_invalid_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "invalid toml content").unwrap();

        let result = extract_metadata(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_missing_package_section() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[dependencies]\nfoo = \"1.0\"",
        )
        .unwrap();

        let result = extract_metadata(temp_dir.path());
        assert!(matches!(result, Err(LaibraryError::Parse(_))));
    }

    #[test]
    fn test_missing_readme() {
        let temp_dir = TempDir::new().unwrap();
        create_test_crate(temp_dir.path(), None).unwrap();
        fs::remove_file(temp_dir.path().join(README_PATH)).unwrap();

        let result = extract_metadata(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().documentation, "");
    }

    mod entrypoint {
        use super::*;

        #[test]
        fn test_default_entry_point() {
            let temp_dir = TempDir::new().unwrap();

            create_test_crate(temp_dir.path(), None).unwrap();

            let metadata = extract_metadata(temp_dir.path()).unwrap();
            assert_eq!(metadata.entry_point, temp_dir.path().join(DEFAULT_LIB_PATH));
        }

        #[test]
        fn test_custom_entry_point() {
            let temp_dir = TempDir::new().unwrap();
            let custom_lib_path = "src/custom_lib.rs";

            create_test_crate(temp_dir.path(), Some(custom_lib_path.to_string())).unwrap();

            let metadata = extract_metadata(temp_dir.path()).unwrap();
            assert_eq!(metadata.entry_point, temp_dir.path().join(custom_lib_path));
        }
    }
}
