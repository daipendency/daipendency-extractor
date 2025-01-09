use crate::error::LaibraryError;
use crate::types::{Namespace, PackageMetadata};

pub fn format_library_context(
    metadata: &PackageMetadata,
    namespaces: &[Namespace],
    language: &str,
) -> Result<String, LaibraryError> {
    let api_content = format_namespaces_content(namespaces, language);

    Ok(format!(
        r#"---
library_name: {name}
library_version: {version}
---

{documentation}

# API

{api_content}"#,
        name = metadata.name,
        version = metadata.version,
        documentation = metadata.documentation.trim()
    ))
}

fn format_namespaces_content(namespaces: &[Namespace], language: &str) -> String {
    namespaces
        .iter()
        .filter(|n| !n.symbols.is_empty())
        .map(|n| format_namespace_content(n, language))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_namespace_content(namespace: &Namespace, language: &str) -> String {
    let symbols_formatted = namespace
        .symbols
        .iter()
        .map(|s| s.source_code.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "## {}\n\n```{}\n{}\n```\n",
        namespace.name, language, symbols_formatted
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use assertables::assert_contains;

    const STUB_LIBRARY_NAME: &str = "test-lib";
    const STUB_LIBRARY_VERSION: &str = "1.0.0";
    const STUB_DOCUMENTATION: &str = "Test documentation";
    const STUB_LANGUAGE: &str = "rust";

    fn create_metadata() -> PackageMetadata {
        PackageMetadata {
            name: STUB_LIBRARY_NAME.to_string(),
            version: STUB_LIBRARY_VERSION.to_string(),
            documentation: STUB_DOCUMENTATION.to_string(),
        }
    }

    mod metadata {
        use super::*;

        fn get_frontmatter_lines(documentation: String) -> Option<Vec<String>> {
            let mut lines = documentation.lines();
            (lines.next() == Some("---")).then(|| {
                lines
                    .take_while(|&line| line != "---")
                    .map(String::from)
                    .collect()
            })
        }

        #[test]
        fn library_name() {
            let metadata = create_metadata();

            let documentation = format_library_context(&metadata, &[], STUB_LANGUAGE).unwrap();
            let frontmatter_lines = get_frontmatter_lines(documentation).unwrap();

            assert_contains!(
                frontmatter_lines,
                &format!("library_name: {STUB_LIBRARY_NAME}")
            );
        }

        #[test]
        fn library_version() {
            let metadata = create_metadata();

            let documentation = format_library_context(&metadata, &[], STUB_LANGUAGE).unwrap();
            let frontmatter_lines = get_frontmatter_lines(documentation).unwrap();

            assert_contains!(
                frontmatter_lines,
                &format!("library_version: {STUB_LIBRARY_VERSION}")
            );
        }

        #[test]
        fn library_documentation() {
            let metadata = create_metadata();

            let documentation = format_library_context(&metadata, &[], STUB_LANGUAGE).unwrap();

            assert_contains!(
                documentation,
                &format!("\n---\n\n{STUB_DOCUMENTATION}\n\n# API")
            );
        }
    }

    mod api {
        use crate::types::Symbol;

        use super::*;

        const STUB_SOURCE_CODE: &str = "SOURCE_CODE";
        const STUB_MULTI_LINE_SOURCE_CODE: &str = "MULTI_LINE\nSOURCE_CODE";

        fn create_namespace(name: &str, symbols: Vec<Symbol>) -> Namespace {
            Namespace {
                name: name.to_string(),
                symbols,
            }
        }

        fn create_symbol(name: &str, source_code: &str) -> Symbol {
            Symbol {
                name: name.to_string(),
                source_code: source_code.to_string(),
            }
        }

        fn assert_api_is_empty(documentation: &str) {
            let api_content = documentation.split("\n# API\n").nth(1).unwrap_or("").trim();
            assert!(
                api_content.is_empty(),
                "Expected empty API content, got: {api_content}"
            );
        }

        #[test]
        fn no_namespaces() {
            let documentation =
                format_library_context(&create_metadata(), &[], STUB_LANGUAGE).unwrap();
            assert_api_is_empty(&documentation);
        }

        #[test]
        fn single_namespace() {
            let symbol = create_symbol("symbol", STUB_SOURCE_CODE);
            let namespace = create_namespace("test", vec![symbol]);
            let namespace_name = namespace.name.clone();

            let documentation =
                format_library_context(&create_metadata(), &[namespace], STUB_LANGUAGE).unwrap();

            assert_contains!(
                documentation,
                &format!(
                    "## {}\n\n```{}\n{}\n```",
                    namespace_name, STUB_LANGUAGE, STUB_SOURCE_CODE
                )
            );
        }

        #[test]
        fn multiple_namespaces() {
            let namespace1 =
                create_namespace("test1", vec![create_symbol("symbol1", STUB_SOURCE_CODE)]);
            let namespace2 =
                create_namespace("test2", vec![create_symbol("symbol2", STUB_SOURCE_CODE)]);

            let documentation = format_library_context(
                &create_metadata(),
                &[namespace1, namespace2],
                STUB_LANGUAGE,
            )
            .unwrap();

            assert_contains!(
                documentation,
                &format!("## test1\n\n```{}\n", STUB_LANGUAGE)
            );
            assert_contains!(
                documentation,
                &format!("## test2\n\n```{}\n", STUB_LANGUAGE)
            );
        }

        mod symbols {
            use super::*;

            #[test]
            fn namespace_without_symbols() {
                let namespace = create_namespace("test", vec![]);
                let documentation =
                    format_library_context(&create_metadata(), &[namespace], STUB_LANGUAGE)
                        .unwrap();
                assert_api_is_empty(&documentation);
            }

            #[test]
            fn single_symbol() {
                let namespace =
                    create_namespace("test", vec![create_symbol("symbol", STUB_SOURCE_CODE)]);

                let documentation =
                    format_library_context(&create_metadata(), &[namespace], STUB_LANGUAGE)
                        .unwrap();

                assert_contains!(documentation, &format!("```{}\n", STUB_LANGUAGE));
                assert_contains!(documentation, STUB_SOURCE_CODE);
                assert_contains!(documentation, "\n```");
            }

            #[test]
            fn multiple_symbols() {
                let namespace = create_namespace(
                    "test",
                    vec![
                        create_symbol("symbol1", "FIRST"),
                        create_symbol("symbol2", "SECOND"),
                    ],
                );

                let documentation =
                    format_library_context(&create_metadata(), &[namespace], STUB_LANGUAGE)
                        .unwrap();

                assert_contains!(documentation, &format!("```{}\n", STUB_LANGUAGE));
                assert_contains!(documentation, "FIRST\n\nSECOND\n");
                assert_contains!(documentation, "```\n");
            }

            #[test]
            fn single_line_symbol() {
                let namespace =
                    create_namespace("test", vec![create_symbol("symbol", STUB_SOURCE_CODE)]);

                let documentation =
                    format_library_context(&create_metadata(), &[namespace], STUB_LANGUAGE)
                        .unwrap();

                assert_contains!(documentation, &format!("```{}\n", STUB_LANGUAGE));
                assert_contains!(documentation, STUB_SOURCE_CODE);
                assert_contains!(documentation, "\n```");
            }

            #[test]
            fn multi_line_symbol() {
                let namespace = create_namespace(
                    "test",
                    vec![create_symbol("symbol", STUB_MULTI_LINE_SOURCE_CODE)],
                );

                let documentation =
                    format_library_context(&create_metadata(), &[namespace], STUB_LANGUAGE)
                        .unwrap();

                assert_contains!(documentation, &format!("```{}\n", STUB_LANGUAGE));
                assert_contains!(documentation, STUB_MULTI_LINE_SOURCE_CODE);
                assert_contains!(documentation, "\n```");
            }
        }
    }
}
