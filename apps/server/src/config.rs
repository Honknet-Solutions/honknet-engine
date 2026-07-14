use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub engine: EngineSection,
    #[serde(default)]
    pub network: NetworkSection,
    #[serde(default)]
    pub content: ContentSection,
    #[serde(default)]
    pub scripting: ScriptingSection,
    #[serde(default)]
    pub persistence: PersistenceSection,
}

impl ServerConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse config {}", path.display()))
    }

    pub fn load_default() -> Result<Self> {
        let path = std::env::var("HONKNET_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("engine.toml"));
        Self::load(path)
    }

    pub fn apply_environment(&self) {
        if let Some(root) = self.content.prototype_roots.first() {
            std::env::set_var("HONKNET_PROTOTYPES", root);
        }
        std::env::set_var("HONKNET_MAP", &self.content.startup_map);
        std::env::set_var(
            "HONKNET_SNAPSHOT_INTERVAL_MS",
            (1000_u64 / self.network.snapshot_rate.max(1)).to_string(),
        );
        std::env::set_var("HONKNET_PVS_RADIUS", self.network.pvs_radius.to_string());
        std::env::set_var("HONKNET_SAVE_ROOT", &self.persistence.root);
        std::env::set_var(
            "HONKNET_AUTOSAVE_SECONDS",
            self.persistence.autosave_seconds.to_string(),
        );
        if self.persistence.enabled {
            std::env::remove_var("HONKNET_DISABLE_PERSISTENCE");
        } else {
            std::env::set_var("HONKNET_DISABLE_PERSISTENCE", "1");
        }
        if self.scripting.enabled {
            std::env::remove_var("HONKNET_DISABLE_SCRIPTING");
            std::env::set_var("HONKNET_SCRIPT_HOST", &self.scripting.host);
            std::env::set_var("HONKNET_GAME_MODULE", &self.scripting.module);
        } else {
            std::env::set_var("HONKNET_DISABLE_SCRIPTING", "1");
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct EngineSection {
    #[serde(default = "default_engine_name")]
    pub name: String,
    #[serde(default = "default_engine_version")]
    pub version: String,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: u32,
    #[serde(default = "default_tick_rate")]
    pub tick_rate: u64,
}

impl Default for EngineSection {
    fn default() -> Self {
        Self {
            name: default_engine_name(),
            version: default_engine_version(),
            protocol_version: default_protocol_version(),
            tick_rate: default_tick_rate(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkSection {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_snapshot_rate")]
    pub snapshot_rate: u64,
    #[serde(default = "default_pvs_radius")]
    pub pvs_radius: f32,
}

impl Default for NetworkSection {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            snapshot_rate: default_snapshot_rate(),
            pvs_radius: default_pvs_radius(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentSection {
    #[serde(default = "default_prototype_roots")]
    pub prototype_roots: Vec<String>,
    #[serde(default = "default_map_roots")]
    pub map_roots: Vec<String>,
    #[serde(default = "default_startup_map")]
    pub startup_map: String,
    #[serde(default = "default_resource_roots")]
    pub resource_roots: Vec<String>,
    #[serde(default = "default_localization_roots")]
    pub localization_roots: Vec<String>,
}

impl Default for ContentSection {
    fn default() -> Self {
        Self {
            prototype_roots: default_prototype_roots(),
            map_roots: default_map_roots(),
            startup_map: default_startup_map(),
            resource_roots: default_resource_roots(),
            localization_roots: default_localization_roots(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptingSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_script_host")]
    pub host: String,
    #[serde(default = "default_game_module")]
    pub module: String,
}

impl Default for ScriptingSection {
    fn default() -> Self {
        Self {
            enabled: true,
            host: default_script_host(),
            module: default_game_module(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PersistenceSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_save_root")]
    pub root: String,
    #[serde(default = "default_autosave_seconds")]
    pub autosave_seconds: u64,
}

impl Default for PersistenceSection {
    fn default() -> Self {
        Self {
            enabled: true,
            root: default_save_root(),
            autosave_seconds: default_autosave_seconds(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_engine_name() -> String {
    "Honknet Engine".to_owned()
}
fn default_engine_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
fn default_protocol_version() -> u32 {
    1
}
fn default_tick_rate() -> u64 {
    30
}
fn default_listen() -> String {
    "0.0.0.0:3015".to_owned()
}
fn default_snapshot_rate() -> u64 {
    20
}
fn default_pvs_radius() -> f32 {
    32.0
}
fn default_prototype_roots() -> Vec<String> {
    vec!["game/example-module/content/prototypes".to_owned()]
}
fn default_map_roots() -> Vec<String> {
    vec!["game/example-module/maps".to_owned()]
}
fn default_startup_map() -> String {
    "game/example-module/maps/debug-map.yml".to_owned()
}
fn default_resource_roots() -> Vec<String> {
    vec!["game/example-module/resources".to_owned()]
}
fn default_localization_roots() -> Vec<String> {
    vec!["game/example-module/localization".to_owned()]
}
fn default_script_host() -> String {
    "apps/script-host/dist/main.js".to_owned()
}
fn default_game_module() -> String {
    "game/example-module/server/dist/index.js".to_owned()
}
fn default_save_root() -> String {
    "data/saves".to_owned()
}
fn default_autosave_seconds() -> u64 {
    300
}
