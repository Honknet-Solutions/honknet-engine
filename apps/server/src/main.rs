mod client_session;
mod debug_world;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::{error, info};

const TICK_RATE: u64 = 30;
const LISTEN_ADDR: &str = "127.0.0.1:3015";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Space Station 15 authoritative server");
    info!(tick_rate = TICK_RATE, "Server tick configured");
    info!(listen_addr = LISTEN_ADDR, "Starting WebSocket transport");

    let listener = TcpListener::bind(LISTEN_ADDR)
        .await
        .with_context(|| format!("failed to bind WebSocket listener on {LISTEN_ADDR}"))?;

    loop {
        let (stream, peer_addr) = listener
            .accept()
            .await
            .context("failed to accept TCP connection")?;

        tokio::spawn(async move {
            if let Err(err) = client_session::run(stream, peer_addr).await {
                error!(%peer_addr, error = %err, "Client session failed");
            }
        });
    }
}
