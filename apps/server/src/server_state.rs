use std::sync::Arc;

use ss15_protocol::{EntityNetId, EntitySnapshot, NetPosition, ServerMessage};
use tokio::sync::RwLock;

pub type SharedServerState = Arc<RwLock<ServerState>>;

#[derive(Debug, Clone)]
pub struct ServerState {
    tick: u64,
    next_entity_net_id: EntityNetId,
    entities: Vec<EntitySnapshot>,
}

impl ServerState {
    pub fn new_debug() -> Self {
        Self {
            tick: 0,
            next_entity_net_id: 1,
            entities: Vec::new(),
        }
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    pub fn spawn_player_entity(&mut self) -> EntityNetId {
        let entity_net_id = self.allocate_entity_net_id();

        self.entities.push(EntitySnapshot {
            net_id: entity_net_id,
            prototype: "debug.player".to_string(),
            position: NetPosition {
                x: 0.0,
                y: 0.0,
                z: 0,
            },
        });

        entity_net_id
    }

    pub fn snapshot_message(&self) -> ServerMessage {
        ServerMessage::Snapshot {
            tick: self.tick,
            entities: self.entities.clone(),
        }
    }

    fn allocate_entity_net_id(&mut self) -> EntityNetId {
        let entity_net_id = self.next_entity_net_id;
        self.next_entity_net_id = self.next_entity_net_id.saturating_add(1);
        entity_net_id
    }
}

pub fn new_shared_debug_state() -> SharedServerState {
    Arc::new(RwLock::new(ServerState::new_debug()))
}
