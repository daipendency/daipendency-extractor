pub mod extraction;
pub mod formatting;
pub mod metadata;
pub mod public_members;

use crate::analysers::LibraryAnalyser;
use crate::error::LaibraryError;
use crate::types::{ApiRepresentation, Module, PackageMetadata, SourceFile};
use public_members::RustPublicMember;
use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::Language;
use tree_sitter_rust::LANGUAGE;

#[derive(Debug, Clone, PartialEq)]
pub struct RustApi {
    pub modules: HashMap<String, Vec<RustPublicMember>>,
}

impl ApiRepresentation for RustApi {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn modules(&self) -> Vec<Module<Box<dyn std::fmt::Display>>> {
        self.modules
            .iter()
            .map(|(name, members)| Module {
                name: name.clone(),
                public_members: members
                    .iter()
                    .map(|m| Box::new(m.clone()) as Box<dyn std::fmt::Display>)
                    .collect(),
            })
            .collect()
    }
}

pub struct RustAnalyser;

impl Default for RustAnalyser {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalyser {
    pub fn new() -> Self {
        RustAnalyser
    }
}

impl LibraryAnalyser for RustAnalyser {
    type Api = RustApi;

    fn get_parser_language(&self) -> Language {
        LANGUAGE.into()
    }

    fn get_extensions(&self) -> Vec<String> {
        vec!["rs".to_string()]
    }

    fn extract_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError> {
        metadata::extract_metadata(path)
    }

    fn extract_public_api(&self, sources: &[SourceFile]) -> Result<Self::Api, LaibraryError> {
        extraction::extract_public_api(sources)
    }

    fn format_documentation(&self, api: &Self::Api) -> Result<String, LaibraryError> {
        let mut all_members = Vec::new();
        for members in api.modules.values() {
            all_members.extend(members.iter().cloned());
        }
        formatting::format_documentation(&all_members)
    }
}

pub use extraction::extract_public_api;
pub use formatting::format_documentation;
pub use metadata::extract_metadata;
