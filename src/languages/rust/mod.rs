pub mod public_members;
pub mod extraction;
pub mod formatting;
pub mod metadata;

use crate::types::{ApiRepresentation, Module, PackageMetadata, SourceFile};
use std::any::Any;
use public_members::RustPublicMember;
use std::collections::HashMap;
use std::path::Path;
use crate::analysers::LibraryAnalyser;
use crate::error::LaibraryError;

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
            .map(|(name, members)| {
                Module {
                    name: name.clone(),
                    public_members: members.iter()
                        .map(|m| Box::new(m.clone()) as Box<dyn std::fmt::Display>)
                        .collect(),
                }
            })
            .collect()
    }
}

pub struct RustAnalyser;

impl RustAnalyser {
    pub fn new() -> Self {
        RustAnalyser
    }
}

impl LibraryAnalyser for RustAnalyser {
    type Api = RustApi;

    fn extract_metadata(&self, path: &Path) -> Result<PackageMetadata, LaibraryError> {
        metadata::extract_metadata(path)
    }

    fn parse_source(&self, path: &Path) -> Result<Vec<SourceFile>, LaibraryError> {
        let mut sources = Vec::new();
        if path.is_file() {
            let content = std::fs::read_to_string(path)?;
            sources.push(SourceFile {
                path: path.to_path_buf(),
                content,
            });
        } else {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
            {
                let content = std::fs::read_to_string(entry.path())?;
                sources.push(SourceFile {
                    path: entry.path().to_path_buf(),
                    content,
                });
            }
        }
        Ok(sources)
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
