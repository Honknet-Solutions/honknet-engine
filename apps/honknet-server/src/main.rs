use anyhow::Result;
use clap::Parser;
чuse futures_util::{SinkExt, StreamExt};
use honknet_core::Entity;
use honknet_math::Vec2;
use honknet_net_core::{decode_message, encode_message, NetworkMessage};
use honknet_net_server::WsServer;
use honknet_runtime::{EngineRuntime, EngineRuntimeConfig, VelocityComponent};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize)]
struct Hello {
    client_version: String,
}

impl NetworkMessage for Hello {
    const ID: u16 = 1;
}

#[derive(Debug, Serialize, Deserialize)]
struct Input {
    sequence: u32,
    movement: Vec2,
}

impl NetworkMessage for Input {
    const ID: u16 = 2;
}

#[derive(Debug, Serialize, Deserialize)]
struct Welcome {
    peer: u64,
    entity: Entity,
    tick: u64,
}

impl NetworkMessage for Welcome {
    const ID: u16 = 3;
}

#[derive(Debug, Serialize, Deserialize)]
struct State {
    tick: u64,
    entity: Entity,
    position: Vec2,
    velocity: Vec2,
}

impl NetworkMessage for State {
    const ID: u16 = 4;
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args = Args::parse();

    let mut runtime = EngineRuntime::new(EngineRuntimeConfig {
        tick_rate: args.tick_rate,
        listen_address: args.listen.to_string(),
        persistence_path: None,
        replay_path: None,
        auth_signing_key: std::env::var("HONKNET_AUTH_KEY")
            .unwrap_or_else(|_| "honknet-auth-key-1337".to_string())
            .into_bytes(),
        session_key: std::env::var("HONKNET_SESSION_KEY")
            .unwrap_or_else(|_| "honknet-session-key-1337".to_string())
            .into_bytes(),
        reconnect_key: std::env::var("HONKNET_RECONNECT_KEY")
            .unwrap_or_else(|_| "honknet-reconnect-key-1337".to_string())
            .into_bytes(),
    })?;

    runtime.initialize();
    runtime.load_content();
    runtime.ready();
    runtime.start();

    let ws_server = Arc::new(Mutex::new(WsServer::new()));

    // Bind TCP listener for WebSocket connections
    let listener = TcpListener::bind(args.listen).await?;
    info!("Honknet WebSocket Server active on {}", args.listen);

    let runtime = Arc::new(Mutex::new(runtime));
    let peer_counter = Arc::new(Mutex::new(1000u64));

    // Spawn TCP accept loop
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

                // Spawn player in runtime
                {
                    let mut r = runtime_clone.lock().await;
                    let spawn_pos = Vec2::new((peer_id % 8) as f32, 0.);
                    if let Ok(e) = r.spawn_player(peer_id, spawn_pos) {
                        let welcome = Welcome {
                            peer: peer_id,
                            entity: e,
                            tick: r.world.tick(),
                        };
                        if let Ok((_, payload, _)) = encode_message(&welcome, false) {
                            let mut ws_srv = ws_server_clone.lock().await;
                            ws_srv.send_to(peer_id, payload);
                        }
                    }
                }

                let (mut ws_sender, mut ws_receiver) = ws.split();

                // Spawn WebSocket writer task
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

                // Spawn WebSocket reader task
                let runtime_reader = Arc::clone(&runtime_clone);
                tokio::spawn(async move {
                    while let Some(Ok(msg)) = ws_receiver.next().await {
                        if let tokio_tungstenite::tungstenite::Message::Binary(data) = msg {
                            if let Ok(input) = decode_message::<Input>(&data, false, 4096) {
                                let mut r = runtime_reader.lock().await;
                                if let Some(&e) = r.players.get(&peer_id) {
                                    let target_vel = input.movement.normalized() * 4.;
                                    if let Some(v) = r.world.get_mut::<VelocityComponent>(e) {
                                        v.0 = target_vel;
                                    }
                                    if let Some(b) = r.physics.bodies.get_mut(&e) {
                                        b.velocity = target_vel;
                                    }
                                }
                            }
                        }
                    }
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

                // Replicate state over WebSocket connection
                for (&peer, &e) in &r.players {
                    if let Some(b) = r.physics.bodies.get(&e) {
                        if let Ok((_, payload, _)) = encode_message(
                            &State {
                                tick: r.world.tick(),
                                entity: e,
                                position: b.position,
                                velocity: b.velocity,
                            },
                            false,
                        ) {
                            ws_srv.send_to(peer, payload);
                        }
                    }
                }

                ws_srv.update();
            }
        }
    }

    Ok(())
}
