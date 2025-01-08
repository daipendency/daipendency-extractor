use crate::error::LaibraryError;
use crate::types::{Namespace, Symbol};
use std::fmt::Write;

pub fn format_module(module: &Namespace) -> Result<String, LaibraryError> {
    let module_doc = module
        .symbols
        .iter()
        .map(format_symbol)
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(module_doc)
}

fn format_symbol(symbol: &Symbol) -> String {
    let mut formatted = String::new();
    if let Some(doc) = &symbol.doc_comment {
        doc.lines().fold(&mut formatted, |acc, line| {
            let _ = writeln!(acc, "/// {}", line);
            acc
        });
    }
    formatted.push_str(&symbol.source_code);
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_symbol(name: &str, source_code: &str, doc_comment: Option<&str>) -> Symbol {
        Symbol {
            name: name.to_string(),
            source_code: source_code.to_string(),
            doc_comment: doc_comment.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_format_module() {
        let module = Namespace {
            name: "test".to_string(),
            symbols: vec![
                create_symbol("test", "pub fn test() -> () {}", None),
                create_symbol("TestEnum", "pub enum TestEnum { A, B }", None),
            ],
        };

        let formatted = format_module(&module).unwrap();

        assert_eq!(
            formatted, "pub fn test() -> () {}\n\npub enum TestEnum { A, B }",
            "Module should format all symbols with double newlines between them"
        );
    }

    #[test]
    fn test_format_module_empty() {
        let module = Namespace {
            name: "empty".to_string(),
            symbols: vec![],
        };

        let formatted = format_module(&module).unwrap();
        assert!(
            formatted.is_empty(),
            "Empty module should produce empty string"
        );
    }

    #[test]
    fn test_format_module_with_doc_comment() {
        let module = Namespace {
            name: "test".to_string(),
            symbols: vec![create_symbol(
                "test",
                "pub fn test() -> () {}",
                Some("This is a test function"),
            )],
        };

        let formatted = format_module(&module).unwrap();
        assert_eq!(
            formatted, "/// This is a test function\npub fn test() -> () {}",
            "Doc comment should be prepended with /// and a newline"
        );
    }

    #[test]
    fn test_format_module_with_multiline_doc_comment() {
        let module = Namespace {
            name: "test".to_string(),
            symbols: vec![create_symbol(
                "test",
                "pub fn test() -> () {}",
                Some("First line\nSecond line"),
            )],
        };

        let formatted = format_module(&module).unwrap();
        assert_eq!(
            formatted, "/// First line\n/// Second line\npub fn test() -> () {}",
            "Multi-line doc comments should have /// prefix for each line"
        );
    }
}
