mod impl_extractor;
mod impl_generator;
mod impl_metadata;
mod impl_parser;

use crate::analysers::LibraryAnalyser;
use crate::error::LaibraryError;
use crate::types::{ApiRepresentation, Module, PackageMetadata, SourceFile};
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct RustApi {
    modules: HashMap<String, Vec<String>>,
}

impl ApiRepresentation for RustApi {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn modules(&self) -> Vec<Module> {
        self.modules
            .iter()
            .map(|(name, members)| {
                let formatted_members = impl_generator::generate_documentation(members)
                    .unwrap_or_else(|_| members.join("\n"));
                Module {
                    name: name.clone(),
                    public_members: vec![formatted_members],
                }
            })
            .collect()
    }
}

pub struct RustAnalyser;

impl RustAnalyser {
    pub fn new() -> Self {
        Self
    }
}

impl LibraryAnalyser for RustAnalyser {
    type Api = RustApi;

    fn extract_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError> {
        impl_metadata::extract_metadata(path)
    }

    fn parse_source(&self, path: &Path) -> Result<Vec<SourceFile>, LaibraryError> {
        impl_parser::parse_source(path)
    }

    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<Self::Api, LaibraryError> {
        impl_extractor::extract_public_api(sources)
    }

    fn generate_documentation(&self, api: &Self::Api) -> Result<String, LaibraryError> {
        let modules = api.modules();
        Ok(modules
            .iter()
            .map(|module| module.public_members[0].clone())
            .collect::<Vec<_>>()
            .join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_api_modules() {
        let mut modules = HashMap::new();
        modules.insert(
            "test_module".to_string(),
            vec![
                "pub fn test() -> ();".to_string(),
                "pub struct Test { field: String }".to_string(),
            ],
        );
        modules.insert(
            "another_module".to_string(),
            vec!["pub enum TestEnum { A, B }".to_string()],
        );

        let api = RustApi { modules };
        let module_list = api.modules();

        assert_eq!(module_list.len(), 2);
        assert!(module_list.iter().any(|m| m.name == "test_module"));
        assert!(module_list.iter().any(|m| m.name == "another_module"));
        
        // Verify each module's members are properly formatted
        for module in module_list {
            assert_eq!(module.public_members.len(), 1);
            if module.name == "test_module" {
                assert!(module.public_members[0].contains("pub fn test()"));
                assert!(module.public_members[0].contains("pub struct Test"));
            } else {
                assert!(module.public_members[0].contains("pub enum TestEnum"));
            }
        }
    }

    #[test]
    fn test_empty_rust_api() {
        let api = RustApi {
            modules: HashMap::new(),
        };
        let module_list = api.modules();
        assert!(module_list.is_empty());
    }

    #[test]
    fn test_crate_module() {
        let mut modules = HashMap::new();
        modules.insert(
            "rust_crate::text".to_string(),
            vec![
                "pub fn text_function() -> ();".to_string(),
                "pub struct TextStruct { field: String }".to_string(),
            ],
        );

        let api = RustApi { modules };
        let module_list = api.modules();

        assert_eq!(module_list.len(), 1);
        let module = &module_list[0];
        assert_eq!(module.name, "rust_crate::text");
        let content = &module.public_members[0];
        
        // Verify module content is properly formatted
        assert!(content.contains("pub fn text_function() -> ();"));
        assert!(content.contains("pub struct TextStruct { field: String }"));
        
        // Verify items are separated by blank lines
        let lines: Vec<_> = content.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[1].is_empty());
    }
}
