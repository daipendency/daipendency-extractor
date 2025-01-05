pub mod analyser;
pub mod extraction;
pub mod formatting;
pub mod metadata;
pub mod public_members;

use crate::types::{ApiRepresentation, Module};
use public_members::RustPublicMember;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct RustApi {
    pub modules: HashMap<String, Vec<RustPublicMember>>,
}

impl ApiRepresentation for RustApi {
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

pub use analyser::RustAnalyser;
