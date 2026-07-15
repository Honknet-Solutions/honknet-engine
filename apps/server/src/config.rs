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
    #[serde(default)]
    pub auth: AuthSection,
    #[serde(default)]
    pub observability: ObservabilitySection,
}

impl ServerConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        toml::from_str(&text)
            .with_context(|| format!("failed to parse config {}", path.display()))
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
        if let Some(root) = self.content.component_schema_roots.first() {
            std::env::set_var("HONKNET_COMPONENT_SCHEMAS", root);
        }
        std::env::set_var("HONKNET_MAP", &self.content.startup_map);
        std::env::set_var("HONKNET_SNAPSHOT_RATE", self.network.snapshot_rate.to_string());
        std::env::set_var("HONKNET_PVS_RADIUS", self.network.pvs_radius.to_string());
        std::env::set_var(
            "HONKNET_MAX_PVS_ENTITIES",
            self.network.max_pvs_entities.to_string(),
        );
        std::env::set_var(
            "HONKNET_MAX_MESSAGE_BYTES",
            self.network.max_message_bytes.to_string(),
        );
        std::env::set_var(
            "HONKNET_HANDSHAKE_TIMEOUT_MS",
            self.network.handshake_timeout_ms.to_string(),
        );
        std::env::set_var(
            "HONKNET_CLIENT_TIMEOUT_MS",
            self.network.client_timeout_ms.to_string(),
        );
        std::env::set_var(
            "HONKNET_MAX_CONNECTIONS",
            self.network.max_connections.to_string(),
        );
        std::env::set_var(
            "HONKNET_MAX_CONNECTIONS_PER_IP",
            self.network.max_connections_per_ip.to_string(),
        );
        std::env::set_var(
            "HONKNET_EVENT_CHANNEL_CAPACITY",
            self.network.event_channel_capacity.to_string(),
        );
        std::env::set_var(
            "HONKNET_SCRIPT_MAX_TICK_MS",
            self.scripting.max_tick_ms.to_string(),
        );
        std::env::set_var(
            "HONKNET_SCRIPT_MAX_COMMANDS",
            self.scripting.max_commands_per_tick.to_string(),
        );
        std::env::set_var(
            "HONKNET_SCRIPT_INIT_TIMEOUT_MS",
            self.scripting.init_timeout_ms.to_string(),
        );
        std::env::set_var("HONKNET_SAVE_ROOT", &self.persistence.root);
        std::env::set_var(
            "HONKNET_AUTOSAVE_SECONDS",
            self.persistence.autosave_seconds.to_string(),
        );
        std::env::set_var("HONKNET_AUTH_REQUIRED", self.auth.required.to_string());
        std::env::set_var(
            "HONKNET_AUTH_CLOCK_SKEW_SECONDS",
            self.auth.clock_skew_seconds.to_string(),
        );
        std::env::set_var(
            "HONKNET_AUTH_MAX_TOKEN_LIFETIME_SECONDS",
            self.auth.max_token_lifetime_seconds.to_string(),
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
    #[serde(default = "default_max_pvs_entities")]
    pub max_pvs_entities: usize,
    #[serde(default = "default_max_message_bytes")]
    pub max_message_bytes: usize,
    #[serde(default = "default_handshake_timeout_ms")]
    pub handshake_timeout_ms: u64,
    #[serde(default = "default_client_timeout_ms")]
    pub client_timeout_ms: u64,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_max_connections_per_ip")]
    pub max_connections_per_ip: usize,
    #[serde(default = "default_event_channel_capacity")]
    pub event_channel_capacity: usize,
}

