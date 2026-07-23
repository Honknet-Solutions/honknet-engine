use anyhow::Result;
use honknet_admin::AdminConsole;
use honknet_auth::TokenIssuer;
use honknet_core::Entity;
use honknet_ecs::{Component, World};
use honknet_events::EventBus;
use honknet_map::Map;
use honknet_math::Vec2;
use honknet_observability::{HealthState, Metrics};
use honknet_persistence::FileBackend;
use honknet_physics::{Body, Fixture, PhysicsWorld, Shape};
use honknet_prediction::PredictionBuffer;
use honknet_prototypes::PrototypeManager;
use honknet_replay::{ReplayHeader, ReplayRecorder};
use honknet_replication::{Replicator, SpatialProvider};
use honknet_resources::Vfs;
use honknet_scheduler::{Scheduler, System};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct PositionComponent(pub Vec2);
impl Component for PositionComponent {}

#[derive(Debug, Clone, Copy)]
pub struct VelocityComponent(pub Vec2);
impl Component for VelocityComponent {}

#[derive(Debug, Clone, Copy)]
pub struct PlayerPeer(pub u64);
impl Component for PlayerPeer {}

pub struct EngineRuntimeConfig {
    pub tick_rate: u32,
    pub listen_address: String,
    pub persistence_path: Option<PathBuf>,
    pub replay_path: Option<PathBuf>,
}

impl Default for EngineRuntimeConfig {
    fn default() -> Self {
        Self {
            tick_rate: 30,
            listen_address: "127.0.0.1:3015".to_string(),
            persistence_path: None,
            replay_path: None,
        }
    }
}

pub struct EngineRuntime {
    pub world: World,
    pub scheduler: Scheduler,
    pub event_bus: EventBus,
    pub physics: PhysicsWorld,
    pub map: Map,
    pub vfs: Vfs,
    pub prototypes: PrototypeManager,
    pub replication: Replicator,
    pub prediction: PredictionBuffer,
    pub persistence: Option<FileBackend>,
    pub replay: Option<ReplayRecorder>,
    pub auth: TokenIssuer,
    pub admin: AdminConsole,
    pub metrics: Metrics,
    pub health: HealthState,
    pub config: EngineRuntimeConfig,
    pub players: HashMap<u64, Entity>,
    pub client_baselines: HashMap<u64, u64>,
}

impl EngineRuntime {
    pub fn new(config: EngineRuntimeConfig) -> Result<Self> {
        let world = World::default();
        let scheduler = Scheduler::default();
        let event_bus = EventBus::default();
        let physics = PhysicsWorld::default();
        let map = Map {
            id: 1,
            tile_size: 1.0,
            tiles: vec![],
            grids: HashMap::new(),
            metadata: HashMap::new(),
            streaming_regions: vec![],
        };
        let vfs = Vfs::default();
        let prototypes = PrototypeManager::default();

        let mut replication = Replicator::default();
        replication
            .providers
            .push(Box::new(SpatialProvider { radius: 32.0 }));

        let prediction = PredictionBuffer::new(64);

        let persistence = config
            .persistence_path
            .as_ref()
            .map(|p| FileBackend::new(p, 3));

        let auth = TokenIssuer::new(b"honknet-secret");
        let admin = AdminConsole::default();
        let metrics = Metrics::new();
        let health = HealthState::default();
        health.set_check("runtime", true);

        let replay = if let Some(ref path) = config.replay_path {
            let header = ReplayHeader {
                engine_version: "1.0.0-rc.1".to_string(),
                protocol: 1,
                content_hash: "initial-hash".to_string(),
                initial_state: vec![],
                seed: 1337,
            };
            Some(ReplayRecorder::create(path, &header)?)
        } else {
            None
        };

        Ok(Self {
            world,
            scheduler,
            event_bus,
            physics,
            map,
            vfs,
            prototypes,
            replication,
            prediction,
            persistence,
            replay,
            auth,
            admin,
            metrics,
            health,
            config,
            players: HashMap::new(),
            client_baselines: HashMap::new(),
        })
    }

