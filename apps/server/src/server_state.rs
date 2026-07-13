use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use honknet_core::{EntityId, NetworkIdentity, PrototypeRef, SystemManager, Transform, World};
use honknet_protocol::{
    ClientId, ComponentSnapshot, EntityNetId, EntitySnapshot, NetPosition, PlayerIdentityId,
    ServerMessage, Vec2,
};

use crate::{
    components::{
        ColliderComponent, DoorComponent, InventoryComponent, ItemComponent, PlayerComponent,
        PlayerInputComponent,
    },
    game_map::GameMap,
    prototypes::{PrototypeCatalog, PrototypeKind},
    systems::{InputTimeoutSystem, MovementSystem},
};

const PLAYER_MOVE_SPEED: f32 = 4.0;
const PLAYER_INPUT_TIMEOUT: Duration = Duration::from_millis(1500);
const INTERACTION_RANGE: f32 = 1.75;
const PVS_RADIUS: f32 = 24.0;
const PLAYER_PROTOTYPE: &str = "debug.player";
const DOOR_PROTOTYPE: &str = "debug.door";
const WRENCH_PROTOTYPE: &str = "debug.item.wrench";

pub struct ServerState {
    tick: u64,
    next_entity_net_id: EntityNetId,
    world: World,
    systems: SystemManager,
    players: HashMap<PlayerIdentityId, PlayerRecord>,
    network_entities: HashMap<EntityNetId, EntityId>,
    map: Arc<GameMap>,
    prototypes: PrototypeCatalog,
}

#[derive(Debug, Clone)]
pub struct PlayerRecord {
    pub client_id: Option<ClientId>,
    pub entity_id: EntityId,
    pub entity_net_id: EntityNetId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputUpdateResult {
    Accepted,
    Stale,
    EntityMissing,
}

impl ServerState {
    pub fn new_debug() -> Result<Self> {
        let map = Arc::new(GameMap::load_debug()?);
        let prototypes = PrototypeCatalog::load()?;
        let mut systems = SystemManager::new();

        assert!(systems.add(InputTimeoutSystem::new(PLAYER_INPUT_TIMEOUT)));
        assert!(systems.add(MovementSystem::new(PLAYER_MOVE_SPEED, map.clone())));

        let mut state = Self {
            tick: 0,
            next_entity_net_id: 1,
            world: World::new(),
            systems,
            players: HashMap::new(),
            network_entities: HashMap::new(),
            map,
            prototypes,
        };

        state.spawn_door();
        state.spawn_item(WRENCH_PROTOTYPE, Vec2 { x: 4.5, y: 4.5 });

        Ok(state)
    }

    pub fn advance_tick(&mut self, delta_seconds: f32) {
        self.tick = self.tick.saturating_add(1);
        self.systems.update(&mut self.world, delta_seconds);
    }

    pub fn map_snapshot(&self) -> honknet_protocol::MapSnapshot {
        self.map.snapshot()
    }

    pub fn connect_player(
        &mut self,
        client_id: ClientId,
        identity_id: PlayerIdentityId,
    ) -> EntityNetId {
        if let Some(record) = self.players.get(&identity_id).cloned() {
            if let Some(player) = self
                .world
                .get_component_mut::<PlayerComponent>(record.entity_id)
            {
                player.online = true;
            }

            if let Some(input) = self
                .world
                .get_component_mut::<PlayerInputComponent>(record.entity_id)
            {
                *input = PlayerInputComponent::new();
            }

            if let Some(player_record) = self.players.get_mut(&identity_id) {
                player_record.client_id = Some(client_id);
            }

            return record.entity_net_id;
        }

        let entity_net_id = self.allocate_net_id();
        let entity_id = self.world.spawn();
        let display_name = guest_display_name(&identity_id);

        self.add_base_components(
            entity_id,
            entity_net_id,
            PLAYER_PROTOTYPE,
            Vec2 { x: 2.5, y: 2.5 },
        );
        self.world
            .add_component(
                entity_id,
                PlayerComponent {
                    identity_id: identity_id.clone(),
                    display_name,
                    online: true,
                },
            )
            .expect("player entity must exist");
        self.world
            .add_component(entity_id, PlayerInputComponent::new())
            .expect("player entity must exist");
        self.world
            .add_component(entity_id, ColliderComponent { radius: 0.32 })
            .expect("player entity must exist");
        self.world
            .add_component(entity_id, InventoryComponent::default())
            .expect("player entity must exist");

        self.players.insert(
            identity_id,
            PlayerRecord {
                client_id: Some(client_id),
                entity_id,
                entity_net_id,
            },
        );

        entity_net_id
    }