impl Default for NetworkSection {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            snapshot_rate: default_snapshot_rate(),
            pvs_radius: default_pvs_radius(),
            max_pvs_entities: default_max_pvs_entities(),
            max_message_bytes: default_max_message_bytes(),
            handshake_timeout_ms: default_handshake_timeout_ms(),
            client_timeout_ms: default_client_timeout_ms(),
            max_connections: default_max_connections(),
            max_connections_per_ip: default_max_connections_per_ip(),
            event_channel_capacity: default_event_channel_capacity(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentSection {
    #[serde(default = "default_prototype_roots")]
    pub prototype_roots: Vec<String>,
    #[serde(default = "default_map_roots")]
    pub map_roots: Vec<String>,
    #[serde(default = "default_component_schema_roots")]
    pub component_schema_roots: Vec<String>,
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
            component_schema_roots: default_component_schema_roots(),
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
    #[serde(default = "default_init_timeout_ms")]
    pub init_timeout_ms: u64,
    #[serde(default = "default_max_tick_ms")]
    pub max_tick_ms: u64,
    #[serde(default = "default_max_commands_per_tick")]
    pub max_commands_per_tick: usize,
}

impl Default for ScriptingSection {
    fn default() -> Self {
        Self {
            enabled: true,
            host: default_script_host(),
            module: default_game_module(),
            init_timeout_ms: default_init_timeout_ms(),
            max_tick_ms: default_max_tick_ms(),
            max_commands_per_tick: default_max_commands_per_tick(),
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

#[derive(Debug, Clone, Deserialize)]
pub struct AuthSection {
    #[serde(default)]
    pub required: bool,
    #[serde(default = "default_auth_clock_skew_seconds")]
    pub clock_skew_seconds: u64,
    #[serde(default = "default_auth_max_token_lifetime_seconds")]
    pub max_token_lifetime_seconds: u64,
}

impl Default for AuthSection {
    fn default() -> Self {
        Self {
            required: false,
            clock_skew_seconds: default_auth_clock_skew_seconds(),
            max_token_lifetime_seconds: default_auth_max_token_lifetime_seconds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilitySection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_observability_listen")]
    pub listen: String,
}

impl Default for ObservabilitySection {
    fn default() -> Self {
        Self {
            enabled: true,
            listen: default_observability_listen(),
        }
    }
}

fn default_true() -> bool { true }
fn default_engine_name() -> String { "Honknet Engine".to_owned() }
fn default_engine_version() -> String { env!("CARGO_PKG_VERSION").to_owned() }
fn default_protocol_version() -> u32 { 4 }
fn default_tick_rate() -> u64 { 30 }
fn default_listen() -> String { "0.0.0.0:3015".to_owned() }
fn default_snapshot_rate() -> u64 { 20 }
fn default_pvs_radius() -> f32 { 32.0 }
fn default_max_pvs_entities() -> usize { 4_096 }
fn default_max_message_bytes() -> usize { 262_144 }
fn default_handshake_timeout_ms() -> u64 { 10_000 }
fn default_client_timeout_ms() -> u64 { 15_000 }
fn default_max_connections() -> usize { 256 }
fn default_max_connections_per_ip() -> usize { 8 }
fn default_event_channel_capacity() -> usize { 4_096 }
fn default_init_timeout_ms() -> u64 { 5_000 }
fn default_max_tick_ms() -> u64 { 12 }
fn default_max_commands_per_tick() -> usize { 4_096 }
fn default_prototype_roots() -> Vec<String> {
    vec!["examples/minimal-game/content/prototypes".to_owned()]
}
fn default_map_roots() -> Vec<String> {
    vec!["examples/minimal-game/maps".to_owned()]
}
fn default_component_schema_roots() -> Vec<String> {
    vec!["examples/minimal-game/content/component-schemas".to_owned()]
}
fn default_startup_map() -> String {
    "examples/minimal-game/maps/debug-map.yml".to_owned()
}
fn default_resource_roots() -> Vec<String> {
    vec!["examples/minimal-game/resources".to_owned()]
}
fn default_localization_roots() -> Vec<String> {
    vec!["examples/minimal-game/localization".to_owned()]
}
fn default_script_host() -> String { "apps/script-host/dist/main.js".to_owned() }
fn default_game_module() -> String { "examples/minimal-game/server/dist/index.js".to_owned() }
fn default_save_root() -> String { "data/saves".to_owned() }
fn default_autosave_seconds() -> u64 { 300 }
fn default_auth_clock_skew_seconds() -> u64 { 30 }
fn default_auth_max_token_lifetime_seconds() -> u64 { 2_592_000 }
fn default_observability_listen() -> String { "127.0.0.1:3016".to_owned() }
