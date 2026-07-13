use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use honknet_protocol::{ClientMessage, EntityNetId, PlayerIdentityId, ServerMessage};
use tokio::{net::TcpStream, sync::broadcast, time};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{app_state::AppState, server_state::InputUpdateResult};

const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(50);
const MAX_CHAT_LENGTH: usize = 500;

pub async fn run(stream: TcpStream, peer_addr: SocketAddr, state: AppState) -> Result<()> {
    info!(%peer_addr, "Client connected");

    let websocket = accept_async(stream)
        .await
        .context("failed to accept WebSocket connection")?;
    let (mut sender, mut receiver) = websocket.split();
    let client_id = Uuid::new_v4();
    let mut identity_id: Option<PlayerIdentityId> = None;
    let mut player_net_id: Option<EntityNetId> = None;
    let mut snapshot_interval = time::interval(SNAPSHOT_INTERVAL);
    let mut event_receiver = state.events.subscribe();

    let result: Result<()> = async {
        loop {
            tokio::select! {
                maybe_message = receiver.next() => {
                    let Some(message) = maybe_message else { break; };
                    let message = message.context("failed to read WebSocket message")?;

                    match message {
                        Message::Text(text) => {
                            let client_message = match serde_json::from_str::<ClientMessage>(&text) {
                                Ok(message) => message,
                                Err(error) => {
                                    warn!(%peer_addr, %error, "Malformed client message");
                                    send(&mut sender, &ServerMessage::Error {
                                        message: "Malformed client message".to_string(),
                                    }).await?;
                                    continue;
                                }
                            };

                            handle_client_message(
                                &mut sender,
                                &state,
                                client_id,
                                &mut identity_id,
                                &mut player_net_id,
                                client_message,
                            ).await?;
                        }
                        Message::Ping(payload) => {
                            sender.send(Message::Pong(payload)).await?;
                        }
                        Message::Pong(_) => {}
                        Message::Close(_) => break,
                        Message::Binary(_) | Message::Frame(_) => {}
                    }
                }

                _ = snapshot_interval.tick(), if player_net_id.is_some() => {
                    let snapshot = {
                        let game = state.game.read().await;
                        game.snapshot_for(player_net_id.expect("guarded player id"))
                    };
                    send(&mut sender, &snapshot).await?;
                }

                event = event_receiver.recv(), if player_net_id.is_some() => {
                    match event {
                        Ok(message) => send(&mut sender, &message).await?,
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!(%peer_addr, skipped, "Client event receiver lagged");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }

        Ok(())
    }.await;

    if let Some(identity_id) = identity_id {
        state.game.write().await.disconnect_player(&identity_id);
    }

    info!(%peer_addr, %client_id, "Client disconnected");
    result
}

async fn handle_client_message(
    sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    state: &AppState,
    client_id: Uuid,
    identity_id: &mut Option<PlayerIdentityId>,
    player_net_id: &mut Option<EntityNetId>,
    message: ClientMessage,
) -> Result<()> {
    match message {
        ClientMessage::Hello {
            client_version,
            identity_id: requested_identity,
        } => {
            if identity_id.is_some() {
                send(
                    sender,
                    &ServerMessage::Error {
                        message: "Hello was already accepted".to_string(),
                    },
                )
                .await?;
                return Ok(());
            }

            debug!(%client_version, %requested_identity, "Accepted handshake");

            let (entity_net_id, map, snapshot) = {
                let mut game = state.game.write().await;
                let entity_net_id = game.connect_player(client_id, requested_identity.clone());
                let map = game.map_snapshot();
                let snapshot = game.snapshot_for(entity_net_id);
                (entity_net_id, map, snapshot)
            };

            *identity_id = Some(requested_identity);
            *player_net_id = Some(entity_net_id);

            send(
                sender,
                &ServerMessage::Welcome {
                    client_id,
                    entity_net_id,
                    map,
                },
            )
            .await?;
            send(sender, &snapshot).await?;
        }

        ClientMessage::Input {
            seq,
            client_tick,
            movement,
        } => {
            let Some(entity_net_id) = *player_net_id else {
                send(
                    sender,
                    &ServerMessage::Error {
                        message: "Input rejected before handshake".to_string(),
                    },
                )
                .await?;
                return Ok(());
            };

            match state.game.write().await.set_movement_input(
                entity_net_id,
                seq,
                client_tick,
                movement,
            ) {
                InputUpdateResult::Accepted | InputUpdateResult::Stale => {}
                InputUpdateResult::EntityMissing => {
                    send(
                        sender,
                        &ServerMessage::Error {
                            message: "Player entity is missing".to_string(),
                        },
                    )
                    .await?;
                }
            }
        }

        ClientMessage::Interact { target } => {
            let Some(entity_net_id) = *player_net_id else {
                return Ok(());
            };
            if let Some(text) = state.game.write().await.interact(entity_net_id, target) {
                send(sender, &ServerMessage::System { text }).await?;
            }
        }

        ClientMessage::Chat { text } => {
            let Some(entity_net_id) = *player_net_id else {
                return Ok(());
            };
            let text = text.trim();
            if text.is_empty() {
                return Ok(());
            }

            let text = text.chars().take(MAX_CHAT_LENGTH).collect::<String>();
            let from = state
                .game
                .read()
                .await
                .player_name(entity_net_id)
                .unwrap_or_else(|| "Unknown".to_string());

            let _ = state.events.send(ServerMessage::Chat { from, text });
        }
    }

    Ok(())
}

async fn send(
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
