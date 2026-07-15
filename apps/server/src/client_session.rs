use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use honknet_protocol::{
    ClientMessage, EntityNetId, EntitySnapshot, PlayerIdentityId, ServerMessage, PROTOCOL_VERSION,
};
use tokio::{net::TcpStream, sync::broadcast, time};
use tokio_tungstenite::{
    accept_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    outbound::OutboundMessage,
    server_state::{InputUpdateResult, SnapshotView},
};

const MAX_CHAT_LENGTH: usize = 500;
const MAX_IDENTITY_LENGTH: usize = 128;
const MAX_CLIENT_VERSION_LENGTH: usize = 64;
const MAX_PROTOCOL_ERRORS: u32 = 5;

pub async fn run(stream: TcpStream, peer_addr: SocketAddr, state: AppState) -> Result<()> {
    info!(%peer_addr, "Client connected");

    let max_message_bytes = env_usize("HONKNET_MAX_MESSAGE_BYTES", 262_144).max(1_024);
    let handshake_timeout = Duration::from_millis(env_u64("HONKNET_HANDSHAKE_TIMEOUT_MS", 10_000));
    let client_timeout = Duration::from_millis(env_u64("HONKNET_CLIENT_TIMEOUT_MS", 15_000));
    let mut websocket_config = WebSocketConfig::default();
    websocket_config.max_message_size = Some(max_message_bytes);
    websocket_config.max_frame_size = Some(max_message_bytes);

    let websocket = time::timeout(
        handshake_timeout,
        accept_async_with_config(stream, Some(websocket_config)),
    )
    .await
    .context("WebSocket handshake timed out")?
    .context("failed to accept WebSocket connection")?;
    let (mut sender, mut receiver) = websocket.split();
    let client_id = Uuid::new_v4();
    let mut identity_id: Option<PlayerIdentityId> = None;
    let mut player_net_id: Option<EntityNetId> = None;
    let snapshot_rate = env_u64("HONKNET_SNAPSHOT_RATE", 20).max(1);
    let snapshot_period = Duration::from_secs_f64(1.0 / snapshot_rate as f64);
    let jitter_window_micros = snapshot_period.as_micros().max(1);
    let jitter_seed = client_id.as_u128() % jitter_window_micros;
    let snapshot_start =
        time::Instant::now() + Duration::from_micros(jitter_seed.try_into().unwrap_or(u64::MAX));
    let mut snapshot_interval = time::interval_at(snapshot_start, snapshot_period);
    snapshot_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
    let mut maintenance_interval = time::interval(Duration::from_secs(1));
    let mut event_receiver = state.events.subscribe();
    let mut snapshot_tracker = SnapshotTracker::default();
    let mut last_activity = Instant::now();
    let connected_at = Instant::now();
    let mut protocol_errors = 0_u32;
    let mut input_limiter = TokenBucket::new(180.0, 360.0);
    let mut chat_limiter = TokenBucket::new(1.0, 6.0);
    let mut ui_limiter = TokenBucket::new(30.0, 60.0);

    let result: Result<()> = async {
        loop {
            tokio::select! {
                maybe_message = receiver.next() => {
                    let Some(message) = maybe_message else { break; };
                    let message = message.context("failed to read WebSocket message")?;
                    last_activity = Instant::now();

                    match message {
                        Message::Text(text) => {
                            state.metrics.message_received(text.len());
                            if text.len() > max_message_bytes {
                                send_error(&mut sender, "network.message_too_large", "Message exceeds configured size limit", true).await?;
                                break;
                            }
                            let client_message = match serde_json::from_str::<ClientMessage>(&text) {
                                Ok(message) => message,
                                Err(error) => {
                                    protocol_errors = protocol_errors.saturating_add(1);
                                    state.metrics.malformed_message();
                                    warn!(%peer_addr, %error, protocol_errors, "Malformed client message");
                                    send_error(&mut sender, "network.malformed_message", "Malformed client message", protocol_errors >= MAX_PROTOCOL_ERRORS).await?;
                                    if protocol_errors >= MAX_PROTOCOL_ERRORS { break; }
                                    continue;
                                }
                            };

                            let keep_running = handle_client_message(
                                &mut sender,
                                &state,
                                client_id,
                                &mut identity_id,
                                &mut player_net_id,
                                &mut snapshot_tracker,
                                &mut input_limiter,
                                &mut chat_limiter,
                                &mut ui_limiter,
                                client_message,
                            ).await?;
                            if !keep_running { break; }
                        }
                        Message::Ping(payload) => sender.send(Message::Pong(payload)).await?,
                        Message::Pong(_) => {}
                        Message::Close(_) => break,
                        Message::Binary(payload) => {
                            state.metrics.message_received(payload.len());
                            state.metrics.malformed_message();
                            protocol_errors = protocol_errors.saturating_add(1);
                            send_error(&mut sender, "network.binary_not_supported", "Binary client messages are not supported", protocol_errors >= MAX_PROTOCOL_ERRORS).await?;
                            if protocol_errors >= MAX_PROTOCOL_ERRORS { break; }
                        }
                        Message::Frame(_) => {
                            state.metrics.malformed_message();
                            protocol_errors = protocol_errors.saturating_add(1);
                            send_error(&mut sender, "network.frame_not_supported", "Raw WebSocket frames are not supported", protocol_errors >= MAX_PROTOCOL_ERRORS).await?;
                            if protocol_errors >= MAX_PROTOCOL_ERRORS { break; }
                        }
                    }
                }

                _ = snapshot_interval.tick(), if player_net_id.is_some() => {
                    let player = player_net_id.expect("guarded player id");
                    let snapshot_view = {
                        let game = state.game.read().await;
                        if !game.is_session_owner(client_id, player) {
                            drop(game);
                            send_error(
                                &mut sender,
                                "auth.session_replaced",
                                "This identity connected from another session",
                                true,
                            )
                            .await?;
                            break;
                        }
                        game.snapshot_view_for(
                            player,
                            &snapshot_tracker.hashes,
                            snapshot_tracker.needs_full(),
                        )
                    };
                    let snapshot = snapshot_tracker.encode(snapshot_view);
                    state
                        .metrics
                        .snapshot_sent(matches!(&snapshot, ServerMessage::Snapshot { .. }));
                    send(&mut sender, &snapshot).await?;
                }

                event = event_receiver.recv(), if player_net_id.is_some() => {
                    match event {
                        Ok(outbound) => {
                            let player = player_net_id.expect("guarded player id");
                            if outbound.is_for(player) {
                                send(&mut sender, &outbound.message).await?;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!(%peer_addr, skipped, "Client event receiver lagged; forcing full state");
                            snapshot_tracker.force_full();
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }

                _ = maintenance_interval.tick() => {
                    let now = Instant::now();
                    if identity_id.is_none() && now.duration_since(connected_at) >= handshake_timeout {
                        send_error(&mut sender, "network.hello_timeout", "Hello was not received before the handshake deadline", true).await?;
                        break;
                    }
                    if now.duration_since(last_activity) >= client_timeout {
                        send_error(&mut sender, "network.client_timeout", "Client timed out", true).await?;
                        break;
                    }
                    if let Some(player) = player_net_id {
                        if !state.game.read().await.is_session_owner(client_id, player) {
                            send_error(
                                &mut sender,
                                "auth.session_replaced",
                                "This identity connected from another session",
                                true,
                            )
                            .await?;
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    .await;

    if let Some(identity_id) = identity_id {
        state
            .game
            .write()
            .await
            .disconnect_player(&identity_id, client_id);
    }
    info!(%peer_addr, %client_id, "Client disconnected");
    result
}

#[allow(clippy::too_many_arguments)]
async fn handle_client_message(
    sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    state: &AppState,
    client_id: Uuid,
    identity_id: &mut Option<PlayerIdentityId>,
    player_net_id: &mut Option<EntityNetId>,
    snapshot_tracker: &mut SnapshotTracker,
    input_limiter: &mut TokenBucket,
    chat_limiter: &mut TokenBucket,
    ui_limiter: &mut TokenBucket,
    message: ClientMessage,
) -> Result<bool> {
    if !matches!(&message, ClientMessage::Hello { .. }) {
        if let Some(player) = *player_net_id {
            if !state.game.read().await.is_session_owner(client_id, player) {
                send_error(
                    sender,
                    "auth.session_replaced",
                    "This identity connected from another session",
                    true,
                )
                .await?;
                return Ok(false);
            }
        }
    }

    match message {
        ClientMessage::Hello {
            protocol_version,
            client_version,
            identity_id: requested_identity,
            auth_token,
        } => {
            if protocol_version != PROTOCOL_VERSION {
                send_error(
                    sender,
                    "network.protocol_mismatch",
                    &format!(
                        "Protocol mismatch: server={PROTOCOL_VERSION}, client={protocol_version}"
                    ),
                    true,
                )
                .await?;
                return Ok(false);
            }
            if identity_id.is_some() {
                send_error(
                    sender,
                    "network.duplicate_hello",
                    "Hello was already accepted",
                    false,
                )
                .await?;
                return Ok(true);
            }
            if client_version.is_empty() || client_version.len() > MAX_CLIENT_VERSION_LENGTH {
                send_error(
                    sender,
                    "network.invalid_client_version",
                    "Client version is invalid",
                    true,
                )
                .await?;
                return Ok(false);
            }
            if !valid_identity(&requested_identity) {
                send_error(
                    sender,
                    "auth.invalid_identity",
                    "Identity contains unsupported characters or has an invalid length",
                    true,
                )
                .await?;
                return Ok(false);
            }

            if let Err(error) = state
                .auth
                .validate(&requested_identity, auth_token.as_deref())
            {
                send_error(sender, error.code(), error.message(), true).await?;
                return Ok(false);
            }

            debug!(%client_version, %requested_identity, "Accepted handshake");
            let (entity_net_id, map, server_tick, snapshot_view) = {
                let mut game = state.game.write().await;
                let entity_net_id = game.connect_player(client_id, requested_identity.clone());
                let map = game.map_snapshot();
                let server_tick = game.current_tick();
                let snapshot_view = game.snapshot_view_for(entity_net_id, &HashMap::new(), true);
                (entity_net_id, map, server_tick, snapshot_view)
            };
            *identity_id = Some(requested_identity);
            *player_net_id = Some(entity_net_id);

            send(
                sender,
                &ServerMessage::Welcome {
                    protocol_version: PROTOCOL_VERSION,
                    client_id,
                    entity_net_id,
                    server_tick,
                    map,
                },
            )
            .await?;
            snapshot_tracker.force_full();
            let snapshot = snapshot_tracker.encode(snapshot_view);
            state
                .metrics
                .snapshot_sent(matches!(&snapshot, ServerMessage::Snapshot { .. }));
            send(sender, &snapshot).await?;
        }

        ClientMessage::Input {
            seq,
            client_tick,
            movement,
        } => {
            let Some(entity_net_id) = *player_net_id else {
                send_error(
                    sender,
                    "network.input_before_hello",
                    "Input rejected before handshake",
                    false,
                )
                .await?;
                return Ok(true);
            };
            if !input_limiter.allow(1.0) {
                state.metrics.rate_limited_message();
                send_error(
                    sender,
                    "rate_limit.input",
                    "Input rate limit exceeded",
                    false,
                )
                .await?;
                return Ok(true);
            }
            match state.game.write().await.set_movement_input(
                entity_net_id,
                seq,
                client_tick,
                movement,
            ) {
                InputUpdateResult::Accepted | InputUpdateResult::Stale => {}
                InputUpdateResult::EntityMissing => {
                    send_error(
                        sender,
                        "world.player_missing",
                        "Player entity is missing",
                        true,
                    )
                    .await?;
                    return Ok(false);
                }
            }
        }

        ClientMessage::Interact { target } => {
            let Some(entity_net_id) = *player_net_id else {
                return Ok(true);
            };
            if !ui_limiter.allow(1.0) {
                state.metrics.rate_limited_message();
                send_error(
                    sender,
                    "rate_limit.interact",
                    "Interaction rate limit exceeded",
                    false,
                )
                .await?;
                return Ok(true);
            }
            if let Some(text) = state.game.write().await.interact(entity_net_id, target) {
                send(sender, &ServerMessage::System { text }).await?;
            }
        }

        ClientMessage::SnapshotAck { tick } => {
            snapshot_tracker.ack(tick);
        }

        ClientMessage::RequestFullState => {
            snapshot_tracker.force_full();
        }

        ClientMessage::UiAction {
            session_id,
            action,
            payload,
        } => {
            let Some(player) = *player_net_id else {
                return Ok(true);
            };
            if !ui_limiter.allow(1.0) {
                state.metrics.rate_limited_message();
                send_error(
                    sender,
                    "rate_limit.ui",
                    "UI action rate limit exceeded",
                    false,
                )
                .await?;
                return Ok(true);
            }
            if action.len() > 128 || session_id.len() > 160 {
                send_error(sender, "ui.invalid_action", "UI action is invalid", false).await?;
                return Ok(true);
            }
            if let Err(message) =
                state
                    .game
                    .write()
                    .await
                    .handle_ui_action(player, &session_id, action, payload)
            {
                send_error(sender, "ui.invalid_session", message, false).await?;
            }
        }

        ClientMessage::Chat { text } => {
            let Some(entity_net_id) = *player_net_id else {
                return Ok(true);
            };
            if !chat_limiter.allow(1.0) {
                state.metrics.rate_limited_message();
                send_error(sender, "rate_limit.chat", "Chat rate limit exceeded", false).await?;
                return Ok(true);
            }
            let text = text.trim();
            if text.is_empty() {
                return Ok(true);
            }
            let text = text.chars().take(MAX_CHAT_LENGTH).collect::<String>();
            let from = state
                .game
                .read()
                .await
                .player_name(entity_net_id)
                .unwrap_or_else(|| "Unknown".to_owned());
            let _ = state
                .events
                .send(OutboundMessage::broadcast(ServerMessage::Chat {
                    from,
                    text,
                }));
        }

        ClientMessage::Ping { nonce } => {
            let server_tick = state.game.read().await.current_tick();
            send(sender, &ServerMessage::Pong { nonce, server_tick }).await?;
        }
    }

    Ok(true)
}

#[derive(Default)]
struct SnapshotTracker {
    initialized: bool,
    force_full: bool,
    last_sent_tick: Option<u64>,
    last_acked_tick: Option<u64>,
    hashes: HashMap<EntityNetId, u64>,
}

impl SnapshotTracker {
    fn encode(&mut self, view: SnapshotView) -> ServerMessage {
        let SnapshotView {
            tick,
            last_processed_input_seq,
            last_processed_client_tick,
            visible_revisions,
            changed_entities,
        } = view;
        let next_hashes = visible_revisions.into_iter().collect::<HashMap<_, _>>();

        if !self.initialized || self.force_full {
            self.initialized = true;
            self.force_full = false;
            self.last_sent_tick = Some(tick);
            self.hashes = next_hashes;
            return ServerMessage::Snapshot {
                tick,
                last_processed_input_seq,
                last_processed_client_tick,
                entities: changed_entities,
            };
        }

        let baseline_tick = self.last_sent_tick.unwrap_or(tick);
        let mut spawns = Vec::new();
        let mut updates = Vec::new();
        for entity in changed_entities {
            if self.hashes.contains_key(&entity.net_id) {
                updates.push(entity);
            } else {
                spawns.push(entity);
            }
        }
        let mut despawns = self
            .hashes
            .keys()
            .filter(|net_id| !next_hashes.contains_key(net_id))
            .copied()
            .collect::<Vec<_>>();
        despawns.sort_unstable();
        self.hashes = next_hashes;
        self.last_sent_tick = Some(tick);

        ServerMessage::StateDelta {
            tick,
            baseline_tick,
            last_processed_input_seq,
            last_processed_client_tick,
            spawns,
            updates,
            despawns,
        }
    }

    fn needs_full(&self) -> bool {
        !self.initialized || self.force_full
    }

    fn ack(&mut self, tick: u64) {
        if self.last_acked_tick.is_none_or(|current| tick > current) {
            self.last_acked_tick = Some(tick);
        }
        if let (Some(sent), Some(acked)) = (self.last_sent_tick, self.last_acked_tick) {
            if sent.saturating_sub(acked) > 120 {
                self.force_full = true;
            }
        }
    }

    fn force_full(&mut self) {
        self.force_full = true;
    }
}

struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_per_second: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(refill_per_second: f64, capacity: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_per_second,
            last_refill: Instant::now(),
        }
    }

    fn allow(&mut self, cost: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.last_refill = now;
        self.tokens = (self.tokens + elapsed * self.refill_per_second).min(self.capacity);
        if self.tokens < cost {
            return false;
        }
        self.tokens -= cost;
        true
    }
}

fn valid_identity(identity: &str) -> bool {
    !identity.is_empty()
        && identity.len() <= MAX_IDENTITY_LENGTH
        && identity.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
        })
}

async fn send_error(
    sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    code: &str,
    message: &str,
    fatal: bool,
) -> Result<()> {
    send(
        sender,
        &ServerMessage::Error {
            code: code.to_owned(),
            message: message.to_owned(),
            fatal,
        },
    )
    .await
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

fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{valid_identity, TokenBucket};

    #[test]
    fn validates_identity_format() {
        assert!(valid_identity("guest-1234"));
        assert!(!valid_identity(""));
        assert!(!valid_identity("bad identity"));
    }

    #[test]
    fn token_bucket_limits_bursts() {
        let mut limiter = TokenBucket::new(1.0, 2.0);
        assert!(limiter.allow(1.0));
        assert!(limiter.allow(1.0));
        assert!(!limiter.allow(1.0));
        std::thread::sleep(Duration::from_millis(5));
        assert!(!limiter.allow(1.0));
    }
}
