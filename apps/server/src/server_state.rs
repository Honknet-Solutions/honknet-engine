use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use honknet_content::{ComponentDefinition, ComponentSchemaRegistry, ReplicationMode};
use honknet_core::{
    EntityId, NetworkIdentity, PrototypeRef, SpatialHash, SystemManager, Transform, World,
};
use honknet_protocol::{
    ClientId, ComponentSnapshot, EntityNetId, EntitySnapshot, InventoryItemSnapshot, MapSnapshot,
    NetPosition, PlayerIdentityId, ServerMessage, SpriteLayerSnapshot, SpriteSourceSnapshot, Vec2,
};
use honknet_script::{ScriptCommand, ScriptEntitySnapshot, ScriptEvent, ScriptWorldDelta};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use serde_yaml::{Mapping, Value as YamlValue};
use tracing::warn;

use crate::{
    components::{
        ColliderComponent, ContainedInComponent, DoorComponent, DynamicComponentSet,
        InventoryComponent, InventoryEntry, ItemComponent, PlayerComponent, PlayerInputCommand,
        PlayerInputComponent, SpriteComponent,
    },
    game_map::GameMap,
    outbound::OutboundMessage,
    prototypes::{field_bool, field_f32, field_string, field_u32, PrototypeCatalog},
    systems::{InputTimeoutSystem, MovementSystem},
};

