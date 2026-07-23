use anyhow::Result;
use clap::Parser;
use honknet_core::Entity;
use honknet_math::Vec2;
use honknet_net_core::{decode_message, encode_message, Channel, NetworkMessage};
use honknet_net_transport::{NetworkTransport, TransportEvent, UdpTransport};
use honknet_runtime::{EngineRuntime, EngineRuntimeConfig, VelocityComponent};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};

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
    let transport = UdpTransport::bind(args.listen).await?;

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

    let dt = 1. / args.tick_rate.max(1) as f64;
    let mut interval = tokio::time::interval(Duration::from_secs_f64(dt));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = interval.tick() => {
                for event in transport.poll().await? {
                    match event {
                        TransportEvent::Connected(peer, _) => {
                            let spawn_pos = Vec2::new(peer as f32 % 8., 0.);
                            let e = runtime.spawn_player(peer, spawn_pos)?;

                            let (_, b, _) = encode_message(
                                &Welcome {
                                    peer,
                                    entity: e,
                                    tick: runtime.world.tick(),
                                },
                                false,
                            )?;
                            transport.send(peer, Channel::Control, Welcome::ID, &b).await?;
                        }
                        TransportEvent::Data(peer, _, kind, data) => {
                            if kind == Input::ID {
                                if let Ok(i) = decode_message::<Input>(&data, false, 4096) {
                                    if let Some(&e) = runtime.players.get(&peer) {
                                        let target_vel = i.movement.normalized() * 4.;
                                        if let Some(v) = runtime.world.get_mut::<VelocityComponent>(e) {
                                            v.0 = target_vel;
                                        }
                                        if let Some(b) = runtime.physics.bodies.get_mut(&e) {
                                            b.velocity = target_vel;
                                        }
                                    }
                                }
                            } else if kind == Hello::ID {
                                let _ = decode_message::<Hello>(&data, false, 4096);
                            }
                        }
                        TransportEvent::Disconnected(peer, _) => {
                            runtime.despawn_player(peer)?;
                        }
                    }
                }

                // Tick full EngineRuntime kernel
                runtime.tick(dt as f32)?;

                // Replicate state over network transport
                for (&peer, &e) in &runtime.players {
                    if let Some(b) = runtime.physics.bodies.get(&e) {
                        let (_, payload, _) = encode_message(
                            &State {
                                tick: runtime.world.tick(),
                                entity: e,
                                position: b.position,
                                velocity: b.velocity,
                            },
                            false,
                        )?;
                        transport.send(peer, Channel::UnreliableSequenced, State::ID, &payload).await?;
                    }
                }
            }
        }
    }
    Ok(())
}
