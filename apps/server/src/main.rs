mod app_state;
mod client_session;
mod components;
mod config;
mod game_map;
mod prototypes;
mod server_state;
mod systems;
mod tick_loop;
mod transport;

use anyhow::{bail, Result};
use honknet_protocol::PROTOCOL_VERSION;
use tokio::signal;
use tracing::{info, warn};

use crate::{app_state::AppState, config::ServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = ServerConfig::load_default()?;
    if config.engine.protocol_version != PROTOCOL_VERSION {
        bail!(
            "engine.toml protocol_version {} does not match compiled protocol {}",
            config.engine.protocol_version,
            PROTOCOL_VERSION,
        );
    }
    config.apply_environment();

    info!(
        engine = %config.engine.name,
        version = %config.engine.version,
        tick_rate = config.engine.tick_rate,
        listen = %config.network.listen,
        snapshot_rate = config.network.snapshot_rate,
        pvs_radius = config.network.pvs_radius,
        startup_map = %config.content.startup_map,
        scripting = config.scripting.enabled,
        persistence = config.persistence.enabled,
        save_root = %config.persistence.root,
        autosave_seconds = config.persistence.autosave_seconds,
        resource_roots = ?config.content.resource_roots,
        localization_roots = ?config.content.localization_roots,
        map_roots = ?config.content.map_roots,
        "Starting Space Station 15 server",
    );

    let state = AppState::new()?;
    let tick_rate = config.engine.tick_rate.max(1);
    let listen = config.network.listen;

    let result = tokio::select! {
        result = tick_loop::run(state.clone(), tick_rate) => result,
        result = transport::run(&listen, state.clone()) => result,
        result = signal::ctrl_c() => {
            result?;
            info!("Shutdown signal received");
            Ok(())
        },
    };

    if let Err(error) = state.shutdown().await {
        warn!(%error, "Graceful shutdown encountered an error");
    }

    result
}