const PLAYER_MOVE_SPEED: f32 = 4.0;
const PLAYER_INPUT_TIMEOUT: Duration = Duration::from_millis(1_500);
const INTERACTION_RANGE: f32 = 1.75;
const PLAYER_PROTOTYPE: &str = "DebugPlayer";
const MAX_SCRIPT_TEXT_LENGTH: usize = 2_048;
const MAX_SCRIPT_STATE_BYTES: usize = 262_144;
pub struct ServerState {
    tick: u64,
    next_entity_net_id: EntityNetId,
    next_entity_revision: u64,
    next_ui_session_id: u64,
    world: World,
    systems: SystemManager,
    players: HashMap<PlayerIdentityId, PlayerRecord>,
    network_entities: HashMap<EntityNetId, EntityId>,
    entity_revisions: HashMap<EntityNetId, u64>,
    movement_replication_sequences: HashMap<EntityNetId, u32>,
    movement_scan: Vec<EntityId>,
    map: Arc<GameMap>,
    prototypes: PrototypeCatalog,
    component_schemas: ComponentSchemaRegistry,
    script_events: Vec<ScriptEvent>,
    pvs_radius: f32,
    max_pvs_entities: usize,
    spatial: Arc<RwLock<SpatialHash>>,
    ui_sessions: HashMap<String, UiSession>,
    script_dirty_entities: HashSet<EntityNetId>,
    script_removed_entities: Vec<EntityNetId>,
    script_requires_full_sync: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerRecord {
    pub client_id: Option<ClientId>,
    pub entity_id: EntityId,
    pub entity_net_id: EntityNetId,
}

#[derive(Debug, Clone)]
struct UiSession {
    owner: EntityNetId,
    target: EntityNetId,
    key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputUpdateResult {
    Accepted,
    Stale,
    EntityMissing,
}

#[derive(Debug)]
pub struct SnapshotView {
    pub tick: u64,
    pub last_processed_input_seq: Option<u32>,
    pub last_processed_client_tick: Option<u32>,
    pub visible_revisions: Vec<(EntityNetId, u64)>,
    pub changed_entities: Vec<EntitySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedWorld {
    pub version: u32,
    #[serde(default)]
    pub players: Vec<LegacyPersistedPlayer>,
    #[serde(default)]
    pub entities: Vec<PersistedEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPersistedPlayer {
    pub identity_id: PlayerIdentityId,
    pub position: NetPosition,
    #[serde(default)]
    pub inventory: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedEntity {
    pub prototype: String,
    pub map_id: String,
    #[serde(default)]
    pub grid: Option<String>,
    pub position: NetPosition,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub player_identity: Option<PlayerIdentityId>,
    #[serde(default)]
    pub player_display_name: Option<String>,
    #[serde(default)]
    pub door_open: Option<bool>,
    #[serde(default)]
    pub dynamic: BTreeMap<String, JsonValue>,
    #[serde(default)]
    pub inventory: Vec<PersistedInventoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedInventoryItem {
    pub prototype: String,
    pub display_name: String,
}

impl ServerState {
    pub fn new_debug() -> Result<Self> {
        let map = Arc::new(GameMap::load_debug()?);
        let prototypes = PrototypeCatalog::load()?;
        let component_schemas = ComponentSchemaRegistry::load_directory(component_schema_root())
            .context("failed to load component schemas")?;
        let spatial = Arc::new(RwLock::new(SpatialHash::new(8.0)));
        let mut systems = SystemManager::new();
        assert!(systems.add(InputTimeoutSystem::new(PLAYER_INPUT_TIMEOUT)));
        assert!(systems.add(MovementSystem::new(
            PLAYER_MOVE_SPEED,
            map.clone(),
            spatial.clone(),
        )));

        let mut state = Self {
            tick: 0,
            next_entity_net_id: 1,
            next_entity_revision: 1,
            next_ui_session_id: 1,
            world: World::new(),
            systems,
            players: HashMap::new(),
            network_entities: HashMap::new(),
            entity_revisions: HashMap::new(),
            movement_replication_sequences: HashMap::new(),
            movement_scan: Vec::new(),
            map,
            prototypes,
            component_schemas,
            script_events: Vec::new(),
            pvs_radius: env_f32("HONKNET_PVS_RADIUS", 32.0),
            max_pvs_entities: env_usize("HONKNET_MAX_PVS_ENTITIES", 4_096, 64, 65_536),
            spatial,
            ui_sessions: HashMap::new(),
            script_dirty_entities: HashSet::new(),
            script_removed_entities: Vec::new(),
            script_requires_full_sync: true,
        };
        state.spawn_map_entities()?;
        state.spawn_synthetic_entities_from_environment()?;
        state.rebuild_spatial();
        Ok(state)
    }

    pub fn advance_tick(&mut self, delta_seconds: f32) {
        self.tick = self.tick.saturating_add(1);
        self.systems.update(&mut self.world, delta_seconds);
        // Movement is a native hot-path system. Only player entities can be
        // moved by native input at the moment, so mark those entities for the
        // script-side cache instead of serializing the entire world every tick.
        let mut moved = Vec::new();
        self.world
            .query_ids2_into::<NetworkIdentity, PlayerInputComponent>(&mut self.movement_scan);
        for &entity_id in &self.movement_scan {
            let Some(net_id) = self
                .world
                .get_component::<NetworkIdentity>(entity_id)
                .map(|network| network.net_id)
            else {
                continue;
            };
            let Some(sequence) = self
                .world
                .get_component::<PlayerInputComponent>(entity_id)
                .and_then(|input| input.last_processed_sequence)
            else {
                continue;
            };
            if self.movement_replication_sequences.get(&net_id) != Some(&sequence) {
                self.movement_replication_sequences.insert(net_id, sequence);
                moved.push(net_id);
            }
        }
        for net_id in moved {
            self.mark_entity_dirty(net_id);
        }
    }

    pub fn map_snapshot(&self) -> MapSnapshot {
        self.map.snapshot()
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    pub fn entity_count(&self) -> usize {
        self.world.entity_count()
    }

    pub fn online_player_count(&self) -> usize {
        self.players
            .values()
            .filter(|record| record.client_id.is_some())
            .count()
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
            self.script_events.push(ScriptEvent {
                name: "player.reconnected".to_owned(),
                entity: Some(record.entity_net_id),
                payload: json!({ "identity": identity_id }),
            });
            self.mark_entity_dirty(record.entity_net_id);
            self.upsert_spatial_entity(record.entity_id);
            return record.entity_net_id;
        }

        let grid_id = self.map.default_grid_id().map(ToOwned::to_owned);
        let position = grid_id
            .as_deref()
            .and_then(|grid| self.map.local_to_world(grid, [2.5, 2.5]))
            .unwrap_or([2.5, 2.5]);
        let (entity_id, entity_net_id) = self
            .spawn_prototype(
                PLAYER_PROTOTYPE,
                Vec2 {
                    x: position[0],
                    y: position[1],
                },
                0,
                grid_id,
                0.0,
            )
            .expect("player prototype must be valid");
        let display_name = guest_display_name(&identity_id);

        if let Some(player) = self.world.get_component_mut::<PlayerComponent>(entity_id) {
            player.identity_id = identity_id.clone();
            player.display_name = display_name;
            player.online = true;
        } else {
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
        }
        if self
            .world
            .get_component::<PlayerInputComponent>(entity_id)
            .is_none()
        {
            self.world
                .add_component(entity_id, PlayerInputComponent::new())
                .expect("player entity must exist");
        }

        self.players.insert(
            identity_id.clone(),
            PlayerRecord {
                client_id: Some(client_id),
                entity_id,
                entity_net_id,
            },
        );
        self.script_events.push(ScriptEvent {
            name: "player.connected".to_owned(),
            entity: Some(entity_net_id),
            payload: json!({ "identity": identity_id }),
        });
        self.mark_entity_dirty(entity_net_id);
        self.upsert_spatial_entity(entity_id);
        entity_net_id
    }

    pub fn is_session_owner(&self, client_id: ClientId, entity_net_id: EntityNetId) -> bool {
        self.players.values().any(|record| {
            record.entity_net_id == entity_net_id && record.client_id == Some(client_id)
        })
    }

    pub fn disconnect_player(
        &mut self,
        identity_id: &PlayerIdentityId,
        client_id: ClientId,
    ) -> Option<EntityNetId> {
        let record = self.players.get_mut(identity_id)?;
        if record.client_id != Some(client_id) {
            return None;
        }
        record.client_id = None;
        let entity_id = record.entity_id;
        let entity_net_id = record.entity_net_id;
        if let Some(player) = self.world.get_component_mut::<PlayerComponent>(entity_id) {
            player.online = false;
        }
        if let Some(input) = self
            .world
            .get_component_mut::<PlayerInputComponent>(entity_id)
        {
            input.stop();
        }
        self.ui_sessions
            .retain(|_, session| session.owner != entity_net_id);
        self.script_events.push(ScriptEvent {
            name: "player.disconnected".to_owned(),
            entity: Some(entity_net_id),
            payload: json!({ "identity": identity_id }),
        });
        self.mark_entity_dirty(entity_net_id);
        Some(entity_net_id)
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

        if let Some(last_sequence) = input.last_received_sequence {
            if !is_sequence_newer(sequence, last_sequence) {
                return InputUpdateResult::Stale;
            }
        }
        input.last_received_sequence = Some(sequence);
        input.last_received_at = Some(Instant::now());
        input.enqueue(PlayerInputCommand {
            sequence,
            client_tick,
            movement: sanitize_movement(movement),
        });
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
        let actor_transform = self.world.get_component::<Transform>(actor_id)?.clone();
        let target_transform = self.world.get_component::<Transform>(target_id)?.clone();
        if actor_transform.map_id != target_transform.map_id
            || actor_transform.z != target_transform.z
        {
            return Some("Target is not on the same map level.".to_owned());
        }
        let distance = ((actor_transform.position.x - target_transform.position.x).powi(2)
            + (actor_transform.position.y - target_transform.position.y).powi(2))
        .sqrt();
        if distance > INTERACTION_RANGE {
            return Some("Target is too far away.".to_owned());
        }

        self.script_events.push(ScriptEvent {
            name: "entity.interact".to_owned(),
            entity: Some(target_net_id),
            payload: json!({ "actor": actor_net_id, "target": target_net_id }),
        });

        if self
            .world
            .get_component::<DoorComponent>(target_id)
            .is_some()
        {
            let open = !self
                .world
                .get_component::<DoorComponent>(target_id)
                .is_some_and(|door| door.open);
            self.set_door_open(target_id, open);
            self.mark_entity_dirty(target_net_id);
            return Some(if open {
                "Door opened.".to_owned()
            } else {
                "Door closed.".to_owned()
            });
        }

        let item = self
            .world
            .get_component::<ItemComponent>(target_id)
            .cloned();
        if let Some(item) = item {
            let prototype = self
                .world
                .get_component::<PrototypeRef>(target_id)?
                .id
                .clone();
            let inventory = self
                .world
                .get_component_mut::<InventoryComponent>(actor_id)?;
            if !inventory.can_insert() {
                return Some("Inventory is full.".to_owned());
            }
            inventory.items.push(InventoryEntry {
                entity_net_id: target_net_id,
                prototype,
                display_name: item.name.clone(),
            });
            let _ = self.world.remove_component::<Transform>(target_id);
            let _ = self.world.remove_component::<ColliderComponent>(target_id);
            let _ = self.world.add_component(
                target_id,
                ContainedInComponent {
                    owner_net_id: actor_net_id,
                },
            );
            self.mark_entity_dirty(actor_net_id);
            self.mark_entity_dirty(target_net_id);
            self.spatial
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(target_id);
            return Some(format!("Picked up {}.", item.name));
        }

        None
    }

    pub fn handle_ui_action(
        &mut self,
        player: EntityNetId,
        session_id: &str,
        action: String,
        payload: JsonValue,
    ) -> Result<(), &'static str> {
        if json_size(&payload) > MAX_SCRIPT_STATE_BYTES {
            return Err("UI action payload exceeds the configured size limit");
        }
        let Some(session) = self.ui_sessions.get(session_id) else {
            return Err("unknown UI session");
        };
        if session.owner != player {
            return Err("UI session is owned by another player");
        }
        self.script_events.push(ScriptEvent {
            name: "ui.action".to_owned(),
            entity: Some(session.target),
            payload: json!({
                "player": player,
                "sessionId": session_id,
                "key": session.key.clone(),
                "action": action,
                "payload": payload,
            }),
        });
        Ok(())
    }

    pub fn player_name(&self, entity_net_id: EntityNetId) -> Option<String> {
        let entity_id = self.network_entities.get(&entity_net_id)?;
        self.world
            .get_component::<PlayerComponent>(*entity_id)
            .map(|player| player.display_name.clone())
    }

    pub fn snapshot_for(&self, requester_net_id: EntityNetId) -> ServerMessage {
        let view = self.snapshot_view_for(requester_net_id, &HashMap::new(), true);
        ServerMessage::Snapshot {
            tick: view.tick,
            last_processed_input_seq: view.last_processed_input_seq,
            last_processed_client_tick: view.last_processed_client_tick,
            entities: view.changed_entities,
        }
    }

    pub fn snapshot_view_for(
        &self,
        requester_net_id: EntityNetId,
        known_revisions: &HashMap<EntityNetId, u64>,
        force_full: bool,
    ) -> SnapshotView {
        let requester_id = self.network_entities.get(&requester_net_id).copied();
        let requester_transform = requester_id
            .and_then(|id| self.world.get_component::<Transform>(id))
            .cloned();
        let input_state =
            requester_id.and_then(|id| self.world.get_component::<PlayerInputComponent>(id));

        let candidate_ids = requester_transform.as_ref().map_or_else(
            || self.world.entity_ids().collect::<Vec<_>>(),
            |transform| {
                self.spatial
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .query_circle(
                        self.map.map_hash,
                        transform.z,
                        transform.position.x,
                        transform.position.y,
                        self.pvs_radius,
                    )
            },
        );

        let mut ranked = candidate_ids
            .into_iter()
            .filter_map(|entity_id| {
                let candidate = self.world.get_component::<Transform>(entity_id)?;
                let distance_squared = requester_transform.as_ref().map_or(0.0, |requester| {
                    if candidate.map_id != requester.map_id || candidate.z != requester.z {
                        return f32::INFINITY;
                    }
                    let dx = candidate.position.x - requester.position.x;
                    let dy = candidate.position.y - requester.position.y;
                    dx * dx + dy * dy
                });
                if distance_squared > self.pvs_radius * self.pvs_radius {
                    return None;
                }
                Some((distance_squared, entity_id))
            })
            .collect::<Vec<_>>();
        if ranked.len() > self.max_pvs_entities {
            ranked.select_nth_unstable_by(self.max_pvs_entities, |left, right| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.value().cmp(&right.1.value()))
            });
            ranked.truncate(self.max_pvs_entities);
        }
        if let Some(requester_id) = requester_id {
            let requester_is_visible = ranked
                .iter()
                .any(|(_, entity_id)| *entity_id == requester_id);
            if !requester_is_visible && self.max_pvs_entities > 0 {
                if ranked.len() == self.max_pvs_entities {
                    let _ = ranked.pop();
                }
                ranked.push((0.0, requester_id));
            }
        }
        ranked.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.value().cmp(&right.1.value()))
        });

