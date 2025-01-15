use super::helpers::{extract_name, is_public};
use crate::error::LaibraryError;
use crate::languages::rust::api::types::RustSymbol;
use tree_sitter::Node;

pub fn extract_reexports(
    use_declaration_node: &Node,
    source_code: &str,
) -> Result<Vec<RustSymbol>, LaibraryError> {
    if !is_public(use_declaration_node) {
        return Ok(Vec::new());
    }

    let mut symbols = Vec::new();
    let mut cursor = use_declaration_node.walk();
    let children: Vec<_> = use_declaration_node.children(&mut cursor).collect();

    if let Some(scoped) = children.iter().find(|c| c.kind() == "scoped_identifier") {
        let mut scoped_cursor = scoped.walk();
        let scoped_children: Vec<_> = scoped.children(&mut scoped_cursor).collect();

        let mut path_parts = Vec::new();
        for scoped_child in &scoped_children {
            let text = scoped_child
                .utf8_text(source_code.as_bytes())
                .map_err(|e| LaibraryError::Parse(e.to_string()))?;
            path_parts.push(text);
        }

        if let Some(name) = path_parts.last() {
            let path = path_parts[..path_parts.len() - 1].join("");
            symbols.push(RustSymbol::SymbolReexport {
                name: name.to_string(),
                source_path: format!("{}{}", path, name),
            });
        }
    } else if let Some(scoped_list) = children.iter().find(|c| c.kind() == "scoped_use_list") {
        let mut scoped_cursor = scoped_list.walk();
        let scoped_children: Vec<_> = scoped_list.children(&mut scoped_cursor).collect();

        let path_prefix = if let Some(path) = scoped_children.first() {
            path.utf8_text(source_code.as_bytes())
                .map_err(|e| LaibraryError::Parse(e.to_string()))?
                .to_string()
        } else {
            String::new()
        };

        if let Some(use_list) = scoped_children.iter().find(|c| c.kind() == "use_list") {
            let mut list_cursor = use_list.walk();
            let list_items: Vec<_> = use_list.children(&mut list_cursor).collect();
            for list_item in list_items {
                if list_item.kind() == "identifier" {
                    let name = list_item
                        .utf8_text(source_code.as_bytes())
                        .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                    symbols.push(RustSymbol::SymbolReexport {
                        name: name.to_string(),
                        source_path: format!("{}::{}", path_prefix, name),
                    });
                }
            }
        }
    } else if let Some(tree) = children.iter().find(|c| c.kind() == "use_tree") {
        let mut tree_cursor = tree.walk();
        let tree_children: Vec<_> = tree.children(&mut tree_cursor).collect();

        if let Some(list) = tree_children.iter().find(|c| c.kind() == "use_tree_list") {
            let mut list_cursor = list.walk();
            for list_item in list.children(&mut list_cursor) {
                if list_item.kind() == "use_tree" {
                    if let Some(name_node) = list_item.child_by_field_name("name") {
                        let name = extract_name(&name_node, source_code)?;
                        let path_prefix = if let Some(path) = tree.child_by_field_name("path") {
                            let prefix = path
                                .utf8_text(source_code.as_bytes())
                                .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                            format!("{}::", prefix)
                        } else {
                            String::new()
                        };
                        symbols.push(RustSymbol::SymbolReexport {
                            name: name.clone(),
                            source_path: format!("{}{}", path_prefix, name),
                        });
                    }
                }
            }
        } else if let Some(name_node) = tree.child_by_field_name("name") {
            let name = extract_name(&name_node, source_code)?;
            let path_prefix = if let Some(path) = tree.child_by_field_name("path") {
                let prefix = path
                    .utf8_text(source_code.as_bytes())
                    .map_err(|e| LaibraryError::Parse(e.to_string()))?;
                format!("{}::", prefix)
            } else {
                String::new()
            };
            symbols.push(RustSymbol::SymbolReexport {
                name: name.clone(),
                source_path: format!("{}{}", path_prefix, name),
            });
        }
    }

    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::test_helpers::setup_parser;

    #[test]
    fn single_item() {
        let source_code = r#"
pub use inner::Format;
"#;
        let mut parser = setup_parser();

        let tree = parser.parse(source_code, None).unwrap();
        let use_declaration = tree.root_node().child(0).unwrap();
        let symbols = extract_reexports(&use_declaration, source_code).unwrap();

        assert_eq!(symbols.len(), 1);
        match &symbols[0] {
            RustSymbol::SymbolReexport {
                name, source_path, ..
            } => {
                assert_eq!(name, "Format");
                assert_eq!(source_path, "inner::Format");
            }
            _ => panic!("Expected SymbolReexport variant"),
        }
    }

    #[test]
    fn use_tree_list() {
        let source_code = r#"
pub use inner::{TextFormatter, OtherType};
"#;
        let mut parser = setup_parser();

        let tree = parser.parse(source_code, None).unwrap();
        let use_declaration = tree.root_node().child(0).unwrap();
        let symbols = extract_reexports(&use_declaration, source_code).unwrap();

        assert_eq!(symbols.len(), 2);
        let mut found_text_formatter = false;
        let mut found_other_type = false;

        for symbol in &symbols {
            match symbol {
                RustSymbol::SymbolReexport {
                    name, source_path, ..
                } => match name.as_str() {
                    "TextFormatter" => {
                        assert_eq!(source_path, "inner::TextFormatter");
                        found_text_formatter = true;
                    }
                    "OtherType" => {
                        assert_eq!(source_path, "inner::OtherType");
                        found_other_type = true;
                    }
                    _ => panic!("Unexpected symbol name"),
                },
                _ => panic!("Expected SymbolReexport variant"),
            }
        }

        assert!(found_text_formatter, "TextFormatter not found");
        assert!(found_other_type, "OtherType not found");
    }
}
