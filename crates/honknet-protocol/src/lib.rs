//! Shared wire protocol for the Honknet server and browser client.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ClientId = Uuid;
pub type EntityNetId = u64;
pub type PlayerIdentityId = String;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NetPosition {
    pub x: f32,
    pub y: f32,
    pub z: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapSnapshot {
    pub id: String,
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "component", content = "data")]
pub enum ComponentSnapshot {
    Player { display_name: String, online: bool },
    Door { open: bool },
    Item { name: String },
    Inventory { items: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub net_id: EntityNetId,
    pub prototype: String,
    pub position: NetPosition,
    pub components: Vec<ComponentSnapshot>,
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
    Interact {
        target: EntityNetId,
    },
    Chat {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Welcome {
        client_id: ClientId,
        entity_net_id: EntityNetId,
        map: MapSnapshot,
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
    System {
        text: String,
    },
    Error {
        message: String,
    },
}
