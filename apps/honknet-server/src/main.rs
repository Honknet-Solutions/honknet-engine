use anyhow::Result;
use clap::Parser;
use honknet_core::Entity;
use honknet_ecs::{Component, World};
use honknet_math::Vec2;
use honknet_net_core::{decode_message, encode_message, Channel, NetworkMessage};
use honknet_net_transport::{NetworkTransport, TransportEvent, UdpTransport};
use honknet_observability::{HealthState, Metrics};
use honknet_physics::{Body, Fixture, PhysicsWorld, Shape};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, time::Duration};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:3015")]
    listen: SocketAddr,
    #[arg(long, default_value_t = 30)]
    tick_rate: u32,
}

#[derive(Debug, Clone, Copy)]
struct PositionComponent(pub Vec2);
impl Component for PositionComponent {}

#[derive(Debug, Clone, Copy)]
struct VelocityComponent(pub Vec2);
impl Component for VelocityComponent {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct PlayerPeer(pub u64);
impl Component for PlayerPeer {}

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
    let mut world = World::default();
    let mut physics = PhysicsWorld::default();
    let mut players = HashMap::new();
    let metrics = Metrics::new();
    let health = HealthState::default();
    health.set_check("transport", true);

    let mut interval =
        tokio::time::interval(Duration::from_secs_f64(1. / args.tick_rate.max(1) as f64));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = interval.tick() => {
                for event in transport.poll().await? {
                    match event {
                        TransportEvent::Connected(peer, _) => {
                            let e = world.spawn();
                            let spawn_pos = Vec2::new(peer as f32 % 8., 0.);
                            world.insert(e, PlayerPeer(peer))?;
                            world.insert(e, PositionComponent(spawn_pos))?;
                            world.insert(e, VelocityComponent(Vec2::ZERO))?;
                            world.initialize(e)?;
                            players.insert(peer, e);

                            physics.insert(Body::dynamic(
                                e,
                                spawn_pos,
                                1.,
                                Fixture {
                                    shape: Shape::Circle { radius: 0.35 },
                                    friction: 0.5,
                                    restitution: 0.05,
                                    sensor: false,
                                    layer: 1,
                                    mask: 1,
                                },
                            ));
                            let (_, b, _compressed) = encode_message(
                                &Welcome {
                                    peer,
                                    entity: e,
                                    tick: world.tick(),
                                },
                                false,
                            )?;
                            transport.send(peer, Channel::Control, Welcome::ID, &b).await?;
                        }
                        TransportEvent::Data(peer, _, kind, data) => {
                            if kind == Input::ID {
                                if let Ok(i) = decode_message::<Input>(&data, false, 4096) {
                                    if let Some(&e) = players.get(&peer) {
                                        let target_vel = i.movement.normalized() * 4.;
                                        if let Some(v) = world.get_mut::<VelocityComponent>(e) {
                                            v.0 = target_vel;
                                        }
                                        if let Some(b) = physics.bodies.get_mut(&e) {
                                            b.velocity = target_vel;
                                        }
                                    }
                                }
                            } else if kind == Hello::ID {
                                let _ = decode_message::<Hello>(&data, false, 4096);
                            }
                        }
                        TransportEvent::Disconnected(peer, _) => {
                            if let Some(e) = players.remove(&peer) {
                                physics.remove(e);
                                let _ = world.despawn(e);
                            }
                        }
                    }
                }
                let dt = 1. / args.tick_rate.max(1) as f64;
                physics.step(dt as f32);

                // Sync physics bodies back into ECS World components
                for (peer, &e) in &players {
                    if let Some(b) = physics.bodies.get(&e) {
                        if let Some(pos) = world.get_mut::<PositionComponent>(e) {
                            pos.0 = b.position;
                        }
                        if let Some(vel) = world.get_mut::<VelocityComponent>(e) {
                            vel.0 = b.velocity;
                        }
                        let (_, payload, _) = encode_message(
                            &State {
                                tick: world.tick(),
                                entity: e,
                                position: b.position,
                                velocity: b.velocity,
                            },
                            false,
                        )?;
                        transport.send(*peer, Channel::UnreliableSequenced, State::ID, &payload).await?;
                    }
                }

                world.advance_tick();
                health.tick(world.tick());
                metrics.entities.set(players.len() as i64);
                metrics.physics_contacts.set(physics.events.len() as i64);
            }
        }
    }
    Ok(())
}
