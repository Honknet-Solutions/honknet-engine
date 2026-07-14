use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

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
    pub kind: PrototypeKind,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct PrototypeCatalog {
    prototypes: HashMap<String, EntityPrototype>,
}

impl PrototypeCatalog {
    pub fn load() -> Result<Self> {
        let root = configured_workspace_path(
            "HONKNET_PROTOTYPES",
            "game/example-module/content/prototypes",
        );

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

fn configured_workspace_path(
    environment_variable: &str,
    default_relative_path: impl AsRef<Path>,
) -> PathBuf {
    match env::var_os(environment_variable) {
        Some(value) => {
            let configured_path = PathBuf::from(value);

            if configured_path.is_absolute() {
                configured_path
            } else {
                workspace_root().join(configured_path)
            }
        }

        None => workspace_root().join(default_relative_path),
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}
