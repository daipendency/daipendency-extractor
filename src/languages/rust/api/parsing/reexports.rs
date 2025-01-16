use super::files::RustSymbol;
use super::helpers::is_public;
use crate::error::LaibraryError;
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
    }

    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::api::parsing::test_helpers::make_tree;
    use crate::treesitter_test_helpers::find_child_node;

    fn get_symbol(name: &str, symbols: &[RustSymbol]) -> RustSymbol {
        symbols
            .iter()
            .find(|s| matches!(s, RustSymbol::SymbolReexport { name: n, .. } if n == name))
            .expect("Symbol not found")
            .clone()
    }

    #[test]
    fn private_module() {
        let source_code = r#"
use inner::Format;
"#;
        let tree = make_tree(source_code);
        let use_declaration = find_child_node(tree.root_node(), "use_declaration");

        let symbols = extract_reexports(&use_declaration, source_code).unwrap();

        assert!(symbols.is_empty());
    }

    #[test]
    fn single_item() {
        let source_code = r#"
pub use inner::Format;
"#;
        let tree = make_tree(source_code);
        let use_declaration = tree.root_node().child(0).unwrap();

        let symbols = extract_reexports(&use_declaration, source_code).unwrap();

        let symbol = get_symbol("Format", &symbols);
        assert!(
            matches!(symbol, RustSymbol::SymbolReexport { source_path, .. } if source_path == "inner::Format")
        );
    }

    #[test]
    fn multiple_items() {
        let source_code = r#"
pub use inner::{TextFormatter, OtherType};
"#;
        let tree = make_tree(source_code);
        let use_declaration = tree.root_node().child(0).unwrap();

        let symbols = extract_reexports(&use_declaration, source_code).unwrap();

        let formatter = get_symbol("TextFormatter", &symbols);
        assert!(
            matches!(formatter, RustSymbol::SymbolReexport { source_path, .. } if source_path == "inner::TextFormatter")
        );
        let other = get_symbol("OtherType", &symbols);
        assert!(
            matches!(other, RustSymbol::SymbolReexport { source_path, .. } if source_path == "inner::OtherType")
        );
    }
}
