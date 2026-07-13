mod app_state;
mod client_session;
mod components;
mod game_map;
mod prototypes;
mod server_state;
mod systems;
mod tick_loop;
mod transport;

use anyhow::Result;
use tracing::info;

use crate::app_state::AppState;

const TICK_RATE: u64 = 30;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!(tick_rate = TICK_RATE, "Starting Space Station 15 server");

    let state = AppState::new()?;

    tokio::select! {
        result = tick_loop::run(state.clone(), TICK_RATE) => result,
        result = transport::run(transport::DEFAULT_LISTEN_ADDR, state) => result,
    }
}
