use super::doc_comments::extract_outer_doc_comments;
use super::helpers::{extract_attributes, get_declaration_list};
use crate::error::LaibraryError;
use tree_sitter::Node;

pub fn get_symbol_source_code(node: Node, source_code: &str) -> Result<String, LaibraryError> {
    let mut source_code_with_docs = String::new();

    if let Some(doc_comment) = extract_outer_doc_comments(&node, source_code)? {
        source_code_with_docs.push_str(&doc_comment);
    }

    let attributes = extract_attributes(&node, source_code)?;
    if !attributes.is_empty() {
        let attributes_str = format!("{}\n", attributes.join("\n"));
        source_code_with_docs.push_str(&attributes_str);
    }

    let symbol_source = match node.kind() {
        "function_item" | "function_signature_item" => {
            let mut cursor = node.walk();
            let block_node = node
                .children(&mut cursor)
                .find(|n| n.kind() == "block")
                .ok_or_else(|| LaibraryError::Parse("Failed to find function block".to_string()))?;
            format!(
                "{};",
                &source_code[node.start_byte()..block_node.start_byte()].trim_end()
            )
        }
        "trait_item" => {
            let declaration_list = get_declaration_list(node).ok_or_else(|| {
                LaibraryError::Parse("Failed to find trait declaration list".to_string())
            })?;

            let mut trait_source = String::new();
            trait_source.push_str(&source_code[node.start_byte()..declaration_list.start_byte()]);
            trait_source.push_str("{\n");

            let mut method_cursor = declaration_list.walk();
            for method in declaration_list.children(&mut method_cursor) {
                if method.kind() == "function_item" {
                    let method_source = get_symbol_source_code(method, source_code)?;
                    for line in method_source.lines() {
                        trait_source.push_str("    ");
                        trait_source.push_str(line);
                        trait_source.push('\n');
                    }
                }
            }

            trait_source.push('}');
            trait_source
        }
        _ => node
            .utf8_text(source_code.as_bytes())
            .map(|s| s.to_string())
            .map_err(|e| LaibraryError::Parse(e.to_string()))?,
    };

    source_code_with_docs.push_str(&symbol_source);
    Ok(source_code_with_docs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::test_helpers::setup_parser;

    #[test]
    fn function_with_body() {
        let source_code = r#"pub fn test_function(x: i32) -> i32 {
            x + 42
        }"#;
        let mut parser = setup_parser();
        let tree = parser.parse(source_code, None).unwrap();
        let function_node = tree.root_node().child(0).unwrap();

        let result = get_symbol_source_code(function_node, source_code).unwrap();

        assert_eq!(result, "pub fn test_function(x: i32) -> i32;");
    }

    #[test]
    fn trait_with_method() {
        let source_code = r#"pub trait TestTrait {
            pub fn test_method(&self) -> i32 {
                42
            }
        }"#;
        let mut parser = setup_parser();
        let tree = parser.parse(source_code, None).unwrap();
        let trait_node = tree.root_node().child(0).unwrap();

        let result = get_symbol_source_code(trait_node, source_code).unwrap();

        assert_eq!(
            result,
            "pub trait TestTrait {\n    pub fn test_method(&self) -> i32;\n}"
        );
    }

    #[test]
    fn struct_with_fields() {
        let source_code = r#"pub struct TestStruct {
            field1: i32,
            field2: String,
        }"#;
        let mut parser = setup_parser();
        let tree = parser.parse(source_code, None).unwrap();
        let struct_node = tree.root_node().child(0).unwrap();

        let result = get_symbol_source_code(struct_node, source_code).unwrap();

        assert_eq!(result, source_code);
    }
}
