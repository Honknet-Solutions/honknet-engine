use anyhow::Result;
use honknet_core::Entity;
use honknet_ecs::{Component, World};
use honknet_math::Vec2;
use honknet_prediction::PredictionBuffer;
use honknet_replication::{Delta, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

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
    pub fn update(&mut self, server_tick: u64, rtt_ms: f32) {
        self.server_tick = server_tick;
        self.rtt_ms = rtt_ms;
        self.tick_lead = ((rtt_ms / 1000.0) * 30.0).ceil() as u32;
    }
}

pub struct ClientRuntime {
    pub state: ClientConnectionState,
    pub world: World,
    pub prediction: PredictionBuffer,
    pub clock: ClockSynchronizer,
    pub last_acked_baseline: u64,
    pub predicted_position: Vec2,
    pub server_entity_map: HashMap<Entity, Entity>,
    pub input_queue: VecDeque<(u64, Vec2)>,
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
            server_entity_map: HashMap::new(),
            input_queue: VecDeque::new(),
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
            let local_entity = *self
                .server_entity_map
                .entry(e_state.entity)
                .or_insert_with(|| self.world.spawn());

            let _ = self
                .world
                .insert(local_entity, PositionComponent(e_state.position));
        }
    }

    pub fn apply_delta(&mut self, delta: &Delta) {
        self.clock.server_tick = delta.tick;
        self.last_acked_baseline = delta.tick;

        for spawn in &delta.spawns {
            let local_entity = self.world.spawn();
            self.server_entity_map.insert(spawn.entity, local_entity);
            let _ = self
                .world
                .insert(local_entity, PositionComponent(spawn.position));
        }

        for update in &delta.updates {
            if let Some(&local_e) = self.server_entity_map.get(&update.entity) {
                if let Some(pos) = self.world.get_mut::<PositionComponent>(local_e) {
                    pos.0 = update.position;
                }
            }
        }

        for despawn in &delta.despawns {
            if let Some(local_e) = self.server_entity_map.remove(despawn) {
                let _ = self.world.despawn(local_e);
            }
        }
    }

    pub fn enqueue_input(&mut self, seq: u64, movement: Vec2) {
        self.input_queue.push_back((seq, movement));
        // Predict local movement
        self.predicted_position += movement * (1.0 / 30.0);
    }

    pub fn tick(&mut self, _delta_seconds: f32) -> Result<()> {
        if self.state != ClientConnectionState::Active {
            return Ok(());
        }

        self.world.advance_tick();
        Ok(())
    }
}
