use super::types::{RustFile, RustSymbol};
use crate::error::LaibraryError;
use crate::types::Symbol;
use tree_sitter::{Node, Parser};

mod doc_comments;
mod helpers;
mod reexports;
mod test_helpers;

use doc_comments::{extract_inner_doc_comments, extract_outer_doc_comments};
use helpers::{extract_attributes, extract_name, get_declaration_list, is_public};
use reexports::extract_reexports;

pub fn parse_rust_file(content: &str, parser: &mut Parser) -> Result<RustFile, LaibraryError> {
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| LaibraryError::Parse("Failed to parse source file".to_string()))?;

    let doc_comment = extract_inner_doc_comments(&tree.root_node(), content)?;
    let symbols = extract_symbols_from_module(tree.root_node(), content)?;
    Ok(RustFile {
        doc_comment,
        symbols,
    })
}

fn extract_symbols_from_module(
    module_node: Node,
    source_code: &str,
) -> Result<Vec<RustSymbol>, LaibraryError> {
    let mut symbols = Vec::new();
    let mut cursor = module_node.walk();

    for child in module_node.children(&mut cursor) {
        match child.kind() {
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "macro_definition" => {
                if !is_public(&child) {
                    continue;
                }
                let name = extract_name(&child, source_code)?;
                symbols.push(RustSymbol::Symbol {
                    symbol: Symbol {
                        name,
                        source_code: get_symbol_source_code(child, source_code)?,
                    },
                });
            }
            "use_declaration" => {
                symbols.extend(extract_reexports(&child, source_code)?);
            }
            "mod_item" => {
                let inner_mod_name = extract_name(&child, source_code)?;
                let is_public = is_public(&child);

                if let Some(declaration_list) = get_declaration_list(child) {
                    // This is a module block (`mod foo { ... }`)
                    if is_public {
                        let doc_comment =
                            extract_inner_doc_comments(&declaration_list, source_code)?;
                        let inner_mod_symbols =
                            extract_symbols_from_module(declaration_list, source_code)?;
                        symbols.push(RustSymbol::Module {
                            name: inner_mod_name,
                            content: inner_mod_symbols,
                            doc_comment,
                        });
                    }
                } else {
                    // This is a module declaration (`mod foo;`)
                    symbols.push(RustSymbol::ModuleDeclaration {
                        name: inner_mod_name,
                        is_public,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(symbols)
}

fn get_symbol_source_code(node: Node, source_code: &str) -> Result<String, LaibraryError> {
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
            trait_source.push_str(" {\n");

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
    use assertables::assert_contains;

    fn get_inner_module<'a>(path: &str, symbols: &'a [RustSymbol]) -> Option<&'a [RustSymbol]> {
        let parts: Vec<&str> = path.split("::").collect();
        let mut current_symbols = symbols;

        for part in parts {
            match current_symbols.iter().find_map(|symbol| {
                if let RustSymbol::Module { name, content, .. } = symbol {
                    if name == part {
                        Some(content)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                Some(next_symbols) => current_symbols = next_symbols,
                None => return None,
            }
        }

        Some(current_symbols)
    }

    fn get_rust_symbol<'a>(symbols: &'a [RustSymbol], name: &str) -> Option<&'a Symbol> {
        symbols.iter().find_map(|s| {
            if let RustSymbol::Symbol { symbol, .. } = s {
                if symbol.name == name {
                    Some(symbol)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    #[test]
    fn empty_source_file() {
        let source_code = "";
        let mut parser = setup_parser();

        let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

        assert!(rust_file.symbols.is_empty());
    }

    #[test]
    fn invalid_syntax() {
        let source_code = "fn main() { let x = 1; let y = 2; let z = x + y; }";
        let mut parser = setup_parser();

        let result = parse_rust_file(source_code, &mut parser);

        assert!(result.is_ok());
    }

    #[test]
    fn reexports_multiple_symbols() {
        let source_code = r#"
pub use other::{One, Two};
"#;
        let mut parser = setup_parser();

        let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

        assert_eq!(rust_file.symbols.len(), 2);
        let symbol_names: Vec<_> = rust_file
            .symbols
            .iter()
            .map(|s| match s {
                RustSymbol::SymbolReexport { name, .. } => name.as_str(),
                _ => panic!("Expected SymbolReexport variant"),
            })
            .collect();
        assert!(symbol_names.contains(&"One"));
        assert!(symbol_names.contains(&"Two"));
    }

    mod function_body {
        use super::*;

        #[test]
        fn function_declaration() {
            let source_code = r#"
pub fn test_function() -> i32 {
    return 42;
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "test_function").unwrap();
            assert_eq!(symbol.source_code, "pub fn test_function() -> i32;");
        }

        #[test]
        fn trait_method_declaration() {
            let source_code = r#"
pub trait TestTrait {
    pub fn test_method(&self) -> i32 {
        42
    }
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = get_rust_symbol(&rust_file.symbols, "TestTrait").unwrap();
            assert_contains!(symbol.source_code, "pub fn test_method(&self) -> i32;");
        }
    }

    mod visibility {
        use super::*;

        #[test]
        fn private_symbols() {
            let source_code = r#"
fn private_function() {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(get_rust_symbol(&rust_file.symbols, "private_function").is_none());
        }

        #[test]
        fn public_symbols() {
            let source_code = r#"
pub fn public_function() {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(get_rust_symbol(&rust_file.symbols, "public_function").is_some());
        }
    }

    mod inner_modules {
        use super::*;

        #[test]
        fn module_without_inner_doc_comment() {
            let source_code = r#"
pub mod inner {
    pub fn nested_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let inner_module = match &rust_file.symbols[0] {
                RustSymbol::Module { name, content, .. } => {
                    assert_eq!(name, "inner");
                    content
                }
                _ => panic!("Expected Module variant"),
            };
            let symbol = get_rust_symbol(inner_module, "nested_function").unwrap();
            assert_eq!(symbol.name, "nested_function");
        }

        #[test]
        fn module_with_inner_doc_comment() {
            let source_code = r#"
pub mod inner {
    //! This is the inner doc comment
    //! It spans multiple lines

    pub fn nested_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let inner_module = match &rust_file.symbols[0] {
                RustSymbol::Module {
                    name,
                    content,
                    doc_comment,
                } => {
                    assert_eq!(name, "inner");
                    assert_eq!(
                        doc_comment,
                        &Some(
                            "//! This is the inner doc comment\n//! It spans multiple lines\n"
                                .to_string()
                        )
                    );
                    content
                }
                _ => panic!("Expected Module variant"),
            };
            let symbol = get_rust_symbol(inner_module, "nested_function").unwrap();
            assert_eq!(symbol.name, "nested_function");
        }

        #[test]
        fn public_modules() {
            let source_code = r#"
pub mod inner {
    pub fn nested_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let inner_content = get_inner_module("inner", &rust_file.symbols).unwrap();
            let symbol = get_rust_symbol(inner_content, "nested_function").unwrap();
            assert_eq!(symbol.name, "nested_function");
        }

        #[test]
        fn private_modules() {
            let source_code = r#"
mod private {
    pub fn private_function() -> String {}
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(
                rust_file.symbols.is_empty(),
                "Private modules should be ignored"
            );
        }

        #[test]
        fn empty_modules() {
            let source_code = r#"
pub mod empty {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let empty_content = get_inner_module("empty", &rust_file.symbols).unwrap();
            assert_eq!(rust_file.symbols.len(), 1);
            assert!(empty_content.is_empty());
        }

        #[test]
        fn inner_module_symbols() {
            let source_code = r#"
pub mod inner {
    pub struct InnerStruct {}

    pub mod deeper {
        pub enum DeeperEnum {
            A, B
        }
    }
}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(rust_file.symbols.len(), 1);
            let inner_content =
                get_inner_module("inner", &rust_file.symbols).expect("Should find inner module");
            assert_eq!(inner_content.len(), 2);
            let inner_struct =
                get_rust_symbol(inner_content, "InnerStruct").expect("Should find InnerStruct");
            assert_eq!(inner_struct.name, "InnerStruct");
            let deeper_content =
                get_inner_module("deeper", inner_content).expect("Should find deeper module");
            assert_eq!(deeper_content.len(), 1);
            let deeper_enum =
                get_rust_symbol(deeper_content, "DeeperEnum").expect("Should find DeeperEnum");
            assert_eq!(deeper_enum.name, "DeeperEnum");
        }

        #[test]
        fn module_declarations() {
            let source_code = r#"
pub mod other;
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            let symbol = match &rust_file.symbols[0] {
                RustSymbol::ModuleDeclaration { name, .. } => name,
                _ => panic!("Expected ModuleDeclaration variant"),
            };
            assert_eq!(symbol, "other");
        }
    }

    mod module_declarations {
        use super::*;

        #[test]
        fn public_module() {
            let source_code = r#"
pub mod test_module;
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(rust_file.symbols.len(), 1);
            match &rust_file.symbols[0] {
                RustSymbol::ModuleDeclaration { name, is_public } => {
                    assert_eq!(name, "test_module");
                    assert!(is_public);
                }
                _ => panic!("Expected ModuleDeclaration variant"),
            }
        }

        #[test]
        fn private_module() {
            let source_code = r#"
mod test_module;
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(rust_file.symbols.len(), 1);
            match &rust_file.symbols[0] {
                RustSymbol::ModuleDeclaration { name, is_public } => {
                    assert_eq!(name, "test_module");
                    assert!(!is_public);
                }
                _ => panic!("Expected ModuleDeclaration variant"),
            }
        }
    }

    mod doc_comments {
        use super::*;

        #[test]
        fn file_without_docs() {
            let source_code = r#"
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(rust_file.doc_comment.is_none());
            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
            assert_eq!(symbol.source_code, "pub struct Test {}");
        }

        #[test]
        fn file_with_docs() {
            let source_code = r#"
//! File-level documentation
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(
                rust_file.doc_comment,
                Some("//! File-level documentation\n".to_string())
            );
            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
            assert_eq!(symbol.source_code, "pub struct Test {}");
        }

        #[test]
        fn symbol_with_outer_doc_comment() {
            let source_code = r#"
/// Symbol documentation
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert!(rust_file.doc_comment.is_none());
            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
            assert_eq!(
                symbol.source_code,
                "/// Symbol documentation\npub struct Test {}"
            );
        }

        #[test]
        fn symbol_with_both_doc_comments() {
            let source_code = r#"
//! File-level documentation
/// Symbol documentation
pub struct Test {}
"#;
            let mut parser = setup_parser();

            let rust_file = parse_rust_file(source_code, &mut parser).unwrap();

            assert_eq!(
                rust_file.doc_comment,
                Some("//! File-level documentation\n".to_string())
            );
            let symbol = get_rust_symbol(&rust_file.symbols, "Test").unwrap();
            assert_eq!(
                symbol.source_code,
                "/// Symbol documentation\npub struct Test {}"
            );
        }
    }
}
