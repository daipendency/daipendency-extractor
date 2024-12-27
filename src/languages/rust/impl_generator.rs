use crate::error::LaibraryError;
use crate::types::{ApiDefinitions, PackageMetadata};

pub(super) fn generate_documentation(
    metadata: &PackageMetadata,
    api: &ApiDefinitions,
) -> Result<String, LaibraryError> {
    let mut output = format!(
        r#"<library name="{name}" version="{version}">
    <documentation>
    <![CDATA[
{documentation}
    ]]>
    </documentation>
    <api>
    <![CDATA[
"#,
        name = metadata.name,
        version = metadata.version,
        documentation = metadata.documentation.trim()
    );

    // Add functions
    if !api.functions.is_empty() {
        output.push_str("\n// Functions\n");
        for function in &api.functions {
            output.push_str(function);
            output.push('\n');
        }
    }

    // Add structs
    if !api.structs.is_empty() {
        output.push_str("\n// Structs\n");
        for struct_def in &api.structs {
            output.push_str(struct_def);
            output.push('\n');
        }
    }

    // Add enums
    if !api.enums.is_empty() {
        output.push_str("\n// Enums\n");
        for enum_def in &api.enums {
            output.push_str(enum_def);
            output.push('\n');
        }
    }

    // Add traits
    if !api.traits.is_empty() {
        output.push_str("\n// Traits\n");
        for trait_def in &api.traits {
            output.push_str(trait_def);
            output.push('\n');
        }
    }

    output.push_str(
        r#"    ]]>
    </api>
</library>"#,
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_documentation() {
        let metadata = PackageMetadata {
            name: "test-crate".to_string(),
            version: "0.1.0".to_string(),
            documentation: "Test documentation".to_string(),
        };

        let api = ApiDefinitions {
            functions: vec!["pub fn test_function() -> ();".to_string()],
            structs: vec!["pub struct TestStruct { pub field: String }".to_string()],
            enums: vec!["pub enum TestEnum { Variant1, Variant2 }".to_string()],
            traits: vec!["pub trait TestTrait { fn method(&self); }".to_string()],
        };

        let output = generate_documentation(&metadata, &api).unwrap();

        // Check metadata
        assert!(output.contains("<library name=\"test-crate\" version=\"0.1.0\">"));
        assert!(output.contains("Test documentation"));

        // Check API sections
        assert!(output.contains("// Functions"));
        assert!(output.contains("pub fn test_function() -> ();"));
        assert!(output.contains("// Structs"));
        assert!(output.contains("pub struct TestStruct"));
        assert!(output.contains("// Enums"));
        assert!(output.contains("pub enum TestEnum"));
        assert!(output.contains("// Traits"));
        assert!(output.contains("pub trait TestTrait"));
    }

    #[test]
    fn test_generate_documentation_empty_api() {
        let metadata = PackageMetadata {
            name: "empty-crate".to_string(),
            version: "0.1.0".to_string(),
            documentation: "Empty crate".to_string(),
        };

        let api = ApiDefinitions {
            functions: vec![],
            structs: vec![],
            enums: vec![],
            traits: vec![],
        };

        let output = generate_documentation(&metadata, &api).unwrap();

        assert!(output.contains("<library name=\"empty-crate\" version=\"0.1.0\">"));
        assert!(output.contains("Empty crate"));
        assert!(!output.contains("// Functions"));
        assert!(!output.contains("// Structs"));
        assert!(!output.contains("// Enums"));
        assert!(!output.contains("// Traits"));
    }
}
