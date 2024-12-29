use crate::error::LaibraryError;
use crate::types::{Module, PackageMetadata};

pub fn format_library_context(
    metadata: &PackageMetadata,
    modules: &[Module],
) -> Result<String, LaibraryError> {
    let mut api_content = String::new();
    
    for module in modules {
        api_content.push_str(&format!(
            r#"        <module name="{name}">
{content}
        </module>
"#,
            name = module.name,
            content = module.public_members.join("\n")
        ));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_library_context() {
        let metadata = PackageMetadata {
            name: "test-crate".to_string(),
            version: "0.1.0".to_string(),
            documentation: "Test documentation".to_string(),
        };

        let modules = vec![
            Module {
                name: "test_module".to_string(),
                public_members: vec![
                    "pub fn test() -> ();".to_string(),
                    "pub struct Test { field: String }".to_string(),
                ],
            },
            Module {
                name: "another_module".to_string(),
                public_members: vec!["pub enum TestEnum { A, B }".to_string()],
            },
        ];

        let output = format_library_context(&metadata, &modules).unwrap();

        assert!(output.contains("<library name=\"test-crate\" version=\"0.1.0\">"));
        assert!(output.contains("Test documentation"));
        assert!(output.contains("<module name=\"test_module\">"));
        assert!(output.contains("pub fn test() -> ();"));
        assert!(output.contains("<module name=\"another_module\">"));
        assert!(output.contains("pub enum TestEnum { A, B }"));
    }

    #[test]
    fn test_format_empty_modules() {
        let metadata = PackageMetadata {
            name: "empty-crate".to_string(),
            version: "0.1.0".to_string(),
            documentation: "Empty crate".to_string(),
        };

        let output = format_library_context(&metadata, &[]).unwrap();

        assert!(output.contains("<library name=\"empty-crate\" version=\"0.1.0\">"));
        assert!(output.contains("Empty crate"));
        assert!(!output.contains("<module"));
    }
}
