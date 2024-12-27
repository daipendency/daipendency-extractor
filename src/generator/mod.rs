use crate::types::LibraryInfo;

pub fn generate_output(info: &LibraryInfo) -> String {
    format!(
        r#"<library name="{name}" version="{version}">
    <documentation>
    <![CDATA[
{documentation}
    ]]>
    </documentation>
    <api>
    <![CDATA[
{api}
    ]]>
    </api>
</library>
"#,
        name = info.name,
        version = info.version,
        documentation = info.documentation.trim(),
        api = info.api.trim(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LibraryInfo;

    #[test]
    fn test_generate_output() {
        let info = LibraryInfo {
            name: "test-crate".to_string(),
            version: "0.1.0".to_string(),
            documentation: "Test documentation".to_string(),
            api: "pub fn test() {}".to_string(),
        };
        let output = generate_output(&info);
        assert!(
            output.contains("<![CDATA[\nTest documentation\n    ]]>"),
            "Output does not contain the expected documentation CDATA section"
        );
        assert!(
            output.contains("<![CDATA[\npub fn test() {}\n    ]]>"),
            "Output does not contain the expected API CDATA section"
        );
    }
}