    pub fn disconnect_player(&mut self, identity_id: &PlayerIdentityId) -> Option<EntityNetId> {
        let record = self.players.get_mut(identity_id)?;
        record.client_id = None;

        if let Some(player) = self
            .world
            .get_component_mut::<PlayerComponent>(record.entity_id)
        {
            player.online = false;
        }
        if let Some(input) = self
            .world
            .get_component_mut::<PlayerInputComponent>(record.entity_id)
        {
            input.stop();
        }

        Some(record.entity_net_id)
    }

    pub fn set_movement_input(
        &mut self,
        entity_net_id: EntityNetId,
        sequence: u32,
        client_tick: u32,
        movement: Vec2,
    ) -> InputUpdateResult {
        let Some(entity_id) = self.network_entities.get(&entity_net_id).copied() else {
            return InputUpdateResult::EntityMissing;
        };
        let Some(input) = self
            .world
            .get_component_mut::<PlayerInputComponent>(entity_id)
        else {
            return InputUpdateResult::EntityMissing;
        };

        if let Some(last_sequence) = input.last_sequence {
            if !is_sequence_newer(sequence, last_sequence) {
                return InputUpdateResult::Stale;
            }
        }

        input.last_sequence = Some(sequence);
        input.last_client_tick = Some(client_tick);
        input.movement = sanitize_movement(movement);
        input.last_received_at = Some(Instant::now());
        InputUpdateResult::Accepted
    }

    pub fn interact(
        &mut self,
        actor_net_id: EntityNetId,
        target_net_id: EntityNetId,
    ) -> Option<String> {
        if actor_net_id == target_net_id {
            return None;
        }

        let actor_id = self.network_entities.get(&actor_net_id).copied()?;
        let target_id = self.network_entities.get(&target_net_id).copied()?;

        let actor_position = self.world.get_component::<Transform>(actor_id)?.position;
        let target_position = self.world.get_component::<Transform>(target_id)?.position;
        let distance = ((actor_position.x - target_position.x).powi(2)
            + (actor_position.y - target_position.y).powi(2))
        .sqrt();

        if distance > INTERACTION_RANGE {
            return Some("Target is too far away.".to_string());
        }

        if let Some(door) = self.world.get_component_mut::<DoorComponent>(target_id) {
            door.open = !door.open;
            return Some(if door.open {
                "Door opened.".to_string()
            } else {
                "Door closed.".to_string()
            });
        }

        let item_name = self
            .world
            .get_component::<ItemComponent>(target_id)
            .map(|item| item.name.clone());

        if let Some(item_name) = item_name {
            let inventory = self
                .world
                .get_component_mut::<InventoryComponent>(actor_id)?;
            inventory.items.push(item_name.clone());
            self.world.despawn(target_id);
            self.network_entities.remove(&target_net_id);
            return Some(format!("Picked up {item_name}."));
        }

        None
    }

    pub fn player_name(&self, entity_net_id: EntityNetId) -> Option<String> {
        let entity_id = self.network_entities.get(&entity_net_id)?;
        self.world
            .get_component::<PlayerComponent>(*entity_id)
            .map(|player| player.display_name.clone())
    }

