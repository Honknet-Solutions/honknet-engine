use anyhow::Result;
use honknet_core::Entity;
use honknet_ecs::{Component, World};
use honknet_math::Vec2;
use honknet_prediction::PredictionBuffer;
use honknet_replication::{Delta, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServerEntityId(pub Entity);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalEntityId(pub Entity);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PersistentEntityId(pub u64);

#[derive(Default)]
pub struct EntityMapping {
    pub server_to_local: HashMap<ServerEntityId, LocalEntityId>,
    pub local_to_server: HashMap<LocalEntityId, ServerEntityId>,
    pub persistent_to_local: HashMap<PersistentEntityId, LocalEntityId>,
}

impl EntityMapping {
    pub fn insert(&mut self, server: ServerEntityId, local: LocalEntityId) {
        self.server_to_local.insert(server, local);
        self.local_to_server.insert(local, server);
    }
    pub fn remove_server(&mut self, server: ServerEntityId) -> Option<LocalEntityId> {
        if let Some(local) = self.server_to_local.remove(&server) {
            self.local_to_server.remove(&local);
            Some(local)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientConnectionState {
    Disconnected,
    TransportConnecting,
    ProtocolHello,
    Authenticating,
    LoadingManifest,
    SynchronizingWorld,
    Active,
    Reconnecting,
    Closed,
}

#[derive(Debug, Clone, Copy)]
pub struct PositionComponent(pub Vec2);
impl Component for PositionComponent {}

#[derive(Debug, Clone, Copy)]
pub struct VelocityComponent(pub Vec2);
impl Component for VelocityComponent {}

#[derive(Default)]
pub struct ClockSynchronizer {
    pub server_tick: u64,
    pub rtt_ms: f32,
    pub tick_lead: u32,
}

impl ClockSynchronizer {
    pub fn update(&mut self, server_tick: u64, rtt_ms: f32, tick_rate: f32) {
        self.server_tick = server_tick;
        self.rtt_ms = rtt_ms;
        self.tick_lead = ((rtt_ms / 1000.0) * tick_rate).ceil() as u32;
    }
}

pub struct ClientRuntime {
    pub state: ClientConnectionState,
    pub world: World,
    pub prediction: PredictionBuffer,
    pub clock: ClockSynchronizer,
    pub last_acked_baseline: u64,
    pub predicted_position: Vec2,
    pub entity_mapping: EntityMapping,
    pub input_queue: VecDeque<(u64, Vec2)>,
    pub client_tick: u64,
    pub interpolation_clock: f32,
    pub prediction_clock: f32,
    pub render_clock: f32,
    pub tick_rate: f32,
}

impl Default for ClientRuntime {
    fn default() -> Self {
        Self {
            state: ClientConnectionState::Disconnected,
            world: World::default(),
            prediction: PredictionBuffer::new(64),
            clock: ClockSynchronizer::default(),
            last_acked_baseline: 0,
            predicted_position: Vec2::ZERO,
            entity_mapping: EntityMapping::default(),
            input_queue: VecDeque::new(),
            client_tick: 0,
            interpolation_clock: 0.0,
            prediction_clock: 0.0,
            render_clock: 0.0,
            tick_rate: 30.0,
        }
    }
}

impl ClientRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_state(&mut self, state: ClientConnectionState) {
        self.state = state;
    }

    pub fn apply_snapshot(&mut self, snapshot: &Snapshot) {
        self.clock.server_tick = snapshot.tick;
        self.last_acked_baseline = snapshot.tick;

        for e_state in &snapshot.entities {
            let server_id = ServerEntityId(e_state.entity);
            let local_entity =
                if let Some(&local_id) = self.entity_mapping.server_to_local.get(&server_id) {
                    local_id.0
                } else {
                    let new_local = self.world.spawn();
                    self.entity_mapping
                        .insert(server_id, LocalEntityId(new_local));
                    new_local
                };

            if let Some(pos) = self.world.get_mut::<PositionComponent>(local_entity) {
                pos.0 = e_state.position;
            } else {
                let _ = self
                    .world
                    .insert(local_entity, PositionComponent(e_state.position));
            }
        }
    }

    pub fn apply_delta(&mut self, delta: &Delta) {
        self.clock.server_tick = delta.tick;
        self.last_acked_baseline = delta.tick;

        for spawn in &delta.spawns {
            let local_entity = self.world.spawn();
            self.entity_mapping
                .insert(ServerEntityId(spawn.entity), LocalEntityId(local_entity));
            let _ = self
                .world
                .insert(local_entity, PositionComponent(spawn.position));
        }

        for update in &delta.updates {
            if let Some(&local_id) = self
                .entity_mapping
                .server_to_local
                .get(&ServerEntityId(update.entity))
            {
                if let Some(pos) = self.world.get_mut::<PositionComponent>(local_id.0) {
                    pos.0 = update.position;
                }
            }
        }

        for despawn in &delta.despawns {
            if let Some(local_id) = self.entity_mapping.remove_server(ServerEntityId(*despawn)) {
                let _ = self.world.despawn(local_id.0);
            }
        }
    }

    pub fn enqueue_input(&mut self, seq: u64, movement: Vec2) {
        if self.input_queue.len() >= 128 {
            self.input_queue.pop_front();
        }
        self.input_queue.push_back((seq, movement));
        // Predict local movement
        let dt = if self.tick_rate > 0.0 {
            1.0 / self.tick_rate
        } else {
            1.0 / 30.0
        };
        self.predicted_position += movement * dt;
    }

    pub fn clean_confirmed_input(&mut self, acked_seq: u64) {
        while let Some(&(seq, _)) = self.input_queue.front() {
            if seq <= acked_seq {
                self.input_queue.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn tick(&mut self, _delta_seconds: f32) -> Result<()> {
        if self.state != ClientConnectionState::Active {
            return Ok(());
        }

        self.client_tick += 1;
        self.world.advance_tick();
        Ok(())
    }
}

pub struct RenderFrame {
    pub tick: u64,
    pub interpolation_alpha: f32,
    pub cameras: Vec<RenderCamera>,
    pub sprites: Vec<RenderSprite>,
    pub tiles: Vec<RenderChunkUpdate>,
    pub lights: Vec<RenderLight>,
    pub particles: Vec<RenderParticle>,
    pub ui_commands: Vec<RenderUiCommand>,
    pub removals: Vec<RenderObjectId>,
}

pub struct RenderSprite {
    pub render_id: u64,
    pub entity_id: u64,
    pub asset_id: u32,
    pub state_id: u16,
    pub frame_id: u16,
    pub direction: u8,
    pub layer: i16,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub color: u32,
    pub alpha: f32,
    pub flags: u32,
}

pub struct RenderCamera {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

pub struct RenderChunkUpdate {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub tiles: Vec<u16>,
}

pub struct RenderLight {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub color: u32,
    pub intensity: f32,
    pub radius: f32,
}

pub struct RenderParticle {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub color: u32,
    pub lifetime: f32,
}

pub struct RenderUiCommand {
    pub command: String,
    pub payload: String,
}

pub type RenderObjectId = u64;
