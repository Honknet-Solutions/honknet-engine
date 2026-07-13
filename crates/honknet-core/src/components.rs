use honknet_protocol::{EntityNetId, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub map_id: String,
    pub position: Vec2,
    pub z: i32,
    pub rotation: f32,
}

impl Transform {
    pub fn new(map_id: impl Into<String>, position: Vec2, z: i32) -> Self {
        Self {
            map_id: map_id.into(),
            position,
            z,
            rotation: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetworkIdentity {
    pub net_id: EntityNetId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrototypeRef {
    pub id: String,
}

impl PrototypeRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}
