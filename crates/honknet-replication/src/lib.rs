use honknet_core::Entity;
use honknet_math::Vec2;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub type ComponentNetId = u16;
pub type SchemaVersion = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: String,
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplicationMode {
    Replicated,
    OwnerOnly,
    ObserverOnly,
    ServerOnly,
    InitialOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterpolationPolicy {
    Linear,
    SphericalLinear,
    Step,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PredictionPolicy {
    Authoritative,
    Predictive,
    Local,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PersistencePolicy {
    Persistent,
    Transient,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkComponentDescriptor {
    pub net_id: ComponentNetId,
    pub name: String,
    pub version: SchemaVersion,
    pub mode: ReplicationMode,
    pub interpolation: InterpolationPolicy,
    pub prediction: PredictionPolicy,
    pub persistence: PersistencePolicy,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirtyMask {
    pub mask: Vec<u64>,
}

impl DirtyMask {
    pub fn new() -> Self {
        Self { mask: Vec::new() }
    }
    pub fn mark(&mut self, field: usize) {
        let word = field / 64;
        if self.mask.len() <= word {
            self.mask.resize(word + 1, 0)
        }
        self.mask[word] |= 1 << (field % 64)
    }
    pub fn clean(&mut self) {
        self.mask.fill(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    pub component_id: ComponentNetId,
    pub revision: u64,
    pub dirty_mask: DirtyMask,
    pub bytes: Vec<u8>,
    pub mode: ReplicationMode,
}

impl ComponentState {
    pub fn encode<T: Serialize>(
        component_id: ComponentNetId,
        revision: u64,
        mode: ReplicationMode,
        val: &T,
    ) -> Self {
        let bytes =
            bincode::serde::encode_to_vec(val, bincode::config::standard()).unwrap_or_default();
        Self {
            component_id,
            revision,
            dirty_mask: DirtyMask::default(),
            bytes,
            mode,
        }
    }

    pub fn decode<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        let (val, _) =
            bincode::serde::decode_from_slice(&self.bytes, bincode::config::standard()).ok()?;
        Some(val)
    }
}

pub const NET_ID_TRANSFORM: ComponentNetId = 1;
pub const NET_ID_SPRITE: ComponentNetId = 2;
pub const NET_ID_METADATA: ComponentNetId = 3;
pub const NET_ID_PHYSICS: ComponentNetId = 4;
pub const NET_ID_MAP_GRID: ComponentNetId = 5;
pub const NET_ID_CONTAINER: ComponentNetId = 6;
pub const NET_ID_MOB_STATUS: ComponentNetId = 100;
pub const NET_ID_HANDS: ComponentNetId = 101;
pub const NET_ID_EQUIPMENT: ComponentNetId = 102;
pub const NET_ID_MEDICAL_STATUS: ComponentNetId = 103;
pub const NET_ID_INTERACTION_STATUS: ComponentNetId = 104;
pub const NET_ID_PREDICTION_ACK: ComponentNetId = 105;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetTransformComponent {
    pub position: Vec2,
    pub rotation: f32,
    pub parent_entity: Option<Entity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetSpriteComponent {
    pub rsi_path: String,
    pub state: String,
    pub color: u32,
    pub visible: bool,
    pub layer: i16,
    pub direction: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetMetadataComponent {
    pub name: String,
    pub description: String,
    pub prototype_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetPhysicsComponent {
    pub velocity: Vec2,
    pub angular_velocity: f32,
    pub mass: f32,
    pub body_type: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetMapGridComponent {
    pub grid_id: u32,
    pub chunk_size: u32,
    pub tile_size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetContainerComponent {
    pub container_id: String,
    pub contained_entities: Vec<Entity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetMobStatusComponent {
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetHandsComponent {
    pub active_hand: u8,
    pub held_item: Option<Entity>,
    pub maximum_hands: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetEquipmentComponent {
    pub slots: Vec<(String, Option<Entity>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetMedicalStatusComponent {
    pub blood_fraction: f32,
    pub oxygen_saturation: f32,
    pub pain: f32,
    pub shock: f32,
    pub conscious: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInteractionStatusComponent {
    pub grabbed: Option<Entity>,
    pub grab_strength: Option<String>,
    pub pulling: Option<Entity>,
    pub carrying: Option<Entity>,
    pub buckled_to: Option<Entity>,
    pub action_kind: Option<String>,
    pub action_started_tick: Option<u64>,
    pub action_completes_tick: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetPredictionAckComponent {
    pub last_processed_input: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub entity: Entity,
    pub revision: u64,
    pub position: Vec2,
    pub owner: Option<u64>,
    pub importance: f32,
    pub frequency: u16,
    pub components: Vec<ComponentState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u64,
    pub entities: Vec<EntityState>,
}

impl honknet_net_core::NetworkMessage for Snapshot {
    const ID: u16 = 200;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub tick: u64,
    pub baseline: u64,
    pub spawns: Vec<EntityState>,
    pub updates: Vec<EntityState>,
    pub despawns: Vec<Entity>,
}

impl honknet_net_core::NetworkMessage for Delta {
    const ID: u16 = 201;
}

pub struct SnapshotBuilder {
    pub tick: u64,
    pub entities: Vec<EntityState>,
}

impl SnapshotBuilder {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            entities: Vec::new(),
        }
    }
    pub fn add(&mut self, state: EntityState) {
        self.entities.push(state);
    }
    pub fn build(self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            entities: self.entities,
        }
    }
}

pub struct DeltaBuilder {
    pub tick: u64,
    pub baseline: u64,
    pub spawns: Vec<EntityState>,
    pub updates: Vec<EntityState>,
    pub despawns: Vec<Entity>,
}

impl DeltaBuilder {
    pub fn new(tick: u64, baseline: u64) -> Self {
        Self {
            tick,
            baseline,
            spawns: vec![],
            updates: vec![],
            despawns: vec![],
        }
    }
    pub fn build(self) -> Delta {
        Delta {
            tick: self.tick,
            baseline: self.baseline,
            spawns: self.spawns,
            updates: self.updates,
            despawns: self.despawns,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerClientBudget {
    pub bytes_per_tick: usize,
    pub bytes_per_second: usize,
    pub reliable_reserve: usize,
    pub spawn_reserve: usize,
    pub critical_reserve: usize,
    pub cosmetic_budget: usize,
}

pub struct InterestContext {
    pub client: u64,
    pub controlled: Option<Entity>,
    pub position: Vec2,
    pub observers: HashSet<Entity>,
    pub forced: HashSet<Entity>,
    pub teams: HashSet<u64>,
    pub containers: HashSet<Entity>,
}

pub trait InterestProvider: Send + Sync {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool;
}

pub struct SpatialProvider {
    pub radius: f32,
}

impl InterestProvider for SpatialProvider {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool {
        (e.position - c.position).length_squared() <= self.radius * self.radius
    }
}

pub struct OwnershipProvider;
impl InterestProvider for OwnershipProvider {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool {
        e.owner == Some(c.client)
    }
}

pub struct ObserverProvider;
impl InterestProvider for ObserverProvider {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool {
        c.observers.contains(&e.entity)
    }
}

pub struct ForcedVisibilityProvider;
impl InterestProvider for ForcedVisibilityProvider {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool {
        c.forced.contains(&e.entity)
    }
}

pub struct ContainerProvider;
impl InterestProvider for ContainerProvider {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool {
        c.containers.contains(&e.entity)
    }
}

#[derive(Default)]
pub struct Replicator {
    pub states: HashMap<Entity, EntityState>,
    pub history: HashMap<(u64, u64), Snapshot>,
    pub providers: Vec<Box<dyn InterestProvider>>,
    pub max_history: usize,
}

impl Replicator {
    pub fn visible<'a>(&'a self, c: &InterestContext) -> Vec<&'a EntityState> {
        self.states
            .values()
            .filter(|e| self.providers.iter().any(|p| p.interested(c, e)))
            .collect()
    }
    pub fn build_snapshot(
        &mut self,
        tick: u64,
        c: &InterestContext,
        byte_budget: usize,
    ) -> Snapshot {
        let mut v = self.visible(c);
        v.sort_by(|a, b| priority(c, b).total_cmp(&priority(c, a)));
        let mut used = 0;
        let mut out = vec![];
        for e in v {
            let mut state = e.clone();
            state.components.retain(|component| match component.mode {
                ReplicationMode::ServerOnly => false,
                ReplicationMode::OwnerOnly => state.owner == Some(c.client),
                ReplicationMode::ObserverOnly => c.observers.contains(&state.entity),
                _ => true,
            });
            let cost = state
                .components
                .iter()
                .map(|x| x.bytes.len() + 24)
                .sum::<usize>()
                + 48;
            if used + cost > byte_budget {
                continue;
            }
            used += cost;
            out.push(state)
        }
        let s = Snapshot {
            tick,
            entities: out,
        };
        self.history.insert((c.client, tick), s.clone());
        let history_limit = self.max_history.max(32);
        let mut client_ticks = self
            .history
            .keys()
            .filter_map(|(client, tick)| (*client == c.client).then_some(*tick))
            .collect::<Vec<_>>();
        client_ticks.sort_unstable();
        let excess = client_ticks.len().saturating_sub(history_limit);
        for old_tick in client_ticks.into_iter().take(excess) {
            self.history.remove(&(c.client, old_tick));
        }
        s
    }
    pub fn delta(&self, client: u64, baseline: u64, current: &Snapshot) -> Option<Delta> {
        let old = self.history.get(&(client, baseline))?;
        let a: HashMap<_, _> = old.entities.iter().map(|e| (e.entity, e)).collect();
        let b: HashMap<_, _> = current.entities.iter().map(|e| (e.entity, e)).collect();
        Some(Delta {
            tick: current.tick,
            baseline,
            spawns: b
                .iter()
                .filter(|(k, _)| !a.contains_key(k))
                .map(|(_, e)| (*e).clone())
                .collect(),
            updates: b
                .iter()
                .filter_map(|(entity, current_entity)| {
                    let old_entity = a.get(entity)?;
                    if old_entity.revision == current_entity.revision {
                        return None;
                    }
                    let old_components = old_entity
                        .components
                        .iter()
                        .map(|component| (component.component_id, component))
                        .collect::<HashMap<_, _>>();
                    let mut update = (*current_entity).clone();
                    update.components.retain_mut(|component| {
                        let changed = old_components
                            .get(&component.component_id)
                            .is_none_or(|old| old.bytes != component.bytes);
                        if changed {
                            component.dirty_mask.mark(0);
                        }
                        changed
                    });
                    Some(update)
                })
                .collect(),
            despawns: a.keys().filter(|k| !b.contains_key(k)).copied().collect(),
        })
    }
    pub fn parallel_snapshots(
        &self,
        clients: &[(InterestContext, usize)],
        tick: u64,
    ) -> Vec<Snapshot> {
        clients
            .par_iter()
            .map(|(c, b)| {
                let mut v = self.visible(c);
                v.sort_by(|a, b| priority(c, b).total_cmp(&priority(c, a)));
                let mut used = 0;
                Snapshot {
                    tick,
                    entities: v
                        .into_iter()
                        .filter_map(|e| {
                            let mut state = e.clone();
                            state.components.retain(|component| match component.mode {
                                ReplicationMode::ServerOnly => false,
                                ReplicationMode::OwnerOnly => state.owner == Some(c.client),
                                ReplicationMode::ObserverOnly => {
                                    c.observers.contains(&state.entity)
                                }
                                _ => true,
                            });
                            let n = state
                                .components
                                .iter()
                                .map(|x| x.bytes.len() + 24)
                                .sum::<usize>()
                                + 48;
                            if used + n > *b {
                                None
                            } else {
                                used += n;
                                Some(state)
                            }
                        })
                        .collect(),
                }
            })
            .collect()
    }
}

fn priority(c: &InterestContext, e: &EntityState) -> f32 {
    let d = (e.position - c.position).length().max(1.);
    e.importance * 1000. / d
        + if e.owner == Some(c.client) {
            10000.
        } else {
            0.
        }
        + if c.forced.contains(&e.entity) {
            20000.
        } else {
            0.
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(client: u64) -> InterestContext {
        InterestContext {
            client,
            controlled: None,
            position: Vec2::ZERO,
            observers: HashSet::new(),
            forced: HashSet::new(),
            teams: HashSet::new(),
            containers: HashSet::new(),
        }
    }

    #[test]
    fn owner_only_components_never_leak_to_other_clients() {
        let entity = Entity::new(1, 0);
        let mut replicator = Replicator {
            providers: vec![Box::new(SpatialProvider { radius: 100.0 })],
            ..Replicator::default()
        };
        replicator.states.insert(
            entity,
            EntityState {
                entity,
                revision: 1,
                position: Vec2::ZERO,
                owner: Some(10),
                importance: 1.0,
                frequency: 1,
                components: vec![
                    ComponentState::encode(
                        NET_ID_MOB_STATUS,
                        1,
                        ReplicationMode::Replicated,
                        &NetMobStatusComponent {
                            state: "Alive".into(),
                        },
                    ),
                    ComponentState::encode(
                        NET_ID_MEDICAL_STATUS,
                        1,
                        ReplicationMode::OwnerOnly,
                        &NetMedicalStatusComponent {
                            blood_fraction: 1.0,
                            oxygen_saturation: 1.0,
                            pain: 0.0,
                            shock: 0.0,
                            conscious: true,
                        },
                    ),
                ],
            },
        );

        let owner = replicator.build_snapshot(1, &context(10), usize::MAX);
        let stranger = replicator.build_snapshot(1, &context(11), usize::MAX);
        assert_eq!(owner.entities[0].components.len(), 2);
        assert_eq!(stranger.entities[0].components.len(), 1);
        assert_eq!(
            stranger.entities[0].components[0].component_id,
            NET_ID_MOB_STATUS
        );
    }

    #[test]
    fn snapshot_history_is_isolated_per_client() {
        let mut replicator = Replicator {
            providers: vec![Box::new(SpatialProvider { radius: 100.0 })],
            ..Replicator::default()
        };
        replicator.build_snapshot(5, &context(1), usize::MAX);
        replicator.build_snapshot(5, &context(2), usize::MAX);
        assert!(replicator.history.contains_key(&(1, 5)));
        assert!(replicator.history.contains_key(&(2, 5)));
    }

    #[test]
    fn delta_contains_only_changed_components_and_marks_them_dirty() {
        let entity = Entity::new(2, 0);
        let old_status = ComponentState::encode(
            NET_ID_MOB_STATUS,
            1,
            ReplicationMode::Replicated,
            &NetMobStatusComponent {
                state: "Alive".into(),
            },
        );
        let unchanged_metadata = ComponentState::encode(
            NET_ID_METADATA,
            1,
            ReplicationMode::Replicated,
            &NetMetadataComponent {
                name: "Patient".into(),
                description: String::new(),
                prototype_id: "MobHuman".into(),
            },
        );
        let old = EntityState {
            entity,
            revision: 1,
            position: Vec2::ZERO,
            owner: None,
            importance: 1.0,
            frequency: 1,
            components: vec![old_status, unchanged_metadata.clone()],
        };
        let mut replicator = Replicator::default();
        replicator.history.insert(
            (7, 1),
            Snapshot {
                tick: 1,
                entities: vec![old],
            },
        );
        let current = Snapshot {
            tick: 2,
            entities: vec![EntityState {
                entity,
                revision: 2,
                position: Vec2::ZERO,
                owner: None,
                importance: 1.0,
                frequency: 1,
                components: vec![
                    ComponentState::encode(
                        NET_ID_MOB_STATUS,
                        2,
                        ReplicationMode::Replicated,
                        &NetMobStatusComponent {
                            state: "Critical".into(),
                        },
                    ),
                    unchanged_metadata,
                ],
            }],
        };

        let delta = replicator.delta(7, 1, &current).unwrap();
        assert_eq!(delta.updates.len(), 1);
        assert_eq!(delta.updates[0].components.len(), 1);
        assert_eq!(
            delta.updates[0].components[0].component_id,
            NET_ID_MOB_STATUS
        );
        assert_eq!(delta.updates[0].components[0].dirty_mask.mask, vec![1]);
    }

    #[test]
    fn leaving_interest_range_produces_a_client_despawn() {
        let entity = Entity::new(3, 0);
        let mut replicator = Replicator::default();
        replicator.history.insert(
            (8, 1),
            Snapshot {
                tick: 1,
                entities: vec![EntityState {
                    entity,
                    revision: 1,
                    position: Vec2::ZERO,
                    owner: None,
                    importance: 1.0,
                    frequency: 1,
                    components: Vec::new(),
                }],
            },
        );
        replicator.states.insert(
            entity,
            EntityState {
                entity,
                revision: 1,
                position: Vec2::new(1000.0, 0.0),
                owner: None,
                importance: 1.0,
                frequency: 1,
                components: Vec::new(),
            },
        );

        let delta = replicator
            .delta(
                8,
                1,
                &Snapshot {
                    tick: 2,
                    entities: Vec::new(),
                },
            )
            .unwrap();
        assert_eq!(delta.despawns, vec![entity]);
    }
}
