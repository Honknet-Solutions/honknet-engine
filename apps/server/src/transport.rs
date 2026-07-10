use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::client_session;

pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:3015";

pub async fn run_websocket_listener(listen_addr: &str) -> Result<()> {
    info!(listen_addr, "Starting WebSocket transport");

    let listener = TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("failed to bind WebSocket listener on {listen_addr}"))?;

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
