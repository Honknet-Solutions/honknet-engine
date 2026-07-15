use std::{future, time::Duration};

use anyhow::{Context, Result};
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time,
};
use tracing::{info, warn};

use crate::app_state::AppState;

const MAX_REQUEST_BYTES: usize = 8_192;

pub async fn run(enabled: bool, listen_addr: &str, state: AppState) -> Result<()> {
    if !enabled {
        future::pending::<()>().await;
        return Ok(());
    }

    let listener = TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("failed to bind health listener {listen_addr}"))?;
    info!(listen_addr, "Health and metrics listener started");

    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = handle(stream, state).await {
                warn!(%peer, %error, "Health request failed");
            }
        });
    }
}

async fn handle(mut stream: TcpStream, state: AppState) -> Result<()> {
    let mut buffer = vec![0_u8; MAX_REQUEST_BYTES];
    let count = time::timeout(Duration::from_secs(2), stream.read(&mut buffer))
        .await
        .context("health request timed out")??;
    if count == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..count]);
    let first_line = request.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    if method != "GET" {
        write_response(&mut stream, 405, "text/plain", "method not allowed\n").await?;
        return Ok(());
    }

    match path {
        "/healthz" | "/readyz" => {
            let game = state.game.read().await;
            let body = serde_json::to_string(&json!({
                "status": "ok",
                "tick": game.current_tick(),
                "entities": game.entity_count(),
                "playersOnline": game.online_player_count(),
                "uptimeSeconds": state.metrics.uptime_seconds(),
                "authenticationRequired": state.auth.required(),
            }))?;
            write_response(&mut stream, 200, "application/json", &body).await?;
        }
        "/metrics" => {
            let game = state.game.read().await;
            let body = state
                .metrics
                .render_prometheus(game.entity_count(), game.online_player_count());
            write_response(&mut stream, 200, "text/plain; version=0.0.4", &body).await?;
        }
        _ => {
            write_response(&mut stream, 404, "text/plain", "not found\n").await?;
        }
    }
    Ok(())
}

async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",
        body.len(),
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}
