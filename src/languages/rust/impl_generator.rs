use crate::error::LaibraryError;

pub(super) fn generate_documentation(public_members: &[String]) -> Result<String, LaibraryError> {
    Ok(public_members.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_documentation() {
        let public_members = vec![
            "pub fn test() -> ();".to_string(),
            "pub struct Test { field: String }".to_string(),
            "pub enum TestEnum { A, B }".to_string(),
        ];

        let documentation = generate_documentation(&public_members).unwrap();

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
        let documentation = generate_documentation(&[]).unwrap();
        assert!(documentation.is_empty());
    }

    #[test]
    fn test_generate_documentation_single_item() {
        let public_members = vec!["pub fn standalone() -> ();".to_string()];
        let documentation = generate_documentation(&public_members).unwrap();
        assert_eq!(documentation, "pub fn standalone() -> ();");
    }
}