    pub fn snapshot_for(&self, requester_net_id: EntityNetId) -> ServerMessage {
        let requester_id = self.network_entities.get(&requester_net_id).copied();
        let requester_position = requester_id
            .and_then(|id| self.world.get_component::<Transform>(id))
            .map(|transform| (transform.position, transform.z));

        let input_state =
            requester_id.and_then(|id| self.world.get_component::<PlayerInputComponent>(id));

        let mut entities = self
            .world
            .iter()
            .filter_map(|(_, entity)| {
                let network = entity.get::<NetworkIdentity>()?;
                let prototype = entity.get::<PrototypeRef>()?;
                let transform = entity.get::<Transform>()?;

                if let Some((requester_position, requester_z)) = requester_position {
                    if requester_z != transform.z {
                        return None;
                    }

                    let distance_squared = (requester_position.x - transform.position.x).powi(2)
                        + (requester_position.y - transform.position.y).powi(2);
                    if distance_squared > PVS_RADIUS * PVS_RADIUS {
                        return None;
                    }
                }

                let mut components = Vec::new();

                if let Some(player) = entity.get::<PlayerComponent>() {
                    components.push(ComponentSnapshot::Player {
                        display_name: player.display_name.clone(),
                        online: player.online,
                    });
                }
                if let Some(door) = entity.get::<DoorComponent>() {
                    components.push(ComponentSnapshot::Door { open: door.open });
                }
                if let Some(item) = entity.get::<ItemComponent>() {
                    components.push(ComponentSnapshot::Item {
                        name: item.name.clone(),
                    });
                }
                if network.net_id == requester_net_id {
                    if let Some(inventory) = entity.get::<InventoryComponent>() {
                        components.push(ComponentSnapshot::Inventory {
                            items: inventory.items.clone(),
                        });
                    }
                }

                Some(EntitySnapshot {
                    net_id: network.net_id,
                    prototype: prototype.id.clone(),
                    position: NetPosition {
                        x: transform.position.x,
                        y: transform.position.y,
                        z: transform.z,
                    },
                    components,
                })
            })
            .collect::<Vec<_>>();

        entities.sort_by_key(|entity| entity.net_id);

        ServerMessage::Snapshot {
            tick: self.tick,
            last_processed_input_seq: input_state.and_then(|input| input.last_sequence),
            last_processed_client_tick: input_state.and_then(|input| input.last_client_tick),
            entities,
        }
    }

    fn spawn_door(&mut self) {
        let prototype = self.prototypes.require(DOOR_PROTOTYPE);
        assert!(matches!(prototype.kind, PrototypeKind::Door));
        let net_id = self.allocate_net_id();
        let entity_id = self.world.spawn();
        self.add_base_components(
            entity_id,
            net_id,
            DOOR_PROTOTYPE,
            Vec2 {
                x: self.map.door_spawn.0,
                y: self.map.door_spawn.1,
            },
        );
        self.world
            .add_component(entity_id, DoorComponent { open: false })
            .expect("door entity must exist");
    }

    fn spawn_item(&mut self, prototype_id: &str, position: Vec2) {
        let prototype = self.prototypes.require(prototype_id).clone();
        assert!(matches!(prototype.kind, PrototypeKind::Item));
        let net_id = self.allocate_net_id();
        let entity_id = self.world.spawn();
        self.add_base_components(entity_id, net_id, prototype_id, position);
        self.world
            .add_component(
                entity_id,
                ItemComponent {
                    name: prototype.display_name,
                },
            )
            .expect("item entity must exist");
    }

    fn add_base_components(
        &mut self,
        entity_id: EntityId,
        net_id: EntityNetId,
        prototype_id: &str,
        position: Vec2,
    ) {
        self.world
            .add_component(entity_id, NetworkIdentity { net_id })
            .expect("entity must exist");
        self.world
            .add_component(entity_id, PrototypeRef::new(prototype_id))
            .expect("entity must exist");
        self.world
            .add_component(entity_id, Transform::new(self.map.id.clone(), position, 0))
            .expect("entity must exist");
        self.network_entities.insert(net_id, entity_id);
    }

    fn allocate_net_id(&mut self) -> EntityNetId {
        let net_id = self.next_entity_net_id;
        self.next_entity_net_id = self
            .next_entity_net_id
            .checked_add(1)
            .expect("network entity id space exhausted");
        net_id
    }
}

fn guest_display_name(identity_id: &str) -> String {
    let suffix = identity_id
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("Guest-{suffix}")
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

#[cfg(test)]
mod tests {
    use honknet_protocol::{ServerMessage, Vec2};
    use uuid::Uuid;

    use super::{InputUpdateResult, ServerState};

    #[test]
    fn player_moves_through_ecs_system() {
        let mut state = ServerState::new_debug().unwrap();
        let player = state.connect_player(Uuid::new_v4(), "guest-test".to_string());
        assert_eq!(
            state.set_movement_input(player, 1, 1, Vec2 { x: 1.0, y: 0.0 }),
            InputUpdateResult::Accepted,
        );

        state.advance_tick(0.5);

        let ServerMessage::Snapshot { entities, .. } = state.snapshot_for(player) else {
            panic!("expected snapshot");
        };
        let snapshot = entities
            .iter()
            .find(|entity| entity.net_id == player)
            .unwrap();
        assert!(snapshot.position.x > 2.5);
    }
}
