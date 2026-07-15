//! Shared wire protocol for the Honknet server and browser client.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u32 = 4;

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
pub struct TileDefinitionSnapshot {
    pub id: String,
    pub solid: bool,
    #[serde(default = "default_tile_color")]
    pub color: [u8; 4],
    #[serde(default)]
    pub texture: Option<String>,
}

fn default_tile_color() -> [u8; 4] {
    [32, 42, 54, 255]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileChunkSnapshot {
    pub position: [i32; 2],
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSnapshot {
    pub id: String,
    pub position: [f32; 2],
    pub rotation: f32,
    pub chunks: Vec<TileChunkSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapSnapshot {
    pub id: String,
    pub tile_size: u16,
    pub tile_definitions: Vec<TileDefinitionSnapshot>,
    pub grids: Vec<GridSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteLayerSnapshot {
    pub key: String,
    pub source: SpriteSourceSnapshot,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_white")]
    pub color: [u8; 4],
    #[serde(default = "default_scale")]
    pub scale: [f32; 2],
    #[serde(default)]
    pub offset: [f32; 2],
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub z_index: i32,
    #[serde(default)]
    pub direction: u8,
}

fn default_true() -> bool {
    true
}

fn default_white() -> [u8; 4] {
    [255, 255, 255, 255]
}

fn default_scale() -> [f32; 2] {
    [1.0, 1.0]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpriteSourceSnapshot {
    Texture { path: String },
    Rsi { path: String, state: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItemSnapshot {
    pub entity_net_id: EntityNetId,
    pub prototype: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "component", content = "data")]
pub enum ComponentSnapshot {
    Player {
        display_name: String,
        online: bool,
    },
    Door {
        open: bool,
    },
    Item {
        name: String,
        size: String,
    },
    Inventory {
        capacity: u32,
        items: Vec<InventoryItemSnapshot>,
    },
    Sprite {
        layers: Vec<SpriteLayerSnapshot>,
    },
    Dynamic {
        name: String,
        state: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub net_id: EntityNetId,
    #[serde(default)]
    pub revision: u64,
    pub prototype: String,
    pub map_id: String,
    #[serde(default)]
    pub grid: Option<String>,
    pub position: NetPosition,
    #[serde(default)]
    pub rotation: f32,
    pub components: Vec<ComponentSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    Hello {
        protocol_version: u32,
        client_version: String,
        identity_id: PlayerIdentityId,
        #[serde(default)]
        auth_token: Option<String>,
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
    SnapshotAck {
        tick: u64,
    },
    RequestFullState,
    UiAction {
        session_id: String,
        action: String,
        payload: Value,
    },
    Ping {
        nonce: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Welcome {
        protocol_version: u32,
        client_id: ClientId,
        entity_net_id: EntityNetId,
        server_tick: u64,
        map: MapSnapshot,
    },
    Snapshot {
        tick: u64,
        last_processed_input_seq: Option<u32>,
        last_processed_client_tick: Option<u32>,
        entities: Vec<EntitySnapshot>,
    },
    StateDelta {
        tick: u64,
        baseline_tick: u64,
        last_processed_input_seq: Option<u32>,
        last_processed_client_tick: Option<u32>,
        spawns: Vec<EntitySnapshot>,
        updates: Vec<EntitySnapshot>,
        despawns: Vec<EntityNetId>,
    },
    Chat {
        from: String,
        text: String,
    },
    System {
        text: String,
    },
    UiOpen {
        session_id: String,
        key: String,
        target: EntityNetId,
        state: Value,
    },
    UiState {
        session_id: String,
        state: Value,
    },
    UiClose {
        session_id: String,
    },
    PlaySound {
        path: String,
        position: Option<NetPosition>,
    },
    Pong {
        nonce: u64,
        server_tick: u64,
    },
    Error {
        code: String,
        message: String,
        #[serde(default)]
        fatal: bool,
    },
}
