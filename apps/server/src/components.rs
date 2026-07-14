use std::{collections::VecDeque, time::Instant};

use honknet_protocol::{PlayerIdentityId, Vec2};

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