        let mut visible_revisions = Vec::with_capacity(ranked.len());
        let mut changed_entities = Vec::new();
        for (_, entity_id) in ranked {
            let Some(network) = self.world.get_component::<NetworkIdentity>(entity_id) else {
                continue;
            };
            let revision = self
                .entity_revisions
                .get(&network.net_id)
                .copied()
                .unwrap_or(0);
            visible_revisions.push((network.net_id, revision));
            if force_full || known_revisions.get(&network.net_id) != Some(&revision) {
                if let Some(snapshot) = self.entity_snapshot(entity_id, requester_net_id) {
                    changed_entities.push(snapshot);
                }
            }
        }
        visible_revisions.sort_unstable_by_key(|(net_id, _)| *net_id);
        changed_entities.sort_by_key(|entity| entity.net_id);

        SnapshotView {
            tick: self.tick,
            last_processed_input_seq: input_state.and_then(|input| input.last_processed_sequence),
            last_processed_client_tick: input_state
                .and_then(|input| input.last_processed_client_tick),
            visible_revisions,
            changed_entities,
        }
    }

    pub fn persistence_snapshot(&self) -> PersistedWorld {
        let mut entities = Vec::new();
        for entity_id in self.world.query_ids2::<Transform, PrototypeRef>() {
            if self
                .world
                .get_component::<ContainedInComponent>(entity_id)
                .is_some()
            {
                continue;
            }
            let Some(prototype) = self.world.get_component::<PrototypeRef>(entity_id) else {
                continue;
            };
            let Some(transform) = self.world.get_component::<Transform>(entity_id) else {
                continue;
            };
            let player = self.world.get_component::<PlayerComponent>(entity_id);
            let dynamic = self
                .world
                .get_component::<DynamicComponentSet>(entity_id)
                .map(|set| set.states.clone())
                .unwrap_or_default();
            let inventory = self
                .world
                .get_component::<InventoryComponent>(entity_id)
                .map(|inventory| {
                    inventory
                        .items
                        .iter()
                        .map(|entry| PersistedInventoryItem {
                            prototype: entry.prototype.clone(),
                            display_name: entry.display_name.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            entities.push(PersistedEntity {
                prototype: prototype.id.clone(),
                map_id: transform.map_id.clone(),
                grid: transform.grid_id.clone(),
                position: NetPosition {
                    x: transform.position.x,
                    y: transform.position.y,
                    z: transform.z,
                },
                rotation: transform.rotation,
                player_identity: player.map(|player| player.identity_id.clone()),
                player_display_name: player.map(|player| player.display_name.clone()),
                door_open: self
                    .world
                    .get_component::<DoorComponent>(entity_id)
                    .map(|door| door.open),
                dynamic,
                inventory,
            });
        }
        entities.sort_by(|left, right| {
            left.prototype
                .cmp(&right.prototype)
                .then_with(|| left.position.z.cmp(&right.position.z))
                .then_with(|| left.position.x.total_cmp(&right.position.x))
                .then_with(|| left.position.y.total_cmp(&right.position.y))
        });
        PersistedWorld {
            version: 2,
            players: Vec::new(),
            entities,
        }
    }

    pub fn restore_persistence(&mut self, persisted: PersistedWorld) {
        match persisted.version {
            1 => self.restore_legacy_players(persisted.players),
            2 => self.restore_entities(persisted.entities),
            _ => {}
        }
        self.script_requires_full_sync = true;
        self.script_dirty_entities.clear();
        self.script_removed_entities.clear();
        self.rebuild_spatial();
    }

    pub fn take_script_events(&mut self) -> Vec<ScriptEvent> {
        std::mem::take(&mut self.script_events)
    }

    pub fn take_script_world_delta(&mut self) -> ScriptWorldDelta {
        let full = std::mem::take(&mut self.script_requires_full_sync);
        let ids = if full {
            self.world
                .query_ids2::<NetworkIdentity, PrototypeRef>()
                .into_iter()
                .filter_map(|entity_id| {
                    self.world
                        .get_component::<NetworkIdentity>(entity_id)
                        .map(|network| network.net_id)
                })
                .collect::<Vec<_>>()
        } else {
            self.script_dirty_entities.drain().collect::<Vec<_>>()
        };

        let mut upserts = ids
            .into_iter()
            .filter_map(|net_id| {
                let entity_id = self.network_entities.get(&net_id).copied()?;
                self.script_entity_snapshot(entity_id)
            })
            .collect::<Vec<_>>();
        upserts.sort_by_key(|entity| entity.entity);

        let mut removals = std::mem::take(&mut self.script_removed_entities);
        removals.sort_unstable();
        removals.dedup();
        self.script_dirty_entities.clear();

        ScriptWorldDelta {
            full,
            upserts,
            removals,
        }
    }

    pub fn apply_script_commands(&mut self, commands: Vec<ScriptCommand>) -> Vec<OutboundMessage> {
        let mut outgoing = Vec::new();
        for command in commands {
            match command {
                ScriptCommand::Log { .. } => {}
                ScriptCommand::EmitSystemMessage { text } => {
                    let text = truncate_text(text, MAX_SCRIPT_TEXT_LENGTH);
                    outgoing.push(OutboundMessage::broadcast(ServerMessage::System { text }));
                }
                ScriptCommand::EmitPlayerMessage { player, text } => {
                    if self.network_entities.contains_key(&player) {
                        outgoing.push(OutboundMessage::player(
                            player,
                            ServerMessage::System {
                                text: truncate_text(text, MAX_SCRIPT_TEXT_LENGTH),
                            },
                        ));
                    }
                }
                ScriptCommand::Spawn { prototype, x, y, z } => {
                    if !x.is_finite() || !y.is_finite() {
                        warn!(%prototype, x, y, z, "Script requested a spawn with invalid coordinates");
                    } else if let Err(error) = self.spawn_prototype(
                        &prototype,
                        Vec2 { x, y },
                        z,
                        self.map.default_grid_id().map(ToOwned::to_owned),
                        0.0,
                    ) {
                        warn!(%prototype, %error, "Script spawn command was rejected");
                    }
                }
                ScriptCommand::Delete { entity } => {
                    if let Some(entity_id) = self.network_entities.get(&entity).copied() {
                        if self
                            .world
                            .get_component::<PlayerComponent>(entity_id)
                            .is_none()
                        {
                            self.delete_entity(entity_id, entity);
                        }
                    }
                }
                ScriptCommand::EmitEvent {
                    name,
                    entity,
                    payload,
                } => self.script_events.push(ScriptEvent {
                    name,
                    entity,
                    payload,
                }),
                ScriptCommand::SetComponent {
                    entity,
                    component,
                    state,
                } => {
                    if json_size(&state) > MAX_SCRIPT_STATE_BYTES {
                        warn!(entity, %component, "Script component state exceeded the size limit");
                    } else if let Err(message) = self.set_component_state(entity, &component, state)
                    {
                        warn!(entity, %component, %message, "Script component command was rejected");
                    }
                }
                ScriptCommand::RemoveComponent { entity, component } => {
                    if let Err(message) = self.remove_component_state(entity, &component) {
                        warn!(entity, %component, %message, "Script component removal was rejected");
                    }
                }
                ScriptCommand::OpenUi {
                    player,
                    target,
                    key,
                    state,
                } => {
                    let valid_key = valid_ui_key(&key);
                    let valid_state = json_size(&state) <= MAX_SCRIPT_STATE_BYTES;
                    if self.network_entities.contains_key(&player)
                        && self.network_entities.contains_key(&target)
                        && valid_key
                        && valid_state
                    {
                        let session_id =
                            format!("ui-{}-{}-{}", self.tick, player, self.next_ui_session_id);
                        self.next_ui_session_id = self.next_ui_session_id.saturating_add(1);
                        self.ui_sessions.insert(
                            session_id.clone(),
                            UiSession {
                                owner: player,
                                target,
                                key: key.clone(),
                            },
                        );
                        self.script_events.push(ScriptEvent {
                            name: "ui.opened".to_owned(),
                            entity: Some(target),
                            payload: json!({
                                "player": player,
                                "target": target,
                                "key": key.clone(),
                                "sessionId": session_id.clone(),
                            }),
                        });
                        outgoing.push(OutboundMessage::player(
                            player,
                            ServerMessage::UiOpen {
                                session_id,
                                key,
                                target,
                                state,
                            },
                        ));
                    } else {
                        warn!(player, target, %key, valid_key, valid_state, "Script UI open command was rejected");
                    }
                }
                ScriptCommand::UpdateUi {
                    player,
                    session_id,
                    state,
                } => {
                    if json_size(&state) <= MAX_SCRIPT_STATE_BYTES
                        && self
                            .ui_sessions
                            .get(&session_id)
                            .is_some_and(|session| session.owner == player)
                    {
                        outgoing.push(OutboundMessage::player(
                            player,
                            ServerMessage::UiState { session_id, state },
                        ));
                    }
                }
                ScriptCommand::CloseUi { player, session_id } => {
                    if self
                        .ui_sessions
                        .get(&session_id)
                        .is_some_and(|session| session.owner == player)
                    {
                        self.ui_sessions.remove(&session_id);
                        outgoing.push(OutboundMessage::player(
                            player,
                            ServerMessage::UiClose { session_id },
                        ));
                    }
                }
                ScriptCommand::PlaySound { path, x, y, z } => {
                    if valid_resource_path(&path) && x.is_finite() && y.is_finite() {
                        outgoing.push(OutboundMessage::broadcast(ServerMessage::PlaySound {
                            path,
                            position: Some(NetPosition { x, y, z }),
                        }));
                    } else {
                        warn!(%path, x, y, z, "Script sound command was rejected");
                    }
                }
            }
        }
        outgoing
    }

    fn spawn_map_entities(&mut self) -> Result<()> {
        let placements = self.map.entities.clone();
        for placement in placements {
            let position = placement
                .grid
                .as_deref()
                .and_then(|grid| self.map.local_to_world(grid, placement.position))
                .unwrap_or(placement.position);
            let (entity_id, _) = self.spawn_prototype(
                &placement.prototype,
                Vec2 {
                    x: position[0],
                    y: position[1],
                },
                0,
                placement.grid,
                placement.rotation,
            )?;
            for override_component in placement.components {
                let state = yaml_fields_to_json(&override_component)?;
                let net_id = self
                    .world
                    .get_component::<NetworkIdentity>(entity_id)
                    .map(|network| network.net_id)
                    .expect("spawned map entity has network identity");
                self.set_component_state(net_id, &override_component.component_type, state)
                    .map_err(anyhow::Error::msg)?;
            }
        }
        Ok(())
    }

    fn spawn_synthetic_entities_from_environment(&mut self) -> Result<()> {
        let count = std::env::var("HONKNET_SYNTHETIC_ENTITY_COUNT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0)
            .min(1_000_000);
        if count == 0 {
            return Ok(());
        }

        let prototype = std::env::var("HONKNET_SYNTHETIC_ENTITY_PROTOTYPE")
            .unwrap_or_else(|_| "DebugWrench".to_owned());
        if self.prototypes.get(&prototype).is_none() {
            anyhow::bail!(
                "HONKNET_SYNTHETIC_ENTITY_PROTOTYPE references unknown prototype {prototype}"
            );
        }
        let grid = self.map.default_grid_id().map(ToOwned::to_owned);
        let columns = (count as f64).sqrt().ceil().max(1.0) as usize;
        for index in 0..count {
            let local = [
                1.5 + (index % columns) as f32 * 1.25,
                1.5 + (index / columns) as f32 * 1.25,
            ];
            let world = grid
                .as_deref()
                .and_then(|grid_id| self.map.local_to_world(grid_id, local))
                .unwrap_or(local);
            self.spawn_prototype(
                &prototype,
                Vec2 {
                    x: world[0],
                    y: world[1],
                },
                0,
                grid.clone(),
                0.0,
            )?;
        }
        tracing::info!(count, %prototype, "Spawned synthetic benchmark entities");
        Ok(())
    }

    fn spawn_prototype(
        &mut self,
        prototype_id: &str,
        position: Vec2,
        z: i32,
        grid_id: Option<String>,
        rotation: f32,
    ) -> Result<(EntityId, EntityNetId)> {
        let prototype = self
            .prototypes
            .get(prototype_id)
            .cloned()
            .with_context(|| format!("unknown prototype {prototype_id}"))?;
        let entity_net_id = self.allocate_net_id();
        let entity_id = self.world.spawn();
        self.world.add_component(
            entity_id,
            NetworkIdentity {
                net_id: entity_net_id,
            },
        )?;
        self.world
            .add_component(entity_id, PrototypeRef::new(prototype_id))?;
        let mut transform = Transform::new(self.map.id.clone(), position, z);
        transform.grid_id = grid_id;
        transform.rotation = rotation;
        self.world.add_component(entity_id, transform)?;
        self.network_entities.insert(entity_net_id, entity_id);

        let mut dynamic = DynamicComponentSet::default();
        for component in &prototype.definition.components {
            match component.component_type.as_str() {
                "Transform" | "NetworkIdentity" | "PhysicsBody" => {}
                "Player" => {
                    self.world.add_component(
                        entity_id,
                        PlayerComponent {
                            identity_id: String::new(),
                            display_name: prototype.display_name.clone(),
                            online: false,
                        },
                    )?;
                    self.world
                        .add_component(entity_id, PlayerInputComponent::new())?;
                }
                "Sprite" => {
                    self.world
                        .add_component(entity_id, parse_sprite(component)?)?;
                }
                "Collider" => {
                    self.world
                        .add_component(entity_id, parse_collider(component))?;
                }
                "Inventory" => {
                    self.world.add_component(
                        entity_id,
                        InventoryComponent::new(field_u32(component, "capacity", 24)),
                    )?;
                }
                "Door" => {
                    self.world.add_component(
                        entity_id,
                        DoorComponent {
                            open: field_bool(component, "open", false),
                        },
                    )?;
                }
                "Item" => {
                    self.world.add_component(
                        entity_id,
                        ItemComponent {
                            name: prototype.display_name.clone(),
                            size: field_string(component, "size")
                                .unwrap_or_else(|| "Small".to_owned()),
                        },
                    )?;
                }
                other => {
                    let state = yaml_fields_to_json(component)?;
                    let state = self
                        .component_schemas
                        .normalize_json(other, &state)
                        .with_context(|| {
                            format!("prototype {prototype_id} has invalid {other} component")
                        })?;
                    dynamic.insert(other, state);
                }
            }
        }

        if self
            .world
            .get_component::<DoorComponent>(entity_id)
            .is_some()
            && self
                .world
                .get_component::<ColliderComponent>(entity_id)
                .is_none()
        {
            self.world
                .add_component(entity_id, ColliderComponent::solid_circle(0.45))?;
        }
        if self
            .world
            .get_component::<PlayerComponent>(entity_id)
            .is_some()
            && self
                .world
                .get_component::<ColliderComponent>(entity_id)
                .is_none()
        {
            self.world
                .add_component(entity_id, ColliderComponent::solid_circle(0.32))?;
        }
        if self
            .world
            .get_component::<InventoryComponent>(entity_id)
            .is_none()
            && self
                .world
                .get_component::<PlayerComponent>(entity_id)
                .is_some()
        {
            self.world
                .add_component(entity_id, InventoryComponent::default())?;
        }
        if !dynamic.states.is_empty() {
            self.world.add_component(entity_id, dynamic)?;
        }
        if self
            .world
            .get_component::<DoorComponent>(entity_id)
            .is_some()
        {
            let open = self
                .world
                .get_component::<DoorComponent>(entity_id)
                .is_some_and(|door| door.open);
            self.set_door_open(entity_id, open);
        }
        self.mark_entity_dirty(entity_net_id);
        self.upsert_spatial_entity(entity_id);
        Ok((entity_id, entity_net_id))
    }

    fn entity_snapshot(
        &self,
        entity_id: EntityId,
        requester_net_id: EntityNetId,
    ) -> Option<EntitySnapshot> {
        let network = self.world.get_component::<NetworkIdentity>(entity_id)?;
        let prototype = self.world.get_component::<PrototypeRef>(entity_id)?;
        let transform = self.world.get_component::<Transform>(entity_id)?;
        let mut components = Vec::new();

        if let Some(sprite) = self.world.get_component::<SpriteComponent>(entity_id) {
            components.push(ComponentSnapshot::Sprite {
                layers: sprite.layers.clone(),
            });
        }
        if let Some(player) = self.world.get_component::<PlayerComponent>(entity_id) {
            components.push(ComponentSnapshot::Player {
                display_name: player.display_name.clone(),
                online: player.online,
            });
        }
        if let Some(door) = self.world.get_component::<DoorComponent>(entity_id) {
            components.push(ComponentSnapshot::Door { open: door.open });
        }
        if let Some(item) = self.world.get_component::<ItemComponent>(entity_id) {
            components.push(ComponentSnapshot::Item {
                name: item.name.clone(),
                size: item.size.clone(),
            });
        }
        if network.net_id == requester_net_id {
            if let Some(inventory) = self.world.get_component::<InventoryComponent>(entity_id) {
                components.push(ComponentSnapshot::Inventory {
                    capacity: inventory.capacity,
                    items: inventory
                        .items
                        .iter()
                        .map(|entry| InventoryItemSnapshot {
                            entity_net_id: entry.entity_net_id,
                            prototype: entry.prototype.clone(),
                            display_name: entry.display_name.clone(),
                        })
                        .collect(),
                });
            }
        }
        if let Some(dynamic) = self.world.get_component::<DynamicComponentSet>(entity_id) {
            for (name, state) in &dynamic.states {
                let replication = self
                    .component_schemas
                    .get(name)
                    .map(|schema| &schema.replication.mode);
                let allowed = match replication {
                    Some(ReplicationMode::ServerToClient) => true,
                    Some(ReplicationMode::OwnerOnly) => network.net_id == requester_net_id,
                    Some(ReplicationMode::None) | None => false,
                };
                if allowed {
                    components.push(ComponentSnapshot::Dynamic {
                        name: name.clone(),
                        state: state.clone(),
                    });
                }
            }
        }

        Some(EntitySnapshot {
            net_id: network.net_id,
            revision: self
                .entity_revisions
                .get(&network.net_id)
                .copied()
                .unwrap_or(0),
            prototype: prototype.id.clone(),
            map_id: transform.map_id.clone(),
            grid: transform.grid_id.clone(),
            position: NetPosition {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.z,
            },
            rotation: transform.rotation,
            components,
        })
    }

    fn script_entity_snapshot(&self, entity_id: EntityId) -> Option<ScriptEntitySnapshot> {
        let network = self.world.get_component::<NetworkIdentity>(entity_id)?;
        let prototype = self.world.get_component::<PrototypeRef>(entity_id)?;
        let mut components = BTreeMap::new();

        if let Some(transform) = self.world.get_component::<Transform>(entity_id) {
            components.insert(
                "Transform".to_owned(),
                json!({
                    "mapId": transform.map_id.clone(),
                    "gridId": transform.grid_id.clone(),
                    "x": transform.position.x,
                    "y": transform.position.y,
                    "z": transform.z,
                    "rotation": transform.rotation,
                }),
            );
        }
        if let Some(player) = self.world.get_component::<PlayerComponent>(entity_id) {
            components.insert(
                "Player".to_owned(),
                json!({
                    "identityId": player.identity_id.clone(),
                    "displayName": player.display_name.clone(),
                    "online": player.online,
                }),
            );
        }
        if let Some(door) = self.world.get_component::<DoorComponent>(entity_id) {
            components.insert("Door".to_owned(), json!({ "open": door.open }));
        }
        if let Some(item) = self.world.get_component::<ItemComponent>(entity_id) {
            components.insert(
                "Item".to_owned(),
                json!({ "name": item.name.clone(), "size": item.size.clone() }),
            );
        }
        if let Some(inventory) = self.world.get_component::<InventoryComponent>(entity_id) {
            components.insert(
                "Inventory".to_owned(),
                json!({
                    "capacity": inventory.capacity,
                    "items": inventory.items.iter().map(|entry| json!({
                        "entity": entry.entity_net_id,
                        "prototype": entry.prototype.clone(),
                        "displayName": entry.display_name.clone(),
                    })).collect::<Vec<_>>(),
                }),
            );
        }
        if let Some(collider) = self.world.get_component::<ColliderComponent>(entity_id) {
            components.insert(
                "Collider".to_owned(),
                json!({
                    "radius": collider.radius,
                    "collisionLayer": collider.collision_layer,
                    "collisionMask": collider.collision_mask,
                    "sensor": collider.sensor,
                }),
            );
        }
        if let Some(container) = self.world.get_component::<ContainedInComponent>(entity_id) {
            components.insert(
                "ContainedIn".to_owned(),
                json!({ "owner": container.owner_net_id }),
            );
        }
        if let Some(dynamic) = self.world.get_component::<DynamicComponentSet>(entity_id) {
            for (name, state) in &dynamic.states {
                components.insert(name.clone(), state.clone());
            }
        }

        Some(ScriptEntitySnapshot {
            entity: network.net_id,
            prototype: prototype.id.clone(),
            components,
        })
    }

    fn set_component_state(
        &mut self,
        entity_net_id: EntityNetId,
        component: &str,
        state: JsonValue,
    ) -> Result<(), String> {
        let Some(entity_id) = self.network_entities.get(&entity_net_id).copied() else {
            return Err(format!("entity {entity_net_id} does not exist"));
        };

        match component {
            "NetworkIdentity" => {
                return Err("NetworkIdentity is engine-owned and immutable".to_owned());
            }
            "Transform" => {
                let current = self
                    .world
                    .get_component::<Transform>(entity_id)
                    .cloned()
                    .ok_or_else(|| "entity has no Transform component".to_owned())?;
                let x = optional_f32(&state, "x")?.unwrap_or(current.position.x);
                let y = optional_f32(&state, "y")?.unwrap_or(current.position.y);
                let z = optional_i32(&state, "z")?.unwrap_or(current.z);
                let rotation = optional_f32(&state, "rotation")?.unwrap_or(current.rotation);
                if !x.is_finite() || !y.is_finite() || !rotation.is_finite() {
                    return Err("Transform values must be finite".to_owned());
                }
                if !(-1_024..=1_024).contains(&z) {
                    return Err("Transform.z is outside the supported range".to_owned());
                }
                let grid_id = match state.get("gridId").or_else(|| state.get("grid")) {
                    None => current.grid_id,
                    Some(JsonValue::Null) => None,
                    Some(JsonValue::String(value)) => {
                        if !self.map.grid_ids().any(|candidate| candidate == value) {
                            return Err(format!("Transform references unknown grid {value}"));
                        }
                        Some(value.clone())
                    }
                    Some(_) => return Err("Transform.gridId must be a string or null".to_owned()),
                };
                if let Some(transform) = self.world.get_component_mut::<Transform>(entity_id) {
                    transform.position = Vec2 { x, y };
                    transform.z = z;
                    transform.rotation = rotation;
                    transform.grid_id = grid_id;
                }
            }
            "Player" => {
                let display_name = state
                    .get("displayName")
                    .or_else(|| state.get("display_name"))
                    .and_then(JsonValue::as_str)
                    .ok_or_else(|| "Player.displayName must be a string".to_owned())?;
                let display_name = sanitize_display_name(display_name)?;
                let player = self
                    .world
                    .get_component_mut::<PlayerComponent>(entity_id)
                    .ok_or_else(|| "entity has no Player component".to_owned())?;
                player.display_name = display_name;
            }
            "Sprite" => {
                let layers = state
                    .get("layers")
                    .cloned()
                    .ok_or_else(|| "Sprite.layers is required".to_owned())?;
                let layers = serde_json::from_value::<Vec<SpriteLayerSnapshot>>(layers)
                    .map_err(|error| format!("invalid Sprite.layers: {error}"))?;
                validate_sprite_layers(&layers)?;
                if let Some(sprite) = self.world.get_component_mut::<SpriteComponent>(entity_id) {
                    sprite.layers = layers;
                } else {
                    self.world
                        .add_component(entity_id, SpriteComponent { layers })
                        .map_err(|error| error.to_string())?;
                }
            }
            "Collider" => {
                let existing = self
                    .world
                    .get_component::<ColliderComponent>(entity_id)
                    .copied()
                    .unwrap_or_else(|| ColliderComponent::solid_circle(0.32));
                let radius = optional_f32(&state, "radius")?.unwrap_or(existing.radius);
                if !radius.is_finite() || !(0.01..=8.0).contains(&radius) {
                    return Err("Collider.radius must be between 0.01 and 8".to_owned());
                }
                let collision_layer = optional_u32(&state, "collisionLayer")?
                    .or(optional_u32(&state, "collision_layer")?)
                    .unwrap_or(existing.collision_layer);
                let collision_mask = optional_u32(&state, "collisionMask")?
                    .or(optional_u32(&state, "collision_mask")?)
                    .unwrap_or(existing.collision_mask);
                let sensor = state
                    .get("sensor")
                    .map(|value| {
                        value
                            .as_bool()
                            .ok_or_else(|| "Collider.sensor must be a boolean".to_owned())
                    })
                    .transpose()?
                    .unwrap_or(existing.sensor);
                let collider = ColliderComponent {
                    radius,
                    collision_layer,
                    collision_mask,
                    sensor,
                };
                if let Some(existing) = self.world.get_component_mut::<ColliderComponent>(entity_id)
                {
                    *existing = collider;
                } else {
                    self.world
                        .add_component(entity_id, collider)
                        .map_err(|error| error.to_string())?;
                }
            }
            "Door" => {
                let open = state
                    .get("open")
                    .and_then(JsonValue::as_bool)
                    .ok_or_else(|| "Door.open must be a boolean".to_owned())?;
                if self
                    .world
                    .get_component::<DoorComponent>(entity_id)
                    .is_none()
                {
                    self.world
                        .add_component(entity_id, DoorComponent { open })
                        .map_err(|error| error.to_string())?;
                }
                self.set_door_open(entity_id, open);
            }
            "Inventory" => {
                let capacity = state
                    .get("capacity")
                    .map(|value| {
                        value
                            .as_u64()
                            .and_then(|value| u32::try_from(value).ok())
                            .ok_or_else(|| "Inventory.capacity must be a u32".to_owned())
                    })
                    .transpose()?
                    .unwrap_or(24)
                    .min(4_096);
                if let Some(inventory) = self
                    .world
                    .get_component_mut::<InventoryComponent>(entity_id)
                {
                    inventory.capacity = capacity.max(inventory.items.len() as u32);
                } else {
                    self.world
                        .add_component(entity_id, InventoryComponent::new(capacity))
                        .map_err(|error| error.to_string())?;
                }
            }
            "Item" => {
                let current = self
                    .world
                    .get_component::<ItemComponent>(entity_id)
                    .cloned();
                let name = state
                    .get("name")
                    .and_then(JsonValue::as_str)
                    .map(sanitize_item_name)
                    .transpose()?
                    .or_else(|| current.as_ref().map(|item| item.name.clone()))
                    .ok_or_else(|| "Item.name is required".to_owned())?;
                let size = state
                    .get("size")
                    .and_then(JsonValue::as_str)
                    .map(sanitize_item_size)
                    .transpose()?
                    .or_else(|| current.as_ref().map(|item| item.size.clone()))
                    .unwrap_or_else(|| "Small".to_owned());
                let item = ItemComponent { name, size };
                if let Some(existing) = self.world.get_component_mut::<ItemComponent>(entity_id) {
                    *existing = item;
                } else {
                    self.world
                        .add_component(entity_id, item)
                        .map_err(|error| error.to_string())?;
                }
            }
            "PhysicsBody" => {
                return Err("PhysicsBody is reserved for the native physics runtime".to_owned());
            }
            other => {
                let normalized = self
                    .component_schemas
                    .normalize_json(other, &state)
                    .map_err(|error| error.to_string())?;
                if self
                    .world
                    .get_component::<DynamicComponentSet>(entity_id)
                    .is_none()
                {
                    self.world
                        .add_component(entity_id, DynamicComponentSet::default())
                        .map_err(|error| error.to_string())?;
                }
                self.world
                    .get_component_mut::<DynamicComponentSet>(entity_id)
                    .expect("dynamic component set was inserted")
                    .insert(other, normalized);
            }
        }
        self.mark_entity_dirty(entity_net_id);
        if matches!(component, "Transform" | "Collider" | "Door") {
            self.upsert_spatial_entity(entity_id);
        }
        Ok(())
    }

    fn remove_component_state(
        &mut self,
        entity_net_id: EntityNetId,
        component: &str,
    ) -> Result<(), String> {
        let Some(entity_id) = self.network_entities.get(&entity_net_id).copied() else {
            return Err(format!("entity {entity_net_id} does not exist"));
        };
        match component {
            "Transform" | "NetworkIdentity" | "Player" => {
                return Err(format!("{component} is required and cannot be removed"));
            }
            "Door" => {
                let _ = self.world.remove_component::<DoorComponent>(entity_id);
            }
            "Inventory" => {
                let has_items = self
                    .world
                    .get_component::<InventoryComponent>(entity_id)
                    .is_some_and(|inventory| !inventory.items.is_empty());
                if has_items {
                    return Err("Inventory cannot be removed while it contains items".to_owned());
                }
                let _ = self.world.remove_component::<InventoryComponent>(entity_id);
            }
            "Sprite" => {
                let _ = self.world.remove_component::<SpriteComponent>(entity_id);
            }
            "Collider" => {
                let _ = self.world.remove_component::<ColliderComponent>(entity_id);
            }
            "Item" => {
                let _ = self.world.remove_component::<ItemComponent>(entity_id);
            }
            "PhysicsBody" => {
                return Err("PhysicsBody is reserved for the native physics runtime".to_owned());
            }
            other => {
                let known = self.component_schemas.get(other).is_some();
                if !known {
                    return Err(format!("unknown component schema {other}"));
                }
                if let Some(dynamic) = self
                    .world
                    .get_component_mut::<DynamicComponentSet>(entity_id)
                {
                    dynamic.remove(other);
                }
            }
        }
        self.mark_entity_dirty(entity_net_id);
        if matches!(component, "Collider" | "Door") {
            self.upsert_spatial_entity(entity_id);
        }
        Ok(())
    }

    fn mark_entity_dirty(&mut self, entity_net_id: EntityNetId) {
        let revision = self.next_entity_revision;
        self.next_entity_revision = self.next_entity_revision.wrapping_add(1).max(1);
        self.entity_revisions.insert(entity_net_id, revision);
        self.script_dirty_entities.insert(entity_net_id);
    }

    fn set_door_open(&mut self, entity_id: EntityId, open: bool) {
        if let Some(door) = self.world.get_component_mut::<DoorComponent>(entity_id) {
            door.open = open;
        }
        if let Some(sprite) = self.world.get_component_mut::<SpriteComponent>(entity_id) {
            for layer in &mut sprite.layers {
                if let SpriteSourceSnapshot::Rsi { state, .. } = &mut layer.source {
                    if state == "open" || state == "closed" {
                        *state = if open { "open" } else { "closed" }.to_owned();
                    }
                }
            }
        }
    }

    fn delete_entity(&mut self, entity_id: EntityId, entity_net_id: EntityNetId) {
        if let Some(player) = self.world.get_component::<PlayerComponent>(entity_id) {
            self.players.remove(&player.identity_id);
        }
        self.ui_sessions
            .retain(|_, session| session.owner != entity_net_id && session.target != entity_net_id);
        self.world.despawn(entity_id);
        self.network_entities.remove(&entity_net_id);
        self.entity_revisions.remove(&entity_net_id);
        self.movement_replication_sequences.remove(&entity_net_id);
        self.spatial
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(entity_id);
        self.script_dirty_entities.remove(&entity_net_id);
        self.script_removed_entities.push(entity_net_id);
    }

    fn upsert_spatial_entity(&mut self, entity_id: EntityId) {
        let Some(transform) = self.world.get_component::<Transform>(entity_id).cloned() else {
            self.spatial
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(entity_id);
            return;
        };
        if self
            .world
            .get_component::<ContainedInComponent>(entity_id)
            .is_some()
        {
            self.spatial
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(entity_id);
            return;
        }
        let radius = self
            .world
            .get_component::<ColliderComponent>(entity_id)
            .map(|collider| collider.radius)
            .unwrap_or(0.05);
        self.spatial
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert_circle(
                entity_id,
                self.map.map_hash,
                transform.z,
                transform.position.x,
                transform.position.y,
                radius,
            );
    }

    fn rebuild_spatial(&mut self) {
        let mut index = self
            .spatial
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        index.clear();
        for entity_id in self.world.query_ids2::<Transform, NetworkIdentity>() {
            if self
                .world
                .get_component::<ContainedInComponent>(entity_id)
                .is_some()
            {
                continue;
            }
            let Some(transform) = self.world.get_component::<Transform>(entity_id) else {
                continue;
            };
            let radius = self
                .world
                .get_component::<ColliderComponent>(entity_id)
                .map(|collider| collider.radius)
                .unwrap_or(0.05);
            index.insert_circle(
                entity_id,
                self.map.map_hash,
                transform.z,
                transform.position.x,
                transform.position.y,
                radius,
            );
        }
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

fn parse_collider(component: &ComponentDefinition) -> ColliderComponent {
    let mut radius = field_f32(component, "radius", 0.32).max(0.01);
    if let Some(shapes) = component
        .fields
        .get("shapes")
        .and_then(YamlValue::as_sequence)
    {
        for shape in shapes {
            if let Some(mapping) = shape.as_mapping() {
                if mapping_string(mapping, "type").as_deref() == Some("Circle") {
                    radius = mapping_f32(mapping, "radius").unwrap_or(radius).max(0.01);
                    break;
                }
            }
        }
    }
    ColliderComponent {
        radius,
        collision_layer: field_u32(component, "collisionLayer", 1),
        collision_mask: field_u32(component, "collisionMask", u32::MAX),
        sensor: field_bool(component, "sensor", false),
    }
}

fn parse_sprite(component: &ComponentDefinition) -> Result<SpriteComponent> {
    let draw_depth = field_string(component, "drawDepth").unwrap_or_default();
    let default_z = match draw_depth.as_str() {
        "Floor" => 0,
        "Structures" => 5,
        "Items" => 7,
        "Mobs" => 10,
        "Effects" => 20,
        _ => 0,
    };
    let mut layers = Vec::new();
    if let Some(sequence) = component
        .fields
        .get("layers")
        .and_then(YamlValue::as_sequence)
    {
        for (index, value) in sequence.iter().enumerate() {
            let Some(mapping) = value.as_mapping() else {
                continue;
            };
            let key = mapping_string(mapping, "map").unwrap_or_else(|| format!("layer-{index}"));
            let source = if let Some(texture) = mapping_string(mapping, "texture") {
                SpriteSourceSnapshot::Texture { path: texture }
            } else if let Some(sprite) = mapping_string(mapping, "sprite") {
                SpriteSourceSnapshot::Rsi {
                    path: sprite,
                    state: mapping_string(mapping, "state").unwrap_or_else(|| "default".to_owned()),
                }
            } else {
                continue;
            };
            layers.push(SpriteLayerSnapshot {
                key,
                source,
                visible: mapping_bool(mapping, "visible").unwrap_or(true),
                color: mapping_color(mapping, "color").unwrap_or([255, 255, 255, 255]),
                scale: mapping_vec2(mapping, "scale").unwrap_or([1.0, 1.0]),
                offset: mapping_vec2(mapping, "offset").unwrap_or([0.0, 0.0]),
                rotation: mapping_f32(mapping, "rotation").unwrap_or(0.0),
                z_index: mapping_i32(mapping, "zIndex").unwrap_or(default_z),
                direction: mapping_u8(mapping, "direction").unwrap_or(0),
            });
        }
    }
    Ok(SpriteComponent { layers })
}

fn yaml_fields_to_json(component: &ComponentDefinition) -> Result<JsonValue> {
    serde_json::to_value(&component.fields).context("failed to convert component fields to JSON")
}

fn mapping_value<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a YamlValue> {
    mapping.get(&YamlValue::String(key.to_owned()))
}

fn mapping_string(mapping: &Mapping, key: &str) -> Option<String> {
    mapping_value(mapping, key)
        .and_then(YamlValue::as_str)
        .map(ToOwned::to_owned)
}

fn mapping_bool(mapping: &Mapping, key: &str) -> Option<bool> {
    mapping_value(mapping, key).and_then(YamlValue::as_bool)
}

fn mapping_f32(mapping: &Mapping, key: &str) -> Option<f32> {
    mapping_value(mapping, key)
        .and_then(YamlValue::as_f64)
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
}

fn mapping_i32(mapping: &Mapping, key: &str) -> Option<i32> {
    mapping_value(mapping, key)
        .and_then(YamlValue::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn mapping_u8(mapping: &Mapping, key: &str) -> Option<u8> {
    mapping_value(mapping, key)
        .and_then(YamlValue::as_u64)
        .and_then(|value| u8::try_from(value).ok())
}

fn mapping_vec2(mapping: &Mapping, key: &str) -> Option<[f32; 2]> {
    let values = mapping_value(mapping, key)?.as_sequence()?;
    if values.len() != 2 {
        return None;
    }
    Some([values[0].as_f64()? as f32, values[1].as_f64()? as f32])
}

fn mapping_color(mapping: &Mapping, key: &str) -> Option<[u8; 4]> {
    let values = mapping_value(mapping, key)?.as_sequence()?;
    if values.len() != 4 {
        return None;
    }
    Some([
        u8::try_from(values[0].as_u64()?).ok()?,
        u8::try_from(values[1].as_u64()?).ok()?,
        u8::try_from(values[2].as_u64()?).ok()?,
        u8::try_from(values[3].as_u64()?).ok()?,
    ])
}

fn component_schema_root() -> PathBuf {
    let configured = std::env::var_os("HONKNET_COMPONENT_SCHEMAS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/minimal-game/content/component-schemas"));
    if configured.is_absolute() {
        configured
    } else {
        workspace_root().join(configured)
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn env_f32(name: &str, fallback: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(fallback)
}

fn env_usize(name: &str, fallback: usize, minimum: usize, maximum: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(fallback)
        .clamp(minimum, maximum)
}

fn json_size(value: &JsonValue) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
}

fn truncate_text(value: String, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

fn valid_resource_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 512
        && path.starts_with('/')
        && !path.contains("..")
        && path
            .chars()
            .all(|character| !character.is_control() && character != '\\')
}

fn valid_ui_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 160
        && key.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '/')
        })
        && !key.contains("..")
}

fn optional_f32(state: &JsonValue, key: &str) -> Result<Option<f32>, String> {
    match state.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_f64()
            .map(|number| number as f32)
            .filter(|number| number.is_finite())
            .map(Some)
            .ok_or_else(|| format!("{key} must be a finite number")),
    }
}

fn optional_i32(state: &JsonValue, key: &str) -> Result<Option<i32>, String> {
    match state.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_i64()
            .and_then(|number| i32::try_from(number).ok())
            .map(Some)
            .ok_or_else(|| format!("{key} must be a 32-bit integer")),
    }
}

fn optional_u32(state: &JsonValue, key: &str) -> Result<Option<u32>, String> {
    match state.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|number| u32::try_from(number).ok())
            .map(Some)
            .ok_or_else(|| format!("{key} must be a 32-bit unsigned integer")),
    }
}

fn sanitize_display_name(value: &str) -> Result<String, String> {
    let value = value.trim();
    let length = value.chars().count();
    if !(1..=64).contains(&length) || value.chars().any(char::is_control) {
        return Err("Player.displayName must contain 1 to 64 printable characters".to_owned());
    }
    Ok(value.to_owned())
}

fn sanitize_item_name(value: &str) -> Result<String, String> {
    let value = value.trim();
    let length = value.chars().count();
    if !(1..=96).contains(&length) || value.chars().any(char::is_control) {
        return Err("Item.name must contain 1 to 96 printable characters".to_owned());
    }
    Ok(value.to_owned())
}

fn sanitize_item_size(value: &str) -> Result<String, String> {
    const ALLOWED: &[&str] = &["Tiny", "Small", "Normal", "Large", "Huge"];
    ALLOWED
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(value.trim()))
        .map(|candidate| (*candidate).to_owned())
        .ok_or_else(|| format!("Item.size must be one of {}", ALLOWED.join(", ")))
}