    pub fn register_system<S: System + 'static>(&mut self, system: S) {
        self.scheduler.add(system);
    }

    pub fn spawn_player(&mut self, peer: u64, position: Vec2) -> Result<Entity> {
        let e = self.world.spawn();
        self.world.insert(e, PlayerPeer(peer))?;
        self.world.insert(e, PositionComponent(position))?;
        self.world.insert(e, VelocityComponent(Vec2::ZERO))?;
        self.world.initialize(e)?;

        self.physics.insert(Body::dynamic(
            e,
            position,
            1.0,
            Fixture {
                shape: Shape::Circle { radius: 0.35 },
                friction: 0.5,
                restitution: 0.05,
                sensor: false,
                layer: 1,
                mask: 1,
            },
        ));

        self.players.insert(peer, e);
        Ok(e)
    }

    pub fn despawn_player(&mut self, peer: u64) -> Result<()> {
        if let Some(e) = self.players.remove(&peer) {
            self.physics.remove(e);
            self.world.despawn(e)?;
        }
        Ok(())
    }

    pub fn tick(&mut self, delta_seconds: f32) -> Result<()> {
        // 1. Run ECS Scheduler Systems
        self.scheduler.run(&mut self.world, delta_seconds).ok();

        // 2. Physics Simulation Step
        self.physics.step(delta_seconds);

        // 3. Sync Physics Positions back into ECS World Components
        for (&_peer, &e) in &self.players {
            if let Some(body) = self.physics.bodies.get(&e) {
                if let Some(pos) = self.world.get_mut::<PositionComponent>(e) {
                    pos.0 = body.position;
                }
                if let Some(vel) = self.world.get_mut::<VelocityComponent>(e) {
                    vel.0 = body.velocity;
                }
            }
        }

        // 4. Update Replicator states for spatial PVS
        for (&peer, &e) in &self.players {
            if let Some(pos) = self.world.get::<PositionComponent>(e) {
                self.replication.states.insert(
                    e,
                    honknet_replication::EntityState {
                        entity: e,
                        revision: self.world.tick(),
                        position: pos.0,
                        owner: Some(peer),
                        importance: 100.0,
                        frequency: 1,
                        components: vec![],
                    },
                );
            }
        }

        // 5. Replication & Metrics Update
        self.world.advance_tick();
        self.health.tick(self.world.tick());
        self.metrics.entities.set(self.players.len() as i64);
        self.metrics
            .physics_contacts
            .set(self.physics.events.len() as i64);

        Ok(())
    }

    pub fn sync_map_physics(&mut self) {
        let tile_size = self.map.tile_size;
        for grid in self.map.grids.values_mut() {
            for (&(cx, cy), chunk) in grid.chunks.iter_mut() {
                if chunk.collision_dirty {
                    for x in 0..honknet_map::CHUNK_SIZE {
                        for y in 0..honknet_map::CHUNK_SIZE {
                            let idx = (y * honknet_map::CHUNK_SIZE + x) as usize;
                            let tile_id = chunk.tiles[idx] as usize;
                            if let Some(def) = self.map.tiles.get(tile_id) {
                                if def.solid {
                                    let world_pos = Vec2::new(
                                        (cx * honknet_map::CHUNK_SIZE + x) as f32 * tile_size,
                                        (cy * honknet_map::CHUNK_SIZE + y) as f32 * tile_size,
                                    );
                                    let dummy_e = Entity::new((cx * 1000 + x) as u32, 0);
                                    self.physics.insert(Body::static_body(
                                        dummy_e,
                                        world_pos,
                                        Fixture {
                                            shape: Shape::Box {
                                                half: Vec2::new(tile_size * 0.5, tile_size * 0.5),
                                            },
                                            friction: def.friction,
                                            restitution: 0.0,
                                            sensor: false,
                                            layer: 1,
                                            mask: 1,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                    chunk.collision_dirty = false;
                }
            }
        }
    }

    pub fn build_client_snapshot(
        &mut self,
        peer: u64,
        byte_budget: usize,
    ) -> honknet_replication::Snapshot {
        let pos = self
            .players
            .get(&peer)
            .and_then(|&e| self.world.get::<PositionComponent>(e))
            .map(|p| p.0)
            .unwrap_or(Vec2::ZERO);

        let ctx = honknet_replication::InterestContext {
            client: peer,
            controlled: self.players.get(&peer).copied(),
            position: pos,
            observers: std::collections::HashSet::new(),
            forced: std::collections::HashSet::new(),
            teams: std::collections::HashSet::new(),
        };

        self.replication
            .build_snapshot(self.world.tick(), &ctx, byte_budget)
    }

    pub fn build_client_delta(
        &mut self,
        peer: u64,
        byte_budget: usize,
    ) -> Option<honknet_replication::Delta> {
        let snapshot = self.build_client_snapshot(peer, byte_budget);
        if let Some(&baseline) = self.client_baselines.get(&peer) {
            self.replication.delta(baseline, &snapshot)
        } else {
            Some(honknet_replication::Delta {
                tick: snapshot.tick,
                baseline: 0,
                spawns: snapshot.entities,
                updates: vec![],
                despawns: vec![],
            })
        }
    }
}

#[derive(Default)]
pub struct EngineBuilder {
    config: EngineRuntimeConfig,
    systems: Vec<Box<dyn System>>,
}

impl EngineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick_rate(mut self, rate: u32) -> Self {
        self.config.tick_rate = rate;
        self
    }

    pub fn listen_address(mut self, addr: impl Into<String>) -> Self {
        self.config.listen_address = addr.into();
        self
    }

    pub fn with_system<S: System + 'static>(mut self, system: S) -> Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn build(self) -> Result<EngineRuntime> {
        let mut runtime = EngineRuntime::new(self.config)?;
        for s in self.systems {
            runtime.scheduler.add_boxed(s);
        }
        Ok(runtime)
    }
}
