use std::sync::Arc;

use ss15_protocol::{
    ClientId, EntityNetId, EntitySnapshot, NetPosition, PlayerIdentityId, ServerMessage, Vec2,
};
use tokio::sync::RwLock;

pub type SharedServerState = Arc<RwLock<ServerState>>;

const DEBUG_MOVE_STEP: f32 = 0.5;

#[derive(Debug, Clone)]
pub struct ServerState {
    tick: u64,
    next_entity_net_id: EntityNetId,
    entities: Vec<EntitySnapshot>,
    players: Vec<PlayerRecord>,
}

#[derive(Debug, Clone)]
pub struct PlayerRecord {
    pub identity_id: PlayerIdentityId,
    pub client_id: Option<ClientId>,
    pub entity_net_id: EntityNetId,
    pub connection_state: PlayerConnectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerConnectionState {
    Online,
    Disconnected,
}

impl ServerState {
    pub fn new_debug() -> Self {
        Self {
            tick: 0,
            next_entity_net_id: 1,
            entities: Vec::new(),
            players: Vec::new(),
        }
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    pub fn connect_player(
        &mut self,
        client_id: ClientId,
        identity_id: PlayerIdentityId,
    ) -> EntityNetId {
        if let Some(player) = self
            .players
            .iter_mut()
            .find(|player| player.identity_id == identity_id)
        {
            player.client_id = Some(client_id);
            player.connection_state = PlayerConnectionState::Online;
            return player.entity_net_id;
        }

        let entity_net_id = self.spawn_player_entity();

        self.players.push(PlayerRecord {
            identity_id,
            client_id: Some(client_id),
            entity_net_id,
            connection_state: PlayerConnectionState::Online,
        });

        entity_net_id
    }

    pub fn mark_player_disconnected(
        &mut self,
        identity_id: &PlayerIdentityId,
    ) -> Option<EntityNetId> {
        let player = self
            .players
            .iter_mut()
            .find(|player| &player.identity_id == identity_id)?;

        player.client_id = None;
        player.connection_state = PlayerConnectionState::Disconnected;

        Some(player.entity_net_id)
    }

    pub fn apply_movement_input(&mut self, entity_net_id: EntityNetId, movement: Vec2) -> bool {
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.net_id == entity_net_id)
        else {
            return false;
        };

        let movement = sanitize_movement(movement);

        entity.position.x += movement.x * DEBUG_MOVE_STEP;
        entity.position.y += movement.y * DEBUG_MOVE_STEP;

        true
    }

    pub fn snapshot_message(&self) -> ServerMessage {
        ServerMessage::Snapshot {
            tick: self.tick,
            entities: self.entities.clone(),
        }
    }

    fn spawn_player_entity(&mut self) -> EntityNetId {
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

    fn allocate_entity_net_id(&mut self) -> EntityNetId {
        let entity_net_id = self.next_entity_net_id;
        self.next_entity_net_id = self.next_entity_net_id.saturating_add(1);
        entity_net_id
    }
}

fn sanitize_movement(movement: Vec2) -> Vec2 {
    if !movement.x.is_finite() || !movement.y.is_finite() {
        return Vec2 { x: 0.0, y: 0.0 };
    }

    let length_squared = movement.x * movement.x + movement.y * movement.y;

    if length_squared <= 1.0 {
        return movement;
    }

    let length = length_squared.sqrt();

    Vec2 {
        x: movement.x / length,
        y: movement.y / length,
    }
}

pub fn new_shared_debug_state() -> SharedServerState {
    Arc::new(RwLock::new(ServerState::new_debug()))
}
