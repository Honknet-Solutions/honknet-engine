use std::{collections::HashMap, sync::Arc};

use honknet_protocol::{
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
    player_inputs: HashMap<EntityNetId, PlayerInputState>,
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

#[derive(Debug, Clone)]
pub struct PlayerInputState {
    pub last_sequence: Option<u32>,
    pub last_client_tick: Option<u32>,
    pub movement: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputUpdateResult {
    Accepted,
    Stale,
    EntityMissing,
}

impl PlayerInputState {
    pub fn new() -> Self {
        Self {
            last_sequence: None,
            last_client_tick: None,
            movement: Vec2 { x: 0.0, y: 0.0 },
        }
    }
}

impl Default for PlayerInputState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerState {
    pub fn new_debug() -> Self {
        Self {
            tick: 0,
            next_entity_net_id: 1,
            entities: Vec::new(),
            players: Vec::new(),
            player_inputs: HashMap::new(),
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

            self.player_inputs
                .insert(player.entity_net_id, PlayerInputState::new());

            return player.entity_net_id;
        }

        let entity_net_id = self.spawn_player_entity();

        self.players.push(PlayerRecord {
            identity_id,
            client_id: Some(client_id),
            entity_net_id,
            connection_state: PlayerConnectionState::Online,
        });

        self.player_inputs
            .insert(entity_net_id, PlayerInputState::new());

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

        if let Some(input_state) = self.player_inputs.get_mut(&player.entity_net_id) {
            input_state.movement = Vec2 { x: 0.0, y: 0.0 };
        }

        Some(player.entity_net_id)
    }

    pub fn set_movement_input(
        &mut self,
        entity_net_id: EntityNetId,
        sequence: u32,
        client_tick: u32,
        movement: Vec2,
    ) -> InputUpdateResult {
        let entity_exists = self
            .entities
            .iter()
            .any(|entity| entity.net_id == entity_net_id);

        if !entity_exists {
            return InputUpdateResult::EntityMissing;
        }

        let input_state = self.player_inputs.entry(entity_net_id).or_default();

        if let Some(last_sequence) = input_state.last_sequence {
            if !is_sequence_newer(sequence, last_sequence) {
                return InputUpdateResult::Stale;
            }
        }

        input_state.last_sequence = Some(sequence);
        input_state.last_client_tick = Some(client_tick);
        input_state.movement = sanitize_movement(movement);

        InputUpdateResult::Accepted
    }

    pub fn snapshot_message_for(&self, entity_net_id: EntityNetId) -> ServerMessage {
        let input_state = self.player_inputs.get(&entity_net_id);

        let last_processed_input_seq = input_state.and_then(|state| state.last_sequence);

        let last_processed_client_tick = input_state.and_then(|state| state.last_client_tick);

        ServerMessage::Snapshot {
            tick: self.tick,
            last_processed_input_seq,
            last_processed_client_tick,
            entities: self.entities.clone(),
        }
    }

    fn apply_movement(&mut self, delta_seconds: f32) {
        for entity in &mut self.entities {
            let Some(input_state) = self.player_inputs.get(&entity.net_id) else {
                continue;
            };

            entity.position.x += input_state.movement.x * PLAYER_MOVE_SPEED * delta_seconds;

            entity.position.y += input_state.movement.y * PLAYER_MOVE_SPEED * delta_seconds;
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

fn is_sequence_newer(candidate: u32, current: u32) -> bool {
    let difference = candidate.wrapping_sub(current);

    difference != 0 && difference < (1_u32 << 31)
}

pub fn new_shared_debug_state() -> SharedServerState {
    Arc::new(RwLock::new(ServerState::new_debug()))
}

#[cfg(test)]
mod tests {
    use super::is_sequence_newer;

    #[test]
    fn newer_sequence_is_accepted() {
        assert!(is_sequence_newer(11, 10));
    }

    #[test]
    fn duplicate_sequence_is_rejected() {
        assert!(!is_sequence_newer(10, 10));
    }

    #[test]
    fn older_sequence_is_rejected() {
        assert!(!is_sequence_newer(9, 10));
    }

    #[test]
    fn wrapped_sequence_is_accepted() {
        assert!(is_sequence_newer(0, u32::MAX,));
    }
}
