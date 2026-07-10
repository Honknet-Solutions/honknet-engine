use std::{collections::HashMap, sync::Arc};

use ss15_protocol::{
    ClientId, EntityNetId, EntitySnapshot, NetPosition, PlayerIdentityId, ServerMessage, Vec2,
};
use tokio::sync::RwLock;

pub type SharedServerState = Arc<RwLock<ServerState>>;

const PLAYER_MOVE_SPEED: f32 = 4.0;

#[derive(Debug, Clone)]
pub struct ServerState {
    tick: u64,
    next_entity_net_id: EntityNetId,
    entities: Vec<EntitySnapshot>,
    players: Vec<PlayerRecord>,
    movement_inputs: HashMap<EntityNetId, Vec2>,
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
            movement_inputs: HashMap::new(),
        }
    }

    pub fn advance_tick(&mut self, delta_seconds: f32) {
        self.tick = self.tick.saturating_add(1);
        self.apply_movement(delta_seconds);
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

            self.movement_inputs
                .insert(player.entity_net_id, Vec2 { x: 0.0, y: 0.0 });

            return player.entity_net_id;
        }

        let entity_net_id = self.spawn_player_entity();

        self.players.push(PlayerRecord {
            identity_id,
            client_id: Some(client_id),
            entity_net_id,
            connection_state: PlayerConnectionState::Online,
        });

        self.movement_inputs
            .insert(entity_net_id, Vec2 { x: 0.0, y: 0.0 });

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

        self.movement_inputs
            .insert(player.entity_net_id, Vec2 { x: 0.0, y: 0.0 });

        Some(player.entity_net_id)
    }

    pub fn set_movement_input(&mut self, entity_net_id: EntityNetId, movement: Vec2) -> bool {
        let entity_exists = self
            .entities
            .iter()
            .any(|entity| entity.net_id == entity_net_id);

        if !entity_exists {
            return false;
        }

        self.movement_inputs
            .insert(entity_net_id, sanitize_movement(movement));

        true
    }

    pub fn snapshot_message(&self) -> ServerMessage {
        ServerMessage::Snapshot {
            tick: self.tick,
            entities: self.entities.clone(),
        }
    }

    fn apply_movement(&mut self, delta_seconds: f32) {
        for entity in &mut self.entities {
            let Some(movement) = self.movement_inputs.get(&entity.net_id) else {
                continue;
            };

            entity.position.x += movement.x * PLAYER_MOVE_SPEED * delta_seconds;
            entity.position.y += movement.y * PLAYER_MOVE_SPEED * delta_seconds;
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
