use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use honknet_persistence::JsonStore;
use honknet_protocol::ServerMessage;
use honknet_script::{EngineToScript, NodeScriptHost, ScriptToEngine};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::server_state::{PersistedWorld, ServerState};

#[derive(Clone)]
pub struct AppState {
    pub game: Arc<RwLock<ServerState>>,
    pub events: broadcast::Sender<ServerMessage>,
    pub script: Arc<Mutex<Option<NodeScriptHost>>>,
    persistence: Option<Arc<JsonStore>>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let (events, _) = broadcast::channel(128);
        let script = initialize_script_host();
        let persistence = initialize_persistence();
        let mut game = ServerState::new_debug()?;

        if let Some(store) = &persistence {
            if let Some(saved) = store
                .load::<PersistedWorld>("world")
                .context("failed to load persisted world")?
            {
                let player_count = saved.players.len();
                game.restore_persistence(saved);
                info!(player_count, "Restored persisted world");
            }
        }

        Ok(Self {
            game: Arc::new(RwLock::new(game)),
            events,
            script: Arc::new(Mutex::new(script)),
            persistence,
        })
    }

    pub async fn save_world(&self) -> Result<()> {
        let Some(store) = self.persistence.clone() else {
            return Ok(());
        };
        let snapshot = self.game.read().await.persistence_snapshot();
        tokio::task::spawn_blocking(move || store.save("world", &snapshot))
            .await
            .context("persistence worker failed")??;
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.save_world().await?;
        let host = {
            let mut guard = self
                .script
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.take()
        };
        if let Some(host) = host {
            tokio::task::spawn_blocking(move || host.shutdown())
                .await
                .context("script host shutdown worker failed")??;
        }
        Ok(())
    }
}

fn initialize_persistence() -> Option<Arc<JsonStore>> {
    if std::env::var_os("HONKNET_DISABLE_PERSISTENCE").is_some() {
        info!("Persistence is disabled by configuration");
        return None;
    }
    let root = std::env::var("HONKNET_SAVE_ROOT")
        .unwrap_or_else(|_| "data/saves".to_owned());
    Some(Arc::new(JsonStore::new(root)))
}

fn initialize_script_host() -> Option<NodeScriptHost> {
    if std::env::var_os("HONKNET_DISABLE_SCRIPTING").is_some() {
        info!("TypeScript scripting is disabled by configuration");
        return None;
    }

    let host_path = std::env::var("HONKNET_SCRIPT_HOST")
        .unwrap_or_else(|_| "apps/script-host/dist/main.js".to_owned());
    let module_path = std::env::var("HONKNET_GAME_MODULE")
        .unwrap_or_else(|_| "game/example-module/server/dist/index.js".to_owned());

    if !Path::new(&host_path).is_file() || !Path::new(&module_path).is_file() {
        warn!(%host_path, %module_path, "TypeScript game module is not built; continuing without script host");
        return None;
    }

    let mut host = match NodeScriptHost::spawn("node", host_path.as_str()) {
        Ok(host) => host,
        Err(error) => {
            warn!(%error, "Failed to start TypeScript script host");
            return None;
        }
    };

    match host.request(&EngineToScript::Initialize {
        engine_version: env!("CARGO_PKG_VERSION").to_owned(),
        module_path: module_path.clone(),
    }) {
        Ok(ScriptToEngine::Ready { module_id }) => {
            info!(%module_id, %module_path, "TypeScript game module loaded");
            Some(host)
        }
        Ok(other) => {
            warn!(?other, "Unexpected response while initializing script host");
            None
        }
        Err(error) => {
            warn!(%error, "Failed to initialize TypeScript game module");
            None
        }
    }
}
