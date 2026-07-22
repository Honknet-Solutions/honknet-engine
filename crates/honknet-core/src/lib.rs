use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

impl Entity {
    pub const DANGLING: Self = Self {
        index: u32::MAX,
        generation: u32::MAX,
    };
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.index, self.generation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tick(pub u64);
#[derive(Debug, Clone)]
pub struct EngineClock {
    start: Instant,
    tick: Tick,
    fixed: Duration,
}

impl EngineClock {
    pub fn new(rate: u32) -> Self {
        Self {
            start: Instant::now(),
            tick: Tick(0),
            fixed: Duration::from_secs_f64(1.0 / rate.max(1) as f64),
        }
    }
    pub fn advance(&mut self) {
        self.tick.0 += 1;
    }
    pub fn tick(&self) -> Tick {
        self.tick
    }
    pub fn fixed_delta(&self) -> Duration {
        self.fixed
    }
    pub fn uptime(&self) -> Duration {
        self.start.elapsed()
    }
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization: {0}")]
    Serialization(String),
}

pub type EngineResult<T> = Result<T, EngineError>;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CVarValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

#[derive(Default, Clone)]
pub struct CVarRegistry {
    values: Arc<RwLock<HashMap<String, CVarValue>>>,
}

impl CVarRegistry {
    pub fn set(&self, key: impl Into<String>, value: CVarValue) {
        self.values.write().insert(key.into(), value);
    }
    pub fn get(&self, key: &str) -> Option<CVarValue> {
        self.values.read().get(key).cloned()
    }
    pub fn snapshot(&self) -> HashMap<String, CVarValue> {
        self.values.read().clone()
    }
    pub fn load_env_prefix(&self, prefix: &str) {
        for (k, v) in std::env::vars().filter(|(k, _)| k.starts_with(prefix)) {
            self.set(k, CVarValue::Text(v));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entity_generation_is_part_of_identity() {
        assert_ne!(Entity::new(1, 0), Entity::new(1, 1));
    }
    #[test]
    fn cvars_round_trip() {
        let c = CVarRegistry::default();
        c.set("tick", CVarValue::Int(30));
        assert!(matches!(c.get("tick"), Some(CVarValue::Int(30))));
    }
}
