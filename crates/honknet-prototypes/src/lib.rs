use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use thiserror::Error;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prototype {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
    #[serde(default, rename = "abstract")]
    pub abstract_: bool,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub data: Mapping,
}

#[derive(Debug, Error)]
pub enum PrototypeError {
    #[error("YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("duplicate prototype {0}")]
    Duplicate(String),
    #[error("missing parent {0}")]
    MissingParent(String),
    #[error("inheritance cycle: {0:?}")]
    Cycle(Vec<String>),
    #[error("validation {prototype}.{field}: {message}")]
    Validation {
        prototype: String,
        field: String,
        message: String,
    },
    #[error("watch: {0}")]
    Watch(String),
}

#[derive(Debug, Clone)]
pub enum FieldKind {
    String,
    Number,
    Bool,
    Sequence,
    Mapping,
    Resource,
}

#[derive(Debug, Clone)]
pub struct FieldRule {
    pub kind: FieldKind,
    pub required: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Default, Clone)]
pub struct SchemaRegistry {
    schemas: Arc<RwLock<HashMap<String, HashMap<String, FieldRule>>>>,
}

impl SchemaRegistry {
    pub fn register(&self, kind: impl Into<String>, fields: HashMap<String, FieldRule>) {
        self.schemas.write().insert(kind.into(), fields);
    }
    pub fn validate(&self, p: &Prototype) -> Result<(), PrototypeError> {
        let s = self.schemas.read();
        let Some(rules) = s.get(&p.kind) else {
            return Ok(());
        };
        for (name, r) in rules {
            let key = Value::String(name.clone());
            let v = p.data.get(&key);
            if r.required && v.is_none() {
                return Err(PrototypeError::Validation {
                    prototype: p.id.clone(),
                    field: name.clone(),
                    message: "required field missing".into(),
                });
            }
            if let Some(v) = v {
                let good = match r.kind {
                    FieldKind::String | FieldKind::Resource => v.is_string(),
                    FieldKind::Number => v.is_number(),
                    FieldKind::Bool => v.is_bool(),
                    FieldKind::Sequence => v.is_sequence(),
                    FieldKind::Mapping => v.is_mapping(),
                };
                if !good {
                    return Err(PrototypeError::Validation {
                        prototype: p.id.clone(),
                        field: name.clone(),
                        message: format!("expected {:?}", r.kind),
                    });
                }
                if let Some(n) = v.as_f64() {
                    if r.min.is_some_and(|m| n < m) || r.max.is_some_and(|m| n > m) {
                        return Err(PrototypeError::Validation {
                            prototype: p.id.clone(),
                            field: name.clone(),
                            message: "outside allowed range".into(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct PrototypeManager {
    raw: Arc<RwLock<HashMap<String, Prototype>>>,
    resolved: Arc<RwLock<HashMap<String, Mapping>>>,
    pub schemas: SchemaRegistry,
}

impl PrototypeManager {
    pub fn load_yaml(&self, text: &str) -> Result<usize, PrototypeError> {
        let mut count = 0;
        for doc in serde_yaml::Deserializer::from_str(text) {
            let p = Prototype::deserialize(doc)?;
            self.schemas.validate(&p)?;
            if self.raw.write().insert(p.id.clone(), p).is_some() {
                return Err(PrototypeError::Duplicate(
                    "duplicate id in load batch".into(),
                ));
            }
            count += 1
        }
        self.resolve_all()?;
        Ok(count)
    }
    pub fn get(&self, id: &str) -> Option<Mapping> {
        self.resolved.read().get(id).cloned()
    }
    pub fn resolve_all(&self) -> Result<(), PrototypeError> {
        let raw = self.raw.read().clone();
        let mut out = HashMap::new();
        for id in raw.keys() {
            let mut stack = vec![];
            let m = resolve(id, &raw, &mut out, &mut stack)?;
            out.insert(id.clone(), m);
        }
        *self.resolved.write() = out;
        Ok(())
    }
    pub fn watch_directory<F: Fn() + Send + 'static>(
        path: PathBuf,
        on_change: F,
    ) -> Result<RecommendedWatcher, PrototypeError> {
        let mut watcher = notify::recommended_watcher(move |r: notify::Result<notify::Event>| {
            if r.is_ok() {
                on_change()
            }
        })
        .map_err(|e| PrototypeError::Watch(e.to_string()))?;
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|e| PrototypeError::Watch(e.to_string()))?;
        Ok(watcher)
    }
}

fn resolve(
    id: &str,
    raw: &HashMap<String, Prototype>,
    out: &mut HashMap<String, Mapping>,
    stack: &mut Vec<String>,
) -> Result<Mapping, PrototypeError> {
    if let Some(m) = out.get(id) {
        return Ok(m.clone());
    }
    if stack.iter().any(|x| x == id) {
        stack.push(id.into());
        return Err(PrototypeError::Cycle(stack.clone()));
    }
    let p = raw
        .get(id)
        .ok_or_else(|| PrototypeError::MissingParent(id.into()))?;
    stack.push(id.into());
    let mut m = Mapping::new();
    for parent in &p.parents {
        let pm = resolve(parent, raw, out, stack)?;
        merge(&mut m, &pm)
    }
    merge(&mut m, &p.data);
    stack.pop();
    out.insert(id.into(), m.clone());
    Ok(m)
}

fn merge(dst: &mut Mapping, src: &Mapping) {
    for (k, v) in src {
        match (dst.get_mut(k), v) {
            (Some(Value::Mapping(d)), Value::Mapping(s)) => merge(d, s),
            _ => {
                dst.insert(k.clone(), v.clone());
            }
        }
    }
}
