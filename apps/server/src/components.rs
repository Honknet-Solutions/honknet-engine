use std::time::Instant;

use honknet_protocol::{PlayerIdentityId, Vec2};

#[derive(Debug, Clone)]
pub struct PlayerComponent {
    pub identity_id: PlayerIdentityId,
    pub display_name: String,
    pub online: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerInputComponent {
    pub last_sequence: Option<u32>,
    pub last_client_tick: Option<u32>,
    pub movement: Vec2,
    pub last_received_at: Option<Instant>,
}

impl PlayerInputComponent {
    pub fn new() -> Self {
        Self {
            last_sequence: None,
            last_client_tick: None,
            movement: Vec2 { x: 0.0, y: 0.0 },
            last_received_at: None,
        }
    }

    pub fn stop(&mut self) {
        self.movement = Vec2 { x: 0.0, y: 0.0 };
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
}

#[derive(Debug, Clone)]
pub struct DoorComponent {
    pub open: bool,
}

#[derive(Debug, Clone)]
pub struct ItemComponent {
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct InventoryComponent {
    pub items: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SpriteComponent {
    pub layers: Vec<honknet_protocol::SpriteLayerSnapshot>,
}
