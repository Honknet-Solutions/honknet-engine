use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_yaml::Error,
    },
    #[error("duplicate prototype id {0}")]
    DuplicatePrototype(String),
    #[error("prototype {prototype} references missing parent {parent}")]
    MissingParent { prototype: String, parent: String },
    #[error("inheritance cycle detected at prototype {0}")]
    InheritanceCycle(String),
    #[error("duplicate component schema id {0}")]
    DuplicateComponentSchema(String),
    #[error("unknown component schema {0}")]
    UnknownComponentSchema(String),
    #[error("invalid component state for {component}: {message}")]
    InvalidComponentState { component: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentDefinition {
    #[serde(rename = "type")]
    pub component_type: String,
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityPrototype {
    #[serde(rename = "type", default = "entity_type")]
    pub document_type: String,
    pub id: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default, rename = "abstract")]
    pub is_abstract: bool,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub components: Vec<ComponentDefinition>,
}

fn entity_type() -> String {
    "entity".to_owned()
}

#[derive(Debug, Clone, Default)]
pub struct PrototypeRegistry {
    raw: HashMap<String, EntityPrototype>,
    resolved: HashMap<String, EntityPrototype>,
}

impl PrototypeRegistry {
    pub fn load_directory(path: impl AsRef<Path>) -> Result<Self, ContentError> {
        let mut registry = Self::default();
        for file in yaml_files(path.as_ref()) {
            let text = fs::read_to_string(&file).map_err(|source| ContentError::Read {
                path: file.clone(),
                source,
            })?;
            let documents: Vec<EntityPrototype> =
                serde_yaml::from_str(&text).map_err(|source| ContentError::Parse {
                    path: file.clone(),
                    source,
                })?;
            for prototype in documents {
                if prototype.document_type != "entity" {
                    continue;
                }
                let id = prototype.id.clone();
                if registry.raw.contains_key(&id) {
                    return Err(ContentError::DuplicatePrototype(id));
                }
                let _ = registry.raw.insert(id, prototype);
            }
        }
        registry.resolve_all()?;
        Ok(registry)
    }

    pub fn insert(&mut self, prototype: EntityPrototype) -> Result<(), ContentError> {
        let id = prototype.id.clone();
        if self.raw.contains_key(&id) {
            return Err(ContentError::DuplicatePrototype(id));
        }
        let _ = self.raw.insert(id, prototype);
        self.resolve_all()
    }

    pub fn get(&self, id: &str) -> Option<&EntityPrototype> {
        self.resolved.get(id)
    }

    pub fn require(&self, id: &str) -> &EntityPrototype {
        self.get(id)
            .unwrap_or_else(|| panic!("missing required prototype: {id}"))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &EntityPrototype)> {
        self.resolved.iter()
    }

    fn resolve_all(&mut self) -> Result<(), ContentError> {
        self.resolved.clear();
        let ids = self.raw.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            let mut visiting = HashSet::new();
            let resolved = self.resolve_one(&id, &mut visiting)?;
            let _ = self.resolved.insert(id, resolved);
        }
        Ok(())
    }

    fn resolve_one(
        &self,
        id: &str,
        visiting: &mut HashSet<String>,
    ) -> Result<EntityPrototype, ContentError> {
        if !visiting.insert(id.to_owned()) {
            return Err(ContentError::InheritanceCycle(id.to_owned()));
        }
        let prototype = self.raw.get(id).expect("known prototype id");
        let mut result = if let Some(parent_id) = &prototype.parent {
            let Some(_) = self.raw.get(parent_id) else {
                return Err(ContentError::MissingParent {
                    prototype: id.to_owned(),
                    parent: parent_id.clone(),
                });
            };
            self.resolve_one(parent_id, visiting)?
        } else {
            prototype.clone()
        };

        result.id = prototype.id.clone();
        result.parent = prototype.parent.clone();
        result.is_abstract = prototype.is_abstract;
        if prototype.name.is_some() {
            result.name = prototype.name.clone();
        }
        if prototype.description.is_some() {
            result.description = prototype.description.clone();
        }
        if !prototype.categories.is_empty() {
            result.categories = prototype.categories.clone();
        }
        if !prototype.tags.is_empty() {
            result.tags = prototype.tags.clone();
        }
        merge_components(&mut result.components, &prototype.components);
        visiting.remove(id);
        Ok(result)
    }
}

