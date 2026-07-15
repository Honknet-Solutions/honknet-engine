mod app_state;
mod auth;
mod client_session;
mod components;
mod config;
mod game_map;
mod health;
mod metrics;
mod outbound;
mod prototypes;
mod server_state;
mod systems;
mod tick_loop;
mod transport;

use anyhow::{bail, Context, Result};
use honknet_protocol::PROTOCOL_VERSION;
use tokio::signal;
use tracing::{info, warn};

use crate::{app_state::AppState, config::ServerConfig};

fn main() -> Result<()> {
    let worker_threads = env_usize("HONKNET_WORKER_THREADS")
        .or_else(|| std::thread::available_parallelism().ok().map(usize::from))
        .unwrap_or(4)
        .max(2);
    let max_blocking_threads = env_usize("HONKNET_MAX_BLOCKING_THREADS")
        .unwrap_or_else(|| worker_threads.saturating_mul(8).max(64));

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(max_blocking_threads)
        .thread_name("honknet-worker")
        .enable_all()
        .build()
        .context("failed to create Tokio runtime")?;

    runtime.block_on(run(worker_threads, max_blocking_threads))
}

async fn run(worker_threads: usize, max_blocking_threads: usize) -> Result<()> {
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
        authentication_required = config.auth.required,
        observability = config.observability.enabled,
        observability_listen = %config.observability.listen,
        save_root = %config.persistence.root,
        autosave_seconds = config.persistence.autosave_seconds,
        resource_roots = ?config.content.resource_roots,
        localization_roots = ?config.content.localization_roots,
        map_roots = ?config.content.map_roots,
        worker_threads,
        max_blocking_threads,
        "Starting Honknet Engine server",
    );

    let state = AppState::new()?;
    let tick_rate = config.engine.tick_rate.max(1);
    let listen = config.network.listen.clone();
    let observability_enabled = config.observability.enabled;
    let observability_listen = config.observability.listen.clone();

    let result = tokio::select! {
        result = tick_loop::run(state.clone(), tick_rate) => result,
        result = transport::run(&listen, state.clone()) => result,
        result = health::run(observability_enabled, &observability_listen, state.clone()) => result,
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

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
}
