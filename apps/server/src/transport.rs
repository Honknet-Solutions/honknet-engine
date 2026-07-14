use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::{app_state::AppState, client_session};

pub async fn run(listen_addr: &str, state: AppState) -> Result<()> {
    let listener = TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("failed to bind {listen_addr}"))?;

    info!(listen_addr, "WebSocket listener started");

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let state = state.clone();

        tokio::spawn(async move {
            if let Err(error) = client_session::run(stream, peer_addr, state).await {
                error!(%peer_addr, %error, "Client session failed");
            }
        });
    }
}
