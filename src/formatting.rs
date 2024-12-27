use crate::error::LaibraryError;
use crate::types::PackageMetadata;

pub fn format_library_context(
    metadata: &PackageMetadata,
    api_content: &str,
) -> Result<String, LaibraryError> {
    Ok(format!(
        r#"<library name="{name}" version="{version}">
    <documentation>
    <![CDATA[
{documentation}
    ]]>
    </documentation>
    <api>
    <![CDATA[
{api_content}    ]]>
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

    #[test]
    fn test_generate_library_xml() {
        let metadata = PackageMetadata {
            name: "test-crate".to_string(),
            version: "0.1.0".to_string(),
            documentation: "Test documentation".to_string(),
        };
        let api_content = "pub fn test() {}";

        let output = format_library_context(&metadata, api_content).unwrap();

        assert!(output.contains("<library name=\"test-crate\" version=\"0.1.0\">"));
        assert!(output.contains("Test documentation"));
        assert!(output.contains("pub fn test() {}"));
    }
}
