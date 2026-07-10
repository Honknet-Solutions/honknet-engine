mod client_session;
mod server_state;
mod transport;

use anyhow::Result;
use tracing::info;

const TICK_RATE: u64 = 30;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Space Station 15 authoritative server");
    info!(tick_rate = TICK_RATE, "Server tick configured");

    let state = server_state::new_shared_debug_state();

    transport::run_websocket_listener(transport::DEFAULT_LISTEN_ADDR, state).await
}
