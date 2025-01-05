use crate::analysers::Analyser;
use crate::error::LaibraryError;
use crate::types::{Module, PackageMetadata};

pub fn format_library_context(
    metadata: &PackageMetadata,
    modules: &[Module],
    analyser: &dyn Analyser,
) -> Result<String, LaibraryError> {
    let mut api_content = String::new();

    for module in modules {
        api_content.push_str(&format!(
            "        <module name=\"{}\">\n{}\n        </module>\n",
            module.name,
            analyser.format_module(module)?
        ));
    }

    Ok(format!(
        r#"<library name="{name}" version="{version}">
    <documentation>
{documentation}
    </documentation>
    <api>
{api_content}
    </api>
</library>"#,
        name = metadata.name,
        version = metadata.version,
        documentation = metadata.documentation.trim()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Symbol;
    use tree_sitter::{Parser, Tree};

    struct TestAnalyser;

    impl Analyser for TestAnalyser {
        fn get_file_extensions(&self) -> Vec<String> {
            vec!["rs".to_string()]
        }

        fn get_parser_language(&self) -> tree_sitter::Language {
            tree_sitter_rust::LANGUAGE.into()
        }

        fn get_package_metadata(
            &self,
            _path: &std::path::Path,
        ) -> Result<PackageMetadata, LaibraryError> {
            unimplemented!()
        }

        fn extract_public_api<'a>(
            &self,
            _sources: &'a [crate::types::SourceFile],
        ) -> Result<Vec<Module<'a>>, LaibraryError> {
            unimplemented!()
        }

        fn format_module(&self, module: &Module) -> Result<String, LaibraryError> {
            let mut module_doc = String::new();
            for symbol in &module.symbols {
                if !module_doc.is_empty() {
                    module_doc.push_str("\n");
                }
                module_doc.push_str(&symbol.source_code);
            }
            Ok(module_doc)
        }
    }

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
    fn test_format_library_context() {
        let metadata = PackageMetadata {
            name: "test-lib".to_string(),
            version: "0.1.0".to_string(),
            documentation: "A test library.".to_string(),
        };

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();

        let content = r#"pub fn test() -> () {}
pub struct Test { field: String }
pub enum TestEnum { A, B }"#;
        let tree = parser.parse(content, None).unwrap();
        let test_module = create_test_module("test", content, &tree);

        let empty_content = "";
        let empty_tree = parser.parse(empty_content, None).unwrap();
        let empty_module = create_test_module("empty", empty_content, &empty_tree);

        let modules = vec![test_module, empty_module];
        let analyser = TestAnalyser;
        let documentation = format_library_context(&metadata, &modules, &analyser).unwrap();

        assert!(
            documentation.contains(r#"<library name="test-lib" version="0.1.0">"#),
            "Library tag not found"
        );
        assert!(
            documentation.contains("<documentation>"),
            "Documentation tag not found"
        );
        assert!(
            documentation.contains("A test library."),
            "Library documentation not found"
        );
        assert!(
            documentation.contains(r#"<module name="test">"#),
            "Module tag not found"
        );
        assert!(
            documentation.contains("pub fn test() -> () {}"),
            "Function not found"
        );
        assert!(
            documentation.contains("pub struct Test { field: String }"),
            "Struct not found"
        );
        assert!(
            documentation.contains("pub enum TestEnum { A, B }"),
            "Enum not found"
        );
        assert!(
            documentation.contains("</module>"),
            "Module closing tag not found"
        );
        assert!(
            documentation.contains("</library>"),
            "Library closing tag not found"
        );
    }

    #[test]
    fn test_format_library_context_empty() {
        let metadata = PackageMetadata {
            name: "empty-lib".to_string(),
            version: "0.1.0".to_string(),
            documentation: "An empty library.".to_string(),
        };

        let analyser = TestAnalyser;
        let documentation = format_library_context(&metadata, &[], &analyser).unwrap();

        assert!(
            documentation.contains(r#"<library name="empty-lib" version="0.1.0">"#),
            "Library tag not found"
        );
        assert!(
            documentation.contains("<documentation>"),
            "Documentation tag not found"
        );
        assert!(
            documentation.contains("An empty library."),
            "Library documentation not found"
        );
        assert!(
            !documentation.contains("<module"),
            "Unexpected module tag found"
        );
    }
}
