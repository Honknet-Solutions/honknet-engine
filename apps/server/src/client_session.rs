use std::net::SocketAddr;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use ss15_protocol::{ClientMessage, ServerMessage};
use tokio::net::TcpStream;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::debug_world;

pub async fn run(stream: TcpStream, peer_addr: SocketAddr) -> Result<()> {
    info!(%peer_addr, "Client connected");

    let websocket = accept_async(stream)
        .await
        .context("failed to accept WebSocket connection")?;

    let (mut sender, mut receiver) = websocket.split();
    let client_id = Uuid::new_v4();

    while let Some(message) = receiver.next().await {
        let message = message.context("failed to read WebSocket message")?;

        match message {
            Message::Text(text) => {
                debug!(%peer_addr, %text, "Received client message");

                let client_message = match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_message) => client_message,
                    Err(err) => {
                        warn!(%peer_addr, error = %err, "Rejected malformed client message");

                        send_server_message(
                            &mut sender,
                            &ServerMessage::Error {
                                message: "Malformed client message".to_string(),
                            },
                        )
                        .await?;

                        continue;
                    }
                };

                handle_client_message(&mut sender, peer_addr, client_id, client_message).await?;
            }
            Message::Close(_) => {
                info!(%peer_addr, "Client disconnected");
                break;
            }
            Message::Ping(payload) => {
                sender.send(Message::Pong(payload)).await?;
            }
            Message::Pong(_) => {}
            Message::Binary(_) | Message::Frame(_) => {
                warn!(%peer_addr, "Ignoring unsupported WebSocket message type");
            }
        }
    }

    Ok(())
}

async fn handle_client_message(
    sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    peer_addr: SocketAddr,
    client_id: Uuid,
    message: ClientMessage,
) -> Result<()> {
    match message {
        ClientMessage::Hello { client_version } => {
            info!(%peer_addr, %client_id, %client_version, "Client handshake accepted");

            send_server_message(sender, &ServerMessage::Welcome { client_id }).await?;
            send_server_message(sender, &debug_world::initial_snapshot()).await?;
        }
        ClientMessage::Input { seq, movement } => {
            debug!(%peer_addr, seq, ?movement, "Received input message");
        }
        ClientMessage::Chat { text } => {
            send_server_message(
                sender,
                &ServerMessage::Chat {
                    from: "server".to_string(),
                    text,
                },
            )
            .await?;
        }
        ClientMessage::Interact { target } => {
            debug!(%peer_addr, target, "Received interaction message");
        }
    }

    Ok(())
}

async fn send_server_message(
    sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    message: &ServerMessage,
) -> Result<()> {
    let text = serde_json::to_string(message).context("failed to serialize server message")?;
    sender.send(Message::Text(text)).await?;
    Ok(())
}
