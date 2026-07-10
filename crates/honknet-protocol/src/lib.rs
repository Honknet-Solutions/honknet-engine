//! Shared network protocol definitions for the Honknet framework.
//!
//! This crate contains stable message shapes used by the Rust server.
//! TypeScript definitions should be generated from the same schema in the future.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ClientId = Uuid;
pub type EntityNetId = u64;
pub type PlayerIdentityId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetPosition {
    pub x: f32,
    pub y: f32,
    pub z: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    Hello {
        client_version: String,
        identity_id: PlayerIdentityId,
    },

    Input {
        seq: u32,
        client_tick: u32,
        movement: Vec2,
    },

    Chat {
        text: String,
    },

    Interact {
        target: EntityNetId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Welcome {
        client_id: ClientId,
        entity_net_id: EntityNetId,
    },

    Snapshot {
        tick: u64,
        last_processed_input_seq: Option<u32>,
        last_processed_client_tick: Option<u32>,
        entities: Vec<EntitySnapshot>,
    },

    Chat {
        from: String,
        text: String,
    },

    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub net_id: EntityNetId,
    pub prototype: String,
    pub position: NetPosition,
}
