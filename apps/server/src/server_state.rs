use std::sync::Arc;

use ss15_protocol::{EntitySnapshot, NetPosition, ServerMessage};
use tokio::sync::RwLock;

pub type SharedServerState = Arc<RwLock<ServerState>>;

#[derive(Debug, Clone)]
pub struct ServerState {
    tick: u64,
    entities: Vec<EntitySnapshot>,
}

impl ServerState {
    pub fn new_debug() -> Self {
        Self {
            tick: 0,
            entities: vec![EntitySnapshot {
                net_id: 1,
                prototype: "debug.player".to_string(),
                position: NetPosition {
                    x: 0.0,
                    y: 0.0,
                    z: 0,
                },
            }],
        }
    }

    pub fn snapshot_message(&self) -> ServerMessage {
        ServerMessage::Snapshot {
            tick: self.tick,
            entities: self.entities.clone(),
        }
    }
}

pub fn new_shared_debug_state() -> SharedServerState {
    Arc::new(RwLock::new(ServerState::new_debug()))
}
