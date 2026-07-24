use anyhow::Result;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use honknet_game::GameApplication;
use honknet_math::Vec2;
use honknet_net_core::{
    decode_message, encode_message_envelope, ClientHelloPayload, ClientInputPayload,
    NetworkMessage, NetworkPacketEnvelope, ServerWelcomePayload, StateAckPayload, BUILD_VERSION,
    CONTENT_MANIFEST_ID, CONTENT_VERSION, PROTOCOL_VERSION,
};
use honknet_net_server::WsServer;
use honknet_replication::{EntityState, Snapshot};
use honknet_runtime::{EngineRuntimeConfig, PlayerPeer, VelocityComponent};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:3015")]
    listen: SocketAddr,
    #[arg(long, default_value_t = 30)]
    tick_rate: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args = Args::parse();

    let auth_key = std::env::var("HONKNET_AUTH_KEY").unwrap_or_else(|_| {
        info!("Notice: Using default development HONKNET_AUTH_KEY");
        "honknet-auth-key-dev".to_string()
    });
    let session_key = std::env::var("HONKNET_SESSION_KEY")
        .unwrap_or_else(|_| "honknet-session-key-dev".to_string());
    let reconnect_key = std::env::var("HONKNET_RECONNECT_KEY")
        .unwrap_or_else(|_| "honknet-reconnect-key-dev".to_string());

    let runtime = GameApplication::new(EngineRuntimeConfig {
        tick_rate: args.tick_rate,
        listen_address: args.listen.to_string(),
        persistence_path: None,
        replay_path: None,
        auth_signing_key: auth_key.into_bytes(),
        session_key: session_key.into_bytes(),
        reconnect_key: reconnect_key.into_bytes(),
    })?
    .initialize()?
    .into_runtime();

    let ws_server = Arc::new(Mutex::new(WsServer::new()));
    let listener = TcpListener::bind(args.listen).await?;
    info!("Honknet WebSocket Server listening on {}", args.listen);

    let runtime = Arc::new(Mutex::new(runtime));
    let peer_counter = Arc::new(Mutex::new(1000u64));

    let ws_server_clone = Arc::clone(&ws_server);
    let runtime_clone = Arc::clone(&runtime);
    let peer_counter_clone = Arc::clone(&peer_counter);

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            if let Ok(ws) = tokio_tungstenite::accept_async(stream).await {
                let mut p_guard = peer_counter_clone.lock().await;
                let peer_id = *p_guard;
                *p_guard += 1;
                drop(p_guard);

                let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
                ws_server_clone.lock().await.handle_connection(peer_id, tx);

                let (mut ws_sender, mut ws_receiver) = ws.split();

                // Writer task
                tokio::spawn(async move {
                    while let Some(bytes) = rx.recv().await {
                        if ws_sender
                            .send(tokio_tungstenite::tungstenite::Message::Binary(
                                bytes.into(),
                            ))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                });

                // Reader task
                let runtime_reader = Arc::clone(&runtime_clone);
                let ws_srv_reader = Arc::clone(&ws_server_clone);

                tokio::spawn(async move {
                    while let Some(Ok(msg)) = ws_receiver.next().await {
                        if let tokio_tungstenite::tungstenite::Message::Binary(data) = msg {
                            if let Ok((env, payload)) = NetworkPacketEnvelope::decode(&data) {
                                let compressed = (env.flags & 4) != 0;

                                match env.message_id {
                                    ClientHelloPayload::ID => {
                                        if let Ok(hello) = decode_message::<ClientHelloPayload>(
                                            payload, compressed, 4096,
                                        ) {
                                            info!(
                                                "Peer {} Handshake: protocol={}, engine={}",
                                                peer_id,
                                                hello.protocol_version,
                                                hello.engine_version
                                            );

                                            if hello.protocol_version != PROTOCOL_VERSION
                                                || hello.engine_version != BUILD_VERSION
                                                || hello.content_version != CONTENT_VERSION
                                                || hello.content_manifest_hash
                                                    != CONTENT_MANIFEST_ID
                                            {
                                                info!(
                                                    "Rejecting peer {}: incompatible Honknet build",
                                                    peer_id
                                                );
                                                break;
                                            }

                                            let mut r = runtime_reader.lock().await;
                                            let mut reattached = false;
                                            if let Some(token) = &hello.reconnect_token {
                                                if let Some(id_str) = token.strip_prefix("rec-") {
                                                    if let Ok(old_peer) = id_str.parse::<u64>() {
                                                        if let Some(existing_entity) =
                                                            r.players.remove(&old_peer)
                                                        {
                                                            r.players
                                                                .insert(peer_id, existing_entity);
                                                            if let Some(p) =
                                                                r.world.get_mut::<PlayerPeer>(
                                                                    existing_entity,
                                                                )
                                                            {
                                                                p.0 = peer_id;
                                                            }
                                                            reattached = true;
                                                            info!("Peer {} re-attached to existing character entity for reconnect token {}", peer_id, token);
                                                        }
                                                    }
                                                }
                                            }

                                            if !reattached {
                                                let spawn_pos = Vec2::new(
                                                    ((peer_id % 8) as f32) * 50.0 - 150.0,
                                                    0.0,
                                                );
                                                let _ = r.spawn_player(peer_id, spawn_pos);
                                            }

                                            let welcome = ServerWelcomePayload {
                                                protocol_version: PROTOCOL_VERSION,
                                                engine_version: BUILD_VERSION.to_string(),
                                                content_version: CONTENT_VERSION.to_string(),
                                                content_manifest_hash: CONTENT_MANIFEST_ID
                                                    .to_string(),
                                                auth_token: Some("auth-ok".to_string()),
                                                reconnect_token: Some(format!("rec-{peer_id}")),
                                                server_tick: r.world.tick(),
                                                peer_id,
                                                tick_rate: args.tick_rate,
                                                session_token: format!("session-{peer_id}"),
                                            };

                                            if let Ok(env_payload) = encode_message_envelope(
                                                &welcome,
                                                r.world.tick(),
                                                false,
                                            ) {
                                                let mut ws_srv = ws_srv_reader.lock().await;
                                                ws_srv.send_to(peer_id, env_payload);
                                            }
                                        }
                                    }
                                    ClientInputPayload::ID => {
                                        if let Ok(input) = decode_message::<ClientInputPayload>(
                                            payload, compressed, 4096,
                                        ) {
                                            let mut r = runtime_reader.lock().await;
                                            if let Some(&e) = r.players.get(&peer_id) {
                                                let speed = 250.0;
                                                let target_vel =
                                                    if input.movement.length_squared() > 0.0 {
                                                        input.movement.normalized() * speed
                                                    } else {
                                                        Vec2::ZERO
                                                    };

                                                if let Some(v) =
                                                    r.world.get_mut::<VelocityComponent>(e)
                                                {
                                                    v.0 = target_vel;
                                                }
                                                if let Some(b) = r.physics.bodies.get_mut(&e) {
                                                    b.velocity = target_vel;
                                                }
                                            }
                                        }
                                    }
                                    StateAckPayload::ID => {
                                        if let Ok(ack) = decode_message::<StateAckPayload>(
                                            payload, compressed, 1024,
                                        ) {
                                            let mut r = runtime_reader.lock().await;
                                            r.client_baselines.insert(peer_id, ack.acked_tick);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    // On Disconnect: Cleanup connection in WsServer so queues & tasks drop,
                    // but retain character entity in world for reconnect!
                    info!("Client disconnected (peer_id: {})", peer_id);
                    ws_srv_reader.lock().await.disconnect_client(peer_id);
                });
            }
        }
    });

    let dt = 1. / args.tick_rate.max(1) as f64;
    let mut interval = tokio::time::interval(Duration::from_secs_f64(dt));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = interval.tick() => {
                let mut r = runtime.lock().await;
                r.tick(dt as f32)?;

                let mut ws_srv = ws_server.lock().await;

                // Replicate snapshot across connected peers
                let mut entities_state = Vec::new();
                for (&peer, &e) in &r.players {
                    let pos = if let Some(b) = r.physics.bodies.get(&e) {
                        b.position
                    } else {
                        Vec2::ZERO
                    };
                    let vel = if let Some(b) = r.physics.bodies.get(&e) {
                        b.velocity
                    } else {
                        Vec2::ZERO
                    };

                    let transform_comp = honknet_replication::ComponentState::encode(
                        honknet_replication::NET_ID_TRANSFORM,
                        r.world.tick(),
                        honknet_replication::ReplicationMode::Replicated,
                        &honknet_replication::NetTransformComponent {
                            position: pos,
                            rotation: 0.0,
                            parent_entity: None,
                        },
                    );

                    let physics_comp = honknet_replication::ComponentState::encode(
                        honknet_replication::NET_ID_PHYSICS,
                        r.world.tick(),
                        honknet_replication::ReplicationMode::Replicated,
                        &honknet_replication::NetPhysicsComponent {
                            velocity: vel,
                            angular_velocity: 0.0,
                            mass: 70.0,
                            body_type: 1,
                        },
                    );

                    let meta_comp = honknet_replication::ComponentState::encode(
                        honknet_replication::NET_ID_METADATA,
                        r.world.tick(),
                        honknet_replication::ReplicationMode::Replicated,
                        &honknet_replication::NetMetadataComponent {
                            name: format!("Player-{peer}"),
                            description: "Human Engineer".to_string(),
                            prototype_id: "MobHuman".to_string(),
                        },
                    );

                    let sprite_comp = honknet_replication::ComponentState::encode(
                        honknet_replication::NET_ID_SPRITE,
                        r.world.tick(),
                        honknet_replication::ReplicationMode::Replicated,
                        &honknet_replication::NetSpriteComponent {
                            rsi_path: "Mobs/Species/Human/human.rsi".to_string(),
                            state: "human_idle".to_string(),
                            direction: 0,
                            layer: 0,
                            color: 0x00f0ff,
                            visible: true,
                        },
                    );

                    entities_state.push(EntityState {
                        entity: e,
                        revision: r.world.tick(),
                        position: pos,
                        owner: Some(peer),
                        importance: 1.0,
                        frequency: 1,
                        components: vec![transform_comp, physics_comp, meta_comp, sprite_comp],
                    });
                }

                let snapshot = Snapshot {
                    tick: r.world.tick(),
                    entities: entities_state,
                };

                let current_tick = r.world.tick();
                let peers: Vec<u64> = ws_srv.clients.keys().copied().collect();
                for peer in peers {
                    if let Some(delta) = r.build_client_delta(peer, 64 * 1024) {
                        if let Ok(env_payload) = encode_message_envelope(&delta, current_tick, false) {
                            ws_srv.send_to(peer, env_payload);
                        }
                    } else if let Ok(env_payload) = encode_message_envelope(&snapshot, current_tick, false) {
                        ws_srv.send_to(peer, env_payload);
                    }
                    r.client_baselines.insert(peer, current_tick);
                }

                ws_srv.update();
            }
        }
    }

    Ok(())
}
