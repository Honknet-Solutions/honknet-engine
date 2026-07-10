use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use honknet_protocol::{ClientMessage, EntityNetId, PlayerIdentityId, ServerMessage};
use tokio::{net::TcpStream, time};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::server_state::{InputUpdateResult, SharedServerState};

const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(50);

pub async fn run(stream: TcpStream, peer_addr: SocketAddr, state: SharedServerState) -> Result<()> {
    info!(%peer_addr, "Client connected");

    let websocket = accept_async(stream)
        .await
        .context("failed to accept WebSocket connection")?;

    let (mut sender, mut receiver) = websocket.split();

    let client_id = Uuid::new_v4();

    let mut player_identity_id: Option<PlayerIdentityId> = None;

    let mut player_entity_net_id: Option<EntityNetId> = None;

    let mut snapshot_interval = time::interval(SNAPSHOT_INTERVAL);

    let result: Result<()> = async {
        loop {
            tokio::select! {
                maybe_message = receiver.next() => {
                    let Some(message) = maybe_message else {
                        info!(
                            %peer_addr,
                            "Client disconnected"
                        );

                        break;
                    };

                    let message = message.context(
                        "failed to read WebSocket message",
                    )?;

                    match message {
                        Message::Text(text) => {
                            debug!(
                                %peer_addr,
                                %text,
                                "Received client message"
                            );

                            let client_message =
                                match serde_json::from_str::<ClientMessage>(
                                    &text,
                                ) {
                                    Ok(client_message) => {
                                        client_message
                                    }

                                    Err(err) => {
                                        warn!(
                                            %peer_addr,
                                            error = %err,
                                            "Rejected malformed client message"
                                        );

                                        send_server_message(
                                            &mut sender,
                                            &ServerMessage::Error {
                                                message:
                                                    "Malformed client message"
                                                        .to_string(),
                                            },
                                        )
                                        .await?;

                                        continue;
                                    }
                                };

                            if let Some((
                                identity_id,
                                entity_net_id,
                            )) = handle_client_message(
                                &mut sender,
                                peer_addr,
                                client_id,
                                client_message,
                                &state,
                                player_identity_id.as_ref(),
                                player_entity_net_id,
                            )
                            .await?
                            {
                                player_identity_id =
                                    Some(identity_id);

                                player_entity_net_id =
                                    Some(entity_net_id);
                            }
                        }

                        Message::Close(_) => {
                            info!(
                                %peer_addr,
                                "Client disconnected"
                            );

                            break;
                        }

                        Message::Ping(payload) => {
                            sender
                                .send(Message::Pong(payload))
                                .await?;
                        }

                        Message::Pong(_) => {}

                        Message::Binary(_) |
                        Message::Frame(_) => {
                            warn!(
                                %peer_addr,
                                "Ignoring unsupported WebSocket message type"
                            );
                        }
                    }
                }

                _ = snapshot_interval.tick(),
                    if player_entity_net_id.is_some() =>
                {
                    let Some(entity_net_id) =
                        player_entity_net_id
                    else {
                        continue;
                    };

                    let state =
                        state.read().await;

                    let snapshot =
                        state.snapshot_message_for(
                            entity_net_id,
                        );

                    drop(state);

                    send_server_message(
                        &mut sender,
                        &snapshot,
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }
    .await;

    if let Some(identity_id) = player_identity_id {
        let mut state = state.write().await;

        if let Some(entity_net_id) = state.mark_player_disconnected(&identity_id) {
            info!(
                %peer_addr,
                %client_id,
                %identity_id,
                entity_net_id,
                "Marked player disconnected; entity remains in world"
            );
        } else {
            warn!(
                %peer_addr,
                %client_id,
                %identity_id,
                "Player record was missing on disconnect"
            );
        }
    }

    result
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
    current_identity_id: Option<&PlayerIdentityId>,
    current_entity_net_id: Option<EntityNetId>,
) -> Result<Option<(PlayerIdentityId, EntityNetId)>> {
    match message {
        ClientMessage::Hello {
            client_version,
            identity_id,
        } => {
            if let Some(existing_identity_id) = current_identity_id {
                if existing_identity_id == &identity_id {
                    warn!(
                        %peer_addr,
                        %client_id,
                        %identity_id,
                        "Client sent duplicate Hello"
                    );

                    if let Some(entity_net_id) = current_entity_net_id {
                        send_server_message(
                            sender,
                            &ServerMessage::Welcome {
                                client_id,
                                entity_net_id,
                            },
                        )
                        .await?;
                    }

                    return Ok(None);
                }

                warn!(
                    %peer_addr,
                    %client_id,
                    old_identity_id = %existing_identity_id,
                    new_identity_id = %identity_id,
                    "Client tried to change identity after handshake"
                );

                send_server_message(
                    sender,
                    &ServerMessage::Error {
                        message: "Cannot change identity after handshake".to_string(),
                    },
                )
                .await?;

                return Ok(None);
            }

            info!(
                %peer_addr,
                %client_id,
                %client_version,
                %identity_id,
                "Client handshake accepted"
            );

            let mut state_write = state.write().await;

            let entity_net_id = state_write.connect_player(client_id, identity_id.clone());

            let snapshot = state_write.snapshot_message_for(entity_net_id);

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

            Ok(Some((identity_id, entity_net_id)))
        }

        ClientMessage::Input {
            seq,
            client_tick,
            movement,
        } => {
            let Some(entity_net_id) = current_entity_net_id else {
                warn!(
                    %peer_addr,
                    seq,
                    client_tick,
                    "Rejected input before handshake"
                );

                send_server_message(
                    sender,
                    &ServerMessage::Error {
                        message: "Input rejected before handshake".to_string(),
                    },
                )
                .await?;

                return Ok(None);
            };

            let mut state_write = state.write().await;

            let update_result =
                state_write.set_movement_input(entity_net_id, seq, client_tick, movement);

            drop(state_write);

            match update_result {
                InputUpdateResult::Accepted => {
                    debug!(
                        %peer_addr,
                        seq,
                        client_tick,
                        entity_net_id,
                        ?movement,
                        "Accepted movement input"
                    );
                }

                InputUpdateResult::Stale => {
                    debug!(
                        %peer_addr,
                        seq,
                        client_tick,
                        entity_net_id,
                        ?movement,
                        "Rejected stale movement input"
                    );
                }

                InputUpdateResult::EntityMissing => {
                    warn!(
                        %peer_addr,
                        seq,
                        client_tick,
                        entity_net_id,
                        ?movement,
                        "Rejected movement input because entity was missing"
                    );
                }
            }

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
            debug!(
                %peer_addr,
                target,
                "Received interaction message"
            );

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