fn validate_sprite_layers(layers: &[SpriteLayerSnapshot]) -> Result<(), String> {
    if layers.len() > 64 {
        return Err("Sprite cannot contain more than 64 layers".to_owned());
    }
    for layer in layers {
        if layer.key.is_empty() || layer.key.len() > 96 || layer.key.chars().any(char::is_control) {
            return Err("Sprite layer keys must contain 1 to 96 printable characters".to_owned());
        }
        if !layer
            .scale
            .iter()
            .all(|value| value.is_finite() && value.abs() <= 64.0)
            || !layer
                .offset
                .iter()
                .all(|value| value.is_finite() && value.abs() <= 4_096.0)
            || !layer.rotation.is_finite()
            || layer.direction > 7
        {
            return Err(format!(
                "Sprite layer {} has invalid transform values",
                layer.key
            ));
        }
        let path = match &layer.source {
            SpriteSourceSnapshot::Texture { path } | SpriteSourceSnapshot::Rsi { path, .. } => path,
        };
        if !valid_resource_path(path) {
            return Err(format!(
                "Sprite layer {} has an invalid resource path",
                layer.key
            ));
        }
        if let SpriteSourceSnapshot::Rsi { state, .. } = &layer.source {
            if state.is_empty() || state.len() > 128 || state.chars().any(char::is_control) {
                return Err(format!(
                    "Sprite layer {} has an invalid RSI state",
                    layer.key
                ));
            }
        }
    }
    Ok(())
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
        let player = state.connect_player(Uuid::new_v4(), "guest-test".to_owned());
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

    #[test]
    fn persistence_roundtrip_restores_world_entities() {
        let state = ServerState::new_debug().unwrap();
        let snapshot = state.persistence_snapshot();
        assert_eq!(snapshot.version, 2);
        assert!(!snapshot.entities.is_empty());
    }
}
