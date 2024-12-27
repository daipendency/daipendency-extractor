use super::ApiDefinitions;
use crate::error::LaibraryError;

pub(super) fn generate_documentation(api: &ApiDefinitions) -> Result<String, LaibraryError> {
    let mut api_content = String::new();

    // Add functions
    if !api.functions.is_empty() {
        api_content.push_str("\n// Functions\n");
        for function in &api.functions {
            api_content.push_str(function);
            api_content.push('\n');
        }
    }

    // Add structs
    if !api.structs.is_empty() {
        api_content.push_str("\n// Structs\n");
        for struct_def in &api.structs {
            api_content.push_str(struct_def);
            api_content.push('\n');
        }
    }

    // Add enums
    if !api.enums.is_empty() {
        api_content.push_str("\n// Enums\n");
        for enum_def in &api.enums {
            api_content.push_str(enum_def);
            api_content.push('\n');
        }
    }

    // Add traits
    if !api.traits.is_empty() {
        api_content.push_str("\n// Traits\n");
        for trait_def in &api.traits {
            api_content.push_str(trait_def);
            api_content.push('\n');
        }
    }

    Ok(api_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_documentation() {
        let api = ApiDefinitions {
            functions: vec!["pub fn test_function() -> ();".to_string()],
            structs: vec!["pub struct TestStruct { pub field: String }".to_string()],
            enums: vec!["pub enum TestEnum { Variant1, Variant2 }".to_string()],
            traits: vec!["pub trait TestTrait { fn method(&self); }".to_string()],
        };

        let output = generate_documentation(&api).unwrap();

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
        let api = ApiDefinitions {
            functions: vec![],
            structs: vec![],
            enums: vec![],
            traits: vec![],
        };

        let output = generate_documentation(&api).unwrap();

        assert!(!output.contains("// Functions"));
        assert!(!output.contains("// Structs"));
        assert!(!output.contains("// Enums"));
        assert!(!output.contains("// Traits"));
    }
}
