//! Setting-agnostic core primitives for Space Station 15.

use serde::{Deserialize, Serialize};
use honknet_protocol::{EntityNetId, Vec2};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub map_id: String,
    pub position: Vec2,
    pub rotation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIdentity {
    pub net_id: EntityNetId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrototypeRef {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMapChunk {
    pub map_id: String,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<u32>,
}