fn merge_components(target: &mut Vec<ComponentDefinition>, overlay: &[ComponentDefinition]) {
    for component in overlay {
        if let Some(existing) = target
            .iter_mut()
            .find(|candidate| candidate.component_type == component.component_type)
        {
            for (key, value) in &component.fields {
                let _ = existing.fields.insert(key.clone(), value.clone());
            }
        } else {
            target.push(component.clone());
        }
    }
}

fn yaml_files(root: &Path) -> Vec<PathBuf> {
    let mut files = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("yml" | "yaml")
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentSchema {
    #[serde(rename = "type")]
    pub document_type: String,
    pub id: String,
    #[serde(default)]
    pub replication: ReplicationSchema,
    #[serde(default)]
    pub fields: BTreeMap<String, FieldSchema>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ReplicationSchema {
    #[serde(default)]
    pub mode: ReplicationMode,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ReplicationMode {
    #[default]
    None,
    ServerToClient,
    OwnerOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldSchema {
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub default: Value,
    #[serde(default)]
    pub minimum: Option<f64>,
    #[serde(default)]
    pub maximum: Option<f64>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ComponentSchemaRegistry {
    schemas: HashMap<String, ComponentSchema>,
}

impl ComponentSchemaRegistry {
    pub fn load_directory(path: impl AsRef<Path>) -> Result<Self, ContentError> {
        let mut registry = Self::default();
        for file in yaml_files(path.as_ref()) {
            let text = fs::read_to_string(&file).map_err(|source| ContentError::Read {
                path: file.clone(),
                source,
            })?;
            let document: ComponentSchema =
                serde_yaml::from_str(&text).map_err(|source| ContentError::Parse {
                    path: file.clone(),
                    source,
                })?;
            if document.document_type != "component-schema" {
                continue;
            }
            if registry.schemas.contains_key(&document.id) {
                return Err(ContentError::DuplicateComponentSchema(document.id));
            }
            registry.schemas.insert(document.id.clone(), document);
        }
        Ok(registry)
    }

    pub fn get(&self, id: &str) -> Option<&ComponentSchema> {
        self.schemas.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ComponentSchema)> {
        self.schemas.iter()
    }

    pub fn normalize_json(
        &self,
        component: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value, ContentError> {
        let Some(schema) = self.schemas.get(component) else {
            return Err(ContentError::UnknownComponentSchema(component.to_owned()));
        };
        let object = input
            .as_object()
            .ok_or_else(|| ContentError::InvalidComponentState {
                component: component.to_owned(),
                message: "component state must be an object".to_owned(),
            })?;
        let mut output = serde_json::Map::new();
        for (field_name, field_schema) in &schema.fields {
            let value = object.get(field_name).cloned().or_else(|| {
                if field_schema.default.is_null() {
                    None
                } else {
                    serde_json::to_value(&field_schema.default).ok()
                }
            });
            match value {
                Some(value) => {
                    validate_field(component, field_name, field_schema, &value)?;
                    output.insert(field_name.clone(), value);
                }
                None if field_schema.required => {
                    return Err(ContentError::InvalidComponentState {
                        component: component.to_owned(),
                        message: format!("missing required field {field_name}"),
                    });
                }
                None => {}
            }
        }
        for (name, value) in object {
            if !schema.fields.contains_key(name) {
                output.insert(name.clone(), value.clone());
            }
        }
        Ok(serde_json::Value::Object(output))
    }
}

fn validate_field(
    component: &str,
    field_name: &str,
    schema: &FieldSchema,
    value: &serde_json::Value,
) -> Result<(), ContentError> {
    let valid_type = match schema.field_type.as_str() {
        "bool" | "boolean" => value.is_boolean(),
        "string" | "resource" | "prototype" => value.is_string(),
        "int" | "integer" | "entity" => value.as_i64().is_some() || value.as_u64().is_some(),
        "float" | "number" => value.as_f64().is_some(),
        "vec2" => value.as_array().is_some_and(|array| {
            array.len() == 2 && array.iter().all(|entry| entry.as_f64().is_some())
        }),
        "vec3" => value.as_array().is_some_and(|array| {
            array.len() == 3 && array.iter().all(|entry| entry.as_f64().is_some())
        }),
        "list" | "array" => value.is_array(),
        "map" | "object" => value.is_object(),
        "json" | "any" => true,
        _ => true,
    };
    if !valid_type {
        return Err(ContentError::InvalidComponentState {
            component: component.to_owned(),
            message: format!(
                "field {field_name} has invalid type; expected {}",
                schema.field_type
            ),
        });
    }
    if let Some(number) = value.as_f64() {
        if let Some(minimum) = schema.minimum {
            if number < minimum {
                return Err(ContentError::InvalidComponentState {
                    component: component.to_owned(),
                    message: format!("field {field_name} is below minimum {minimum}"),
                });
            }
        }
        if let Some(maximum) = schema.maximum {
            if number > maximum {
                return Err(ContentError::InvalidComponentState {
                    component: component.to_owned(),
                    message: format!("field {field_name} is above maximum {maximum}"),
                });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapDocument {
    pub map: MapDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapDefinition {
    pub id: String,
    #[serde(default = "default_tile_size")]
    pub tile_size: u16,
    #[serde(default)]
    pub tile_definitions: Vec<TileDefinition>,
    #[serde(default)]
    pub grids: Vec<GridDefinition>,
    #[serde(default)]
    pub entities: Vec<MapEntityDefinition>,
}

fn default_tile_size() -> u16 {
    32
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileDefinition {
    pub id: String,
    #[serde(default)]
    pub solid: bool,
    #[serde(default = "default_tile_color")]
    pub color: [u8; 4],
    #[serde(default)]
    pub texture: Option<String>,
}

fn default_tile_color() -> [u8; 4] {
    [32, 42, 54, 255]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridDefinition {
    pub id: String,
    #[serde(default)]
    pub position: [f32; 2],
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub chunks: Vec<TileChunkDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileChunkDefinition {
    pub position: [i32; 2],
    pub tiles: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapEntityDefinition {
    pub prototype: String,
    pub position: [f32; 2],
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub grid: Option<String>,
    #[serde(default)]
    pub components: Vec<ComponentDefinition>,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_yaml::Value;

    use super::{ComponentDefinition, EntityPrototype, PrototypeRegistry};

    #[test]
    fn resolves_component_inheritance() {
        let mut registry = PrototypeRegistry::default();
        registry
            .insert(EntityPrototype {
                document_type: "entity".to_owned(),
                id: "Base".to_owned(),
                parent: None,
                is_abstract: true,
                name: None,
                description: None,
                categories: vec![],
                tags: vec![],
                components: vec![ComponentDefinition {
                    component_type: "Health".to_owned(),
                    fields: BTreeMap::from([("maximum".to_owned(), Value::from(100))]),
                }],
            })
            .unwrap();
        registry
            .insert(EntityPrototype {
                document_type: "entity".to_owned(),
                id: "Child".to_owned(),
                parent: Some("Base".to_owned()),
                is_abstract: false,
                name: Some("child".to_owned()),
                description: None,
                categories: vec![],
                tags: vec![],
                components: vec![ComponentDefinition {
                    component_type: "Health".to_owned(),
                    fields: BTreeMap::from([("current".to_owned(), Value::from(80))]),
                }],
            })
            .unwrap();
        let health = &registry.require("Child").components[0];
        assert_eq!(health.fields.get("maximum"), Some(&Value::from(100)));
        assert_eq!(health.fields.get("current"), Some(&Value::from(80)));
    }
}
