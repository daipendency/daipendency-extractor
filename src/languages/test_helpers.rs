#![cfg(test)]

use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

pub fn create_temp_dir() -> TempDir {
    TempDir::new().unwrap()
}

pub fn create_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut file = std::fs::File::create(path).unwrap();
    write!(file, "{}", content).unwrap();
}

pub fn create_temp_file(content: &str) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", content).unwrap();
    temp_file
}
