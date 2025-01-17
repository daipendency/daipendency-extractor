#![cfg(test)]

use crate::types::{Namespace, Symbol};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

pub fn create_temp_dir() -> TempDir {
    TempDir::new().unwrap()
}

pub fn get_namespace<'a>(name: &str, namespaces: &'a [Namespace]) -> Option<&'a Namespace> {
    namespaces.iter().find(|n| n.name == name)
}

pub fn stub_symbol_with_name(name: &str) -> Symbol {
    Symbol {
        name: name.to_string(),
        source_code: format!("pub fn {}() {{}}", name).to_string(),
    }
}

pub fn stub_symbol() -> Symbol {
    let name = "test";
    Symbol {
        name: name.to_string(),
        source_code: format!("pub fn {}() {{}}", name).to_string(),
    }
}

pub fn create_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut file = std::fs::File::create(path).unwrap();
    write!(file, "{}", content).unwrap();
}
