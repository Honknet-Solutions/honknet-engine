use serde::{Deserialize, Serialize};

pub type AssetId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub logical_path: String,
    pub asset_id: AssetId,
    pub content_hash: String,
    pub compressed_size: usize,
    pub asset_type: String,
    pub dependencies: Vec<AssetId>,
    pub bundle: StreamingBundle,
    pub priority: i32,
    pub cache_policy: String,
    pub license_metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamingBundle(pub String);

impl StreamingBundle {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn bootstrap() -> Self {
        Self("bootstrap".into())
    }
    pub fn core_ui() -> Self {
        Self("core-ui".into())
    }
    pub fn core_world() -> Self {
        Self("core-world".into())
    }
    pub fn characters() -> Self {
        Self("characters".into())
    }
    pub fn clothing() -> Self {
        Self("clothing".into())
    }
    pub fn map(map_id: &str) -> Self {
        Self(format!("map-{map_id}"))
    }
}

impl Default for StreamingBundle {
    fn default() -> Self {
        Self::bootstrap()
    }
}

impl From<&str> for StreamingBundle {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for StreamingBundle {
    fn from(s: String) -> Self {
        Self(s)
    }
}
