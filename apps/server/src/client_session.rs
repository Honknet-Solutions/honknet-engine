use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use ss15_protocol::{ClientMessage, EntityNetId, ServerMessage};
use tokio::{net::TcpStream, time};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::server_state::SharedServerState;

const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(250);

pub async fn run(stream: TcpStream, peer_addr: SocketAddr, state: SharedServerState) -> Result<()> {
    info!(%peer_addr, "Client connected");

    let websocket = accept_async(stream)
        .await
        .context("failed to accept WebSocket connection")?;

    let (mut sender, mut receiver) = websocket.split();
    let client_id = Uuid::new_v4();
    let mut player_entity_net_id: Option<EntityNetId> = None;
    let mut snapshot_interval = time::interval(SNAPSHOT_INTERVAL);

    loop {
        tokio::select! {
            maybe_message = receiver.next() => {
                let Some(message) = maybe_message else {
                    info!(%peer_addr, "Client disconnected");
                    break;
                };

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

                        if let Some(entity_net_id) = handle_client_message(
                            &mut sender,
                            peer_addr,
                            client_id,
                            client_message,
                            &state,
                            player_entity_net_id,
                        )
                        .await?
                        {
                            player_entity_net_id = Some(entity_net_id);
                        }
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

            _ = snapshot_interval.tick(), if player_entity_net_id.is_some() => {
                let state = state.read().await;
                send_server_message(&mut sender, &state.snapshot_message()).await?;
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
    state: &SharedServerState,
    player_entity_net_id: Option<EntityNetId>,
) -> Result<Option<EntityNetId>> {
    match message {
        ClientMessage::Hello { client_version } => {
            if let Some(existing_entity_net_id) = player_entity_net_id {
                warn!(%peer_addr, %client_id, "Client sent duplicate Hello");

                send_server_message(
                    sender,
                    &ServerMessage::Welcome {
                        client_id,
                        entity_net_id: existing_entity_net_id,
                    },
                )
                .await?;

                return Ok(None);
            }

            info!(%peer_addr, %client_id, %client_version, "Client handshake accepted");

            let mut state_write = state.write().await;
            let entity_net_id = state_write.spawn_player_entity();
            let snapshot = state_write.snapshot_message();
            drop(state_write);

            send_server_message(
                sender,
                &ServerMessage::Welcome {
                    client_id,
                    entity_net_id,
                },
            )
            .await?;

            send_server_message(sender, &snapshot).await?;

            Ok(Some(entity_net_id))
        }
        ClientMessage::Input { seq, movement } => {
            debug!(%peer_addr, seq, ?movement, "Received input message");
            Ok(None)
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

            Ok(None)
        }
        ClientMessage::Interact { target } => {
            debug!(%peer_addr, target, "Received interaction message");
            Ok(None)
        }
    }
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
