use crate::error::LaibraryError;
use crate::types::{Namespace, PackageMetadata};

pub fn format_library_context(
    metadata: &PackageMetadata,
    namespaces: &[Namespace],
) -> Result<String, LaibraryError> {
    let api_content = format_namespaces_content(namespaces);

    Ok(format!(
        r#"<library name="{name}" version="{version}">
    <documentation>
{documentation}
    </documentation>
    <api>
{api_content}    </api>
</library>"#,
        name = metadata.name,
        version = metadata.version,
        documentation = metadata.documentation.trim()
    ))
}

fn format_namespaces_content(namespaces: &[Namespace]) -> String {
    namespaces
        .iter()
        .filter(|n| !n.symbols.is_empty())
        .map(format_namespace_content)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_namespace_content(namespace: &Namespace) -> String {
    let symbols_formatted = namespace
        .symbols
        .iter()
        .map(|s| s.source_code.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    format!("{}:\n\n```\n{}\n```", namespace.name, symbols_formatted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assertables::{assert_contains, assert_starts_with};

    const STUB_LIBRARY_NAME: &str = "test-lib";
    const STUB_LIBRARY_VERSION: &str = "1.0.0";

    fn assert_api_is_empty(documentation: &str) {
        let api_content = documentation
            .split("<api>")
            .nth(1)
            .and_then(|s| s.split("</api>").next())
            .unwrap_or("")
            .trim();
        assert!(
            api_content.is_empty(),
            "Expected empty API content, got: {api_content}"
        );
    }

    mod metadata {
        use super::*;

        #[test]
        fn library_name() {
            let metadata = PackageMetadata {
                name: STUB_LIBRARY_NAME.to_string(),
                version: STUB_LIBRARY_VERSION.to_string(),
                documentation: "".to_string(),
            };

            let documentation = format_library_context(&metadata, &[]).unwrap();

            assert_starts_with!(documentation, "<library name=\"test-lib\"");
        }

        #[test]
        fn library_version() {
            let metadata = PackageMetadata {
                name: STUB_LIBRARY_NAME.to_string(),
                version: STUB_LIBRARY_VERSION.to_string(),
                documentation: "".to_string(),
            };

            let documentation = format_library_context(&metadata, &[]).unwrap();

            assert_contains!(
                documentation,
                &format!(r#"version="{STUB_LIBRARY_VERSION}""#)
            );
        }
    }

    mod namespaces {
        use crate::types::Symbol;

        use super::*;

        const STUB_SOURCE_CODE: &str = "SOURCE_CODE";
        const STUB_MULTI_LINE_SOURCE_CODE: &str = "MULTI_LINE\nSOURCE_CODE";

        fn create_metadata() -> PackageMetadata {
            PackageMetadata {
                name: STUB_LIBRARY_NAME.to_string(),
                version: STUB_LIBRARY_VERSION.to_string(),
                documentation: "".to_string(),
            }
        }

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

        #[test]
        fn no_namespaces() {
            let documentation = format_library_context(&create_metadata(), &[]).unwrap();
            assert_api_is_empty(&documentation);
        }

        #[test]
        fn single_namespace() {
            let symbol = create_symbol("symbol", STUB_SOURCE_CODE);
            let namespace = create_namespace("test", vec![symbol]);
            let namespace_name = namespace.name.clone();

            let documentation = format_library_context(&create_metadata(), &[namespace]).unwrap();

            assert_contains!(
                documentation,
                &format!("{}:\n\n```\n{}\n```", namespace_name, STUB_SOURCE_CODE)
            );
        }

        #[test]
        fn multiple_namespaces() {
            let namespace1 =
                create_namespace("test1", vec![create_symbol("symbol1", STUB_SOURCE_CODE)]);
            let namespace2 =
                create_namespace("test2", vec![create_symbol("symbol2", STUB_SOURCE_CODE)]);

            let documentation =
                format_library_context(&create_metadata(), &[namespace1, namespace2]).unwrap();

            assert_contains!(documentation, "test1:\n\n```\n");
            assert_contains!(documentation, "test2:\n\n```\n");
        }

        mod symbols {
            use super::*;

            #[test]
            fn namespace_without_symbols() {
                let namespace = create_namespace("test", vec![]);
                let documentation =
                    format_library_context(&create_metadata(), &[namespace]).unwrap();
                assert_api_is_empty(&documentation);
            }

            #[test]
            fn single_symbol() {
                let namespace =
                    create_namespace("test", vec![create_symbol("symbol", STUB_SOURCE_CODE)]);

                let documentation =
                    format_library_context(&create_metadata(), &[namespace]).unwrap();

                assert_contains!(documentation, "```\n");
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
                    format_library_context(&create_metadata(), &[namespace]).unwrap();

                assert_contains!(documentation, "```\n");
                assert_contains!(documentation, "FIRST\n\nSECOND\n");
                assert_contains!(documentation, "```\n");
            }

            #[test]
            fn single_line_symbol() {
                let namespace =
                    create_namespace("test", vec![create_symbol("symbol", STUB_SOURCE_CODE)]);

                let documentation =
                    format_library_context(&create_metadata(), &[namespace]).unwrap();

                assert_contains!(documentation, "```\n");
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
                    format_library_context(&create_metadata(), &[namespace]).unwrap();

                assert_contains!(documentation, "```\n");
                assert_contains!(documentation, STUB_MULTI_LINE_SOURCE_CODE);
                assert_contains!(documentation, "\n```");
            }
        }
    }
}
