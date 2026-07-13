use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrototypeKind {
    Player,
    Door,
    Item,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityPrototype {
    pub id: String,
    pub kind: PrototypeKind,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
struct PrototypeFile {
    prototypes: Vec<EntityPrototype>,
}

#[derive(Debug, Clone)]
pub struct PrototypeCatalog {
    prototypes: HashMap<String, EntityPrototype>,
}

impl PrototypeCatalog {
    pub fn load() -> Result<Self> {
        let file: PrototypeFile =
            serde_json::from_str(include_str!("../../../content/prototypes.json"))
                .context("failed to parse entity prototypes")?;

        let prototypes = file
            .prototypes
            .into_iter()
            .map(|prototype| (prototype.id.clone(), prototype))
            .collect();

        Ok(Self { prototypes })
    }

    pub fn require(&self, id: &str) -> &EntityPrototype {
        self.prototypes
            .get(id)
            .unwrap_or_else(|| panic!("missing required prototype: {id}"))
    }
}
