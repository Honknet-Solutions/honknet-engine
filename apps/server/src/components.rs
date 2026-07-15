use std::{
    collections::{BTreeMap, VecDeque},
    time::Instant,
};

use honknet_protocol::{EntityNetId, PlayerIdentityId, Vec2};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_PENDING_INPUTS: usize = 512;

#[derive(Debug, Clone)]
pub struct PlayerComponent {
    pub identity_id: PlayerIdentityId,
    pub display_name: String,
    pub online: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerInputCommand {
    pub sequence: u32,
    pub client_tick: u32,
    pub movement: Vec2,
}

#[derive(Debug, Clone)]
pub struct PlayerInputComponent {
    pub last_received_sequence: Option<u32>,
    pub last_processed_sequence: Option<u32>,
    pub last_processed_client_tick: Option<u32>,
    pub last_received_at: Option<Instant>,
    pending: VecDeque<PlayerInputCommand>,
}

impl PlayerInputComponent {
    pub fn new() -> Self {
        Self {
            last_received_sequence: None,
            last_processed_sequence: None,
            last_processed_client_tick: None,
            last_received_at: None,
            pending: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, command: PlayerInputCommand) {
        if self.pending.len() >= MAX_PENDING_INPUTS {
            let _ = self.pending.pop_front();
        }
        self.pending.push_back(command);
    }

    pub fn pop_next(&mut self) -> Option<PlayerInputCommand> {
        let command = self.pending.pop_front()?;
        self.last_processed_sequence = Some(command.sequence);
        self.last_processed_client_tick = Some(command.client_tick);
        Some(command)
    }

    pub fn stop(&mut self) {
        self.pending.clear();
        self.last_received_at = None;
    }
}

impl Default for PlayerInputComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ColliderComponent {
    pub radius: f32,
    pub collision_layer: u32,
    pub collision_mask: u32,
    pub sensor: bool,
}

impl ColliderComponent {
    pub const fn solid_circle(radius: f32) -> Self {
        Self {
            radius,
            collision_layer: 1,
            collision_mask: u32::MAX,
            sensor: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DoorComponent {
    pub open: bool,
}

#[derive(Debug, Clone)]
pub struct ItemComponent {
    pub name: String,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryEntry {
    pub entity_net_id: EntityNetId,
    pub prototype: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryComponent {
    pub capacity: u32,
    pub items: Vec<InventoryEntry>,
}

impl InventoryComponent {
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            items: Vec::new(),
        }
    }

    pub fn can_insert(&self) -> bool {
        self.items.len() < self.capacity as usize
    }
}

impl Default for InventoryComponent {
    fn default() -> Self {
        Self::new(24)
    }
}

#[derive(Debug, Clone)]
pub struct ContainedInComponent {
    pub owner_net_id: EntityNetId,
}

#[derive(Debug, Clone)]
pub struct SpriteComponent {
    pub layers: Vec<honknet_protocol::SpriteLayerSnapshot>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DynamicComponentSet {
    pub states: BTreeMap<String, Value>,
}

impl DynamicComponentSet {
    pub fn insert(&mut self, component: impl Into<String>, state: Value) {
        self.states.insert(component.into(), state);
    }

    pub fn remove(&mut self, component: &str) -> Option<Value> {
        self.states.remove(component)
    }
}
