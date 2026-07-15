use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use anyhow::{Context, Result};
use honknet_persistence::JsonStore;
use honknet_script::{EngineToScript, NodeScriptHost, ScriptToEngine};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::{
    auth::AuthService,
    metrics::EngineMetrics,
    outbound::OutboundMessage,
    server_state::{PersistedWorld, ServerState},
};

#[derive(Clone)]
pub struct AppState {
    pub game: Arc<RwLock<ServerState>>,
    pub events: broadcast::Sender<OutboundMessage>,
    pub script: Arc<Mutex<Option<NodeScriptHost>>>,
    pub auth: Arc<AuthService>,
    pub metrics: Arc<EngineMetrics>,
    persistence: Option<Arc<JsonStore>>,
    save_in_progress: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let event_capacity = std::env::var("HONKNET_EVENT_CHANNEL_CAPACITY")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value >= 128)
            .unwrap_or(4_096);
        let (events, _) = broadcast::channel(event_capacity);
        let script = initialize_script_host();
        let persistence = initialize_persistence();
        let auth = Arc::new(AuthService::from_environment()?);
        let metrics = Arc::new(EngineMetrics::default());
        let mut game = ServerState::new_debug()?;

        if let Some(store) = &persistence {
            if let Some(saved) = store
                .load::<PersistedWorld>("world")
                .context("failed to load persisted world")?
            {
                let legacy_player_count = saved.players.len();
                let entity_count = saved.entities.len();
                game.restore_persistence(saved);
                info!(
                    legacy_player_count,
                    entity_count, "Restored persisted world"
                );
            }
        }

        Ok(Self {
            game: Arc::new(RwLock::new(game)),
            events,
            script: Arc::new(Mutex::new(script)),
            auth,
            metrics,
            persistence,
            save_in_progress: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn request_background_save(&self) -> bool {
        if self.persistence.is_none()
            || self
                .save_in_progress
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return false;
        }

        let state = self.clone();
        tokio::spawn(async move {
            let _save_guard = SaveInProgressGuard(state.save_in_progress.clone());
            if let Err(error) = state.save_world_inner().await {
                tracing::error!(%error, "Background autosave failed");
            }
        });
        true
    }

    pub async fn save_world(&self) -> Result<()> {
        while self.save_in_progress.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        self.save_world_inner().await
    }

    async fn save_world_inner(&self) -> Result<()> {
        let Some(store) = self.persistence.clone() else {
            return Ok(());
        };
        let snapshot = self.game.read().await.persistence_snapshot();
        let result = tokio::task::spawn_blocking(move || store.save("world", &snapshot))
            .await
            .context("persistence worker failed")?;
        match result {
            Ok(()) => {
                self.metrics.autosave_completed();
                info!("World save completed");
                Ok(())
            }
            Err(error) => {
                self.metrics.persistence_failure();
                Err(error.into())
            }
        }
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

struct SaveInProgressGuard(Arc<AtomicBool>);

impl Drop for SaveInProgressGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn initialize_persistence() -> Option<Arc<JsonStore>> {
    if std::env::var_os("HONKNET_DISABLE_PERSISTENCE").is_some() {
        info!("Persistence is disabled by configuration");
        return None;
    }
    let root = std::env::var("HONKNET_SAVE_ROOT").unwrap_or_else(|_| "data/saves".to_owned());
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
        .unwrap_or_else(|_| "examples/minimal-game/server/dist/index.js".to_owned());

    if !Path::new(&host_path).is_file() || !Path::new(&module_path).is_file() {
        warn!(%host_path, %module_path, "TypeScript game module is not built; continuing without script host");
        return None;
    }

    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let readable_paths = [
        Path::new(&host_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
        Path::new(&module_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
        workspace_root.join("node_modules"),
        workspace_root.join("packages"),
    ];
    let mut host = match NodeScriptHost::spawn("node", host_path.as_str(), &readable_paths) {
        Ok(host) => host,
        Err(error) => {
            warn!(%error, "Failed to start TypeScript script host");
            return None;
        }
    };

    let timeout = Duration::from_millis(
        std::env::var("HONKNET_SCRIPT_INIT_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5_000),
    );

    match host.request_timeout(
        &EngineToScript::Initialize {
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            module_path: module_path.clone(),
        },
        timeout,
    ) {
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
