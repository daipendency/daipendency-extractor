use super::public_members::RustPublicMember;
use crate::error::LaibraryError;

pub fn format_documentation(public_members: &[RustPublicMember]) -> Result<String, LaibraryError> {
    Ok(public_members
        .iter()
        .map(|member| member.to_string())
        .collect::<Vec<_>>()
        .join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::rust::public_members::{Function, RustPublicMember};

    #[test]
    fn test_generate_documentation() {
        let public_members = vec![
            RustPublicMember::Function(Function {
                name: "test".to_string(),
                parameters: vec![],
                return_type: Some("()".to_string()),
                doc_comment: None,
                type_parameters: vec![],
                where_clause: None,
            }),
            RustPublicMember::from("pub struct Test { field: String }".to_string()),
            RustPublicMember::from("pub enum TestEnum { A, B }".to_string()),
        ];

        let documentation = format_documentation(&public_members).unwrap();

        assert!(documentation.contains("pub fn test() -> ();"));
        assert!(documentation.contains("pub struct Test { field: String }"));
        assert!(documentation.contains("pub enum TestEnum { A, B }"));

        // Verify items are separated by blank lines
        let lines: Vec<_> = documentation.lines().collect();
        assert_eq!(lines.len(), 5); // 3 items with 2 blank lines between them
        assert!(lines[1].is_empty());
        assert!(lines[3].is_empty());
    }

    #[test]
    fn test_generate_documentation_empty() {
        let documentation = format_documentation(&[]).unwrap();
        assert!(documentation.is_empty());
    }

    #[test]
    fn test_generate_documentation_single_item() {
        let public_members = vec![RustPublicMember::Function(Function {
            name: "standalone".to_string(),
            parameters: vec![],
            return_type: Some("()".to_string()),
            doc_comment: None,
            type_parameters: vec![],
            where_clause: None,
        })];
        let documentation = format_documentation(&public_members).unwrap();
        assert_eq!(documentation, "pub fn standalone() -> ();");
    }
}
