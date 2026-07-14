use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use honknet_content::{EntityPrototype as ContentPrototype, PrototypeRegistry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrototypeKind {
    Player,
    Door,
    Item,
    Generic,
}

#[derive(Debug, Clone)]
pub struct EntityPrototype {
    pub id: String,
    pub kind: PrototypeKind,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct PrototypeCatalog {
    prototypes: HashMap<String, EntityPrototype>,
}

impl PrototypeCatalog {
    pub fn load() -> Result<Self> {
        let root = std::env::var("HONKNET_PROTOTYPES")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("game/example-module/content/prototypes"));
        let registry = PrototypeRegistry::load_directory(&root)
            .with_context(|| format!("failed to load prototypes from {}", root.display()))?;

        let prototypes = registry
            .iter()
            .filter_map(|(_, prototype)| {
                if prototype.is_abstract {
                    return None;
                }
                Some((
                    prototype.id.clone(),
                    EntityPrototype {
                        id: prototype.id.clone(),
                        kind: infer_kind(prototype),
                        display_name: prototype
                            .name
                            .clone()
                            .unwrap_or_else(|| prototype.id.clone()),
                    },
                ))
            })
            .collect::<HashMap<_, _>>();

        Ok(Self { prototypes })
    }

    pub fn get(&self, id: &str) -> Option<&EntityPrototype> {
        self.prototypes.get(id)
    }

    pub fn require(&self, id: &str) -> &EntityPrototype {
        self.prototypes
            .get(id)
            .unwrap_or_else(|| panic!("missing required prototype: {id}"))
    }
}

fn infer_kind(prototype: &ContentPrototype) -> PrototypeKind {
    if has_component(prototype, "Player") {
        PrototypeKind::Player
    } else if has_component(prototype, "Door") {
        PrototypeKind::Door
    } else if has_component(prototype, "Item") {
        PrototypeKind::Item
    } else {
        PrototypeKind::Generic
    }
}

fn has_component(prototype: &ContentPrototype, component_type: &str) -> bool {
    prototype
        .components
        .iter()
        .any(|component| component.component_type == component_type)
}
