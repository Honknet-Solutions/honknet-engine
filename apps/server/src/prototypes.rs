use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use honknet_content::{
    ComponentDefinition, EntityPrototype as ContentPrototype, PrototypeRegistry,
};
use serde_yaml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrototypeKind {
    Player,
    Door,
    Item,
    Generic,
}

#[derive(Debug, Clone)]
pub struct EntityPrototype {
    pub definition: ContentPrototype,
    pub kind: PrototypeKind,
    pub display_name: String,
}

impl EntityPrototype {
    pub fn component(&self, component_type: &str) -> Option<&ComponentDefinition> {
        self.definition
            .components
            .iter()
            .find(|component| component.component_type == component_type)
    }

    pub fn has_component(&self, component_type: &str) -> bool {
        self.component(component_type).is_some()
    }
}

#[derive(Debug, Clone)]
pub struct PrototypeCatalog {
    prototypes: HashMap<String, EntityPrototype>,
}

impl PrototypeCatalog {
    pub fn load() -> Result<Self> {
        let root = configured_workspace_path(
            "HONKNET_PROTOTYPES",
            "examples/minimal-game/content/prototypes",
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
                        definition: prototype.clone(),
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

    pub fn iter(&self) -> impl Iterator<Item = (&str, &EntityPrototype)> {
        self.prototypes
            .iter()
            .map(|(id, prototype)| (id.as_str(), prototype))
    }
}

pub fn field<'a>(component: &'a ComponentDefinition, name: &str) -> Option<&'a Value> {
    component.fields.get(name)
}

pub fn field_bool(component: &ComponentDefinition, name: &str, fallback: bool) -> bool {
    field(component, name)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

pub fn field_f32(component: &ComponentDefinition, name: &str, fallback: f32) -> f32 {
    field(component, name)
        .and_then(|value| value.as_f64())
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

pub fn field_u32(component: &ComponentDefinition, name: &str, fallback: u32) -> u32 {
    field(component, name)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(fallback)
}

pub fn field_string(component: &ComponentDefinition, name: &str) -> Option<String> {
    field(component, name)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
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
