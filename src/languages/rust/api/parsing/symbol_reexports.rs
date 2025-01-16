use super::files::RustSymbol;
use super::helpers::is_public;
use crate::error::LaibraryError;
use tree_sitter::Node;

pub fn extract_symbol_reexports(
    use_declaration_node: &Node,
    source_code: &str,
) -> Result<Vec<RustSymbol>, LaibraryError> {
    if !is_public(use_declaration_node) {
        return Ok(Vec::new());
    }

    let mut cursor = use_declaration_node.walk();
    let children: Vec<_> = use_declaration_node.children(&mut cursor).collect();

    if let Some(scoped) = children.iter().find(|c| c.kind() == "scoped_identifier") {
        Ok(vec![extract_single_reexport(scoped, source_code)?])
    } else if let Some(scoped_list) = children.iter().find(|c| c.kind() == "scoped_use_list") {
        extract_multi_reexports(scoped_list, source_code)
    } else {
        Err(LaibraryError::Parse(
            "Failed to find symbol reexport".to_string(),
        ))
    }
}

fn extract_single_reexport(scoped: &Node, source_code: &str) -> Result<RustSymbol, LaibraryError> {
    let mut scoped_cursor = scoped.walk();
    let scoped_children: Vec<_> = scoped.children(&mut scoped_cursor).collect();

    let mut path_parts = Vec::new();
    for scoped_child in &scoped_children {
        let text = scoped_child
            .utf8_text(source_code.as_bytes())
            .map_err(|e| LaibraryError::Parse(e.to_string()))?;
        path_parts.push(text);
    }

    let name = path_parts
        .last()
        .ok_or_else(|| LaibraryError::Parse("Empty path parts".to_string()))?;
    let path = path_parts[..path_parts.len() - 1].join("");
    Ok(RustSymbol::SymbolReexport {
        name: name.to_string(),
        source_path: format!("{}{}", path, name),
    })
}

fn extract_multi_reexports(
    scoped_list: &Node,
    source_code: &str,
) -> Result<Vec<RustSymbol>, LaibraryError> {
    let mut scoped_cursor = scoped_list.walk();
    let scoped_children: Vec<_> = scoped_list.children(&mut scoped_cursor).collect();

    let path_prefix = scoped_children
        .first()
        .ok_or_else(|| LaibraryError::Parse("Empty scoped list".to_string()))?
        .utf8_text(source_code.as_bytes())
        .map_err(|e| LaibraryError::Parse(e.to_string()))?
        .to_string();

    let use_list = scoped_children
        .iter()
        .find(|c| c.kind() == "use_list")
        .ok_or_else(|| LaibraryError::Parse("No use list found".to_string()))?;

    let mut list_cursor = use_list.walk();
    use_list
        .children(&mut list_cursor)
        .filter(|item| item.kind() == "identifier")
        .map(|item| {
            let name = item
                .utf8_text(source_code.as_bytes())
                .map_err(|e| LaibraryError::Parse(e.to_string()))?;
            Ok(RustSymbol::SymbolReexport {
                name: name.to_string(),
                source_path: format!("{}::{}", path_prefix, name),
            })
        })
        .collect()
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
    fn non_import() {
        let source_code = r#"
pub enum Format {}
"#;
        let tree = make_tree(source_code);
        let root_node = tree.root_node();
        let use_declaration = find_child_node(root_node, "enum_item");

        let result = extract_symbol_reexports(&use_declaration, source_code);

        assert!(matches!(
            result.unwrap_err(),
            LaibraryError::Parse(msg) if msg == "Failed to find symbol reexport"
        ));
    }

    #[test]
    fn import_without_reexport() {
        let source_code = r#"
use inner::Format;
"#;
        let tree = make_tree(source_code);
        let use_declaration = find_child_node(tree.root_node(), "use_declaration");

        let symbols = extract_symbol_reexports(&use_declaration, source_code).unwrap();

        assert!(symbols.is_empty());
    }

    #[test]
    fn single_reexport() {
        let source_code = r#"
pub use inner::Format;
"#;
        let tree = make_tree(source_code);
        let use_declaration = find_child_node(tree.root_node(), "use_declaration");

        let symbols = extract_symbol_reexports(&use_declaration, source_code).unwrap();

        let symbol = get_symbol("Format", &symbols);
        assert!(
            matches!(symbol, RustSymbol::SymbolReexport { source_path, .. } if source_path == "inner::Format")
        );
    }

    #[test]
    fn multiple_reexports() {
        let source_code = r#"
pub use inner::{TextFormatter, OtherType};
"#;
        let tree = make_tree(source_code);
        let use_declaration = find_child_node(tree.root_node(), "use_declaration");

        let symbols = extract_symbol_reexports(&use_declaration, source_code).unwrap();

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
