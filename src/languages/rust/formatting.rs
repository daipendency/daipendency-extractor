use crate::error::LaibraryError;
use crate::types::Module;

pub fn format_module(module: &Module) -> Result<String, LaibraryError> {
    let mut module_doc = String::new();
    for symbol in &module.symbols {
        if !module_doc.is_empty() {
            module_doc.push('\n');
        }
        module_doc.push_str(&symbol.source_code);
    }
    Ok(module_doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Symbol;
    use tree_sitter::{Parser, Tree};

    fn create_test_module<'tree>(name: &str, content: &str, tree: &'tree Tree) -> Module<'tree> {
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
                    name,
                    node,
                    source_code: node
                        .utf8_text(content.as_bytes())
                        .expect("Failed to get node text")
                        .to_string(),
                });
            }
        }

        Module {
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
}
