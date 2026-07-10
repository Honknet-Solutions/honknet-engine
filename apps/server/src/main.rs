mod client_session;
mod server_state;
mod tick_loop;
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

    tokio::select! {
        result = tick_loop::run(state.clone(), TICK_RATE) => {
            result
        }
        result = transport::run_websocket_listener(transport::DEFAULT_LISTEN_ADDR, state) => {
            result
        }
    }
}
