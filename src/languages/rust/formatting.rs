use crate::error::LaibraryError;
use crate::types::{Namespace, Symbol};

pub fn format_module(module: &Namespace) -> Result<String, LaibraryError> {
    let module_doc = module
        .symbols
        .iter()
        .map(|symbol| format_symbol(symbol))
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(module_doc)
}

fn format_symbol(symbol: &Symbol) -> String {
    let mut formatted = String::new();
    if let Some(doc) = &symbol.doc_comment {
        formatted.push_str(
            &doc.lines()
                .map(|line| format!("/// {}\n", line))
                .collect::<String>(),
        );
    }
    formatted.push_str(&symbol.source_code);
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Symbol;
    use tree_sitter::{Parser, Tree};

    fn create_test_module<'tree>(name: &str, content: &str, tree: &'tree Tree) -> Namespace<'tree> {
        let root_node = tree.root_node();
        let mut symbols = Vec::new();
        let mut cursor = root_node.walk();

        for node in root_node.children(&mut cursor) {
            if matches!(node.kind(), "function_item" | "struct_item" | "enum_item") {
                let mut name = String::new();
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "identifier" {
                        name = content[child.start_byte()..child.end_byte()].to_string();
                        break;
                    }
                }
                symbols.push(Symbol {
                    name: name.to_string(),
                    node,
                    source_code: node
                        .utf8_text(content.as_bytes())
                        .expect("Failed to get node text")
                        .to_string(),
                    doc_comment: None,
                });
            }
        }

        Namespace {
            name: name.to_string(),
            symbols,
        }
    }

    #[test]
    fn test_format_module() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();

        let content = r#"pub fn test() -> () {}
pub struct Test { field: String }
pub enum TestEnum { A, B }"#;
        let tree = parser.parse(content, None).unwrap();
        let test_module = create_test_module("test", content, &tree);

        let formatted = format_module(&test_module).unwrap();

        assert!(formatted.contains("pub fn test() -> () {}"));
        assert!(formatted.contains("pub struct Test { field: String }"));
        assert!(formatted.contains("pub enum TestEnum { A, B }"));
    }

    #[test]
    fn test_format_module_empty() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();

        let empty_content = "";
        let empty_tree = parser.parse(empty_content, None).unwrap();
        let empty_module = create_test_module("empty", empty_content, &empty_tree);

        let formatted = format_module(&empty_module).unwrap();
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_format_module_with_doc_comment() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();

        let content = "pub fn test() -> () {}";
        let tree = parser.parse(content, None).unwrap();
        let mut test_module = create_test_module("test", content, &tree);
        test_module.symbols[0].doc_comment = Some("This is a test function".to_string());

        let formatted = format_module(&test_module).unwrap();
        assert_eq!(
            formatted, "/// This is a test function\npub fn test() -> () {}",
            "Doc comment should be prepended with /// and a newline"
        );
    }

    #[test]
    fn test_format_module_with_multiline_doc_comment() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();

        let content = "pub fn test() -> () {}";
        let tree = parser.parse(content, None).unwrap();
        let mut test_module = create_test_module("test", content, &tree);
        test_module.symbols[0].doc_comment =
            Some("First line\nSecond line\nThird line".to_string());

        let formatted = format_module(&test_module).unwrap();
        assert_eq!(
            formatted, "/// First line\n/// Second line\n/// Third line\npub fn test() -> () {}",
            "Multi-line doc comments should have /// prefix for each line"
        );
    }
}
