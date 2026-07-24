use honknet_core::Entity;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};
use thiserror::Error;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Allocated,
    ComponentsAttached,
    Initialized,
    Started,
    Active,
    Stopping,
    Removed,
}

pub trait Component: Any + Send + Sync {
    fn on_add(&mut self, _e: Entity) {}
    fn on_initialize(&mut self, _e: Entity) {}
    fn on_start(&mut self, _e: Entity) {}
    fn on_change(&mut self, _e: Entity) {}
    fn on_stop(&mut self, _e: Entity) {}
    fn on_remove(&mut self, _e: Entity) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Packed,
    Sparse,
}

#[derive(Debug, Error)]
pub enum EcsError {
    #[error("entity is stale: {0:?}")]
    Stale(Entity),
    #[error("component already exists")]
    Duplicate,
    #[error("invalid dynamic identifier: {0}")]
    InvalidDynamicId(String),
    #[error("dynamic component is not registered: {0}")]
    UnregisteredDynamicComponent(String),
    #[error("relation kind is not registered: {0}")]
    UnregisteredRelation(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DynamicId(String);

impl DynamicId {
    pub fn new(value: impl Into<String>) -> Result<Self, EcsError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic);
        if valid {
            Ok(Self(value))
        } else {
            Err(EcsError::InvalidDynamicId(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicComponentState {
    pub value: serde_json::Value,
    pub changed_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityRelation {
    pub kind: DynamicId,
    pub source: Entity,
    pub target: Entity,
}

#[derive(Default)]
struct EntityAllocator {
    generations: Vec<u32>,
    free: Vec<u32>,
    alive: HashSet<u32>,
}

impl EntityAllocator {
    fn allocate(&mut self) -> Entity {
        let index = self.free.pop().unwrap_or_else(|| {
            self.generations.push(0);
            (self.generations.len() - 1) as u32
        });
        self.alive.insert(index);
        Entity::new(index, self.generations[index as usize])
    }
    fn valid(&self, e: Entity) -> bool {
        self.alive.contains(&e.index)
            && self.generations.get(e.index as usize) == Some(&e.generation)
    }
    fn free(&mut self, e: Entity) -> bool {
        if !self.valid(e) {
            return false;
        }
        self.alive.remove(&e.index);
        self.generations[e.index as usize] = self.generations[e.index as usize].wrapping_add(1);
        self.free.push(e.index);
        true
    }
}

trait ErasedStorage: Send + Sync {
    fn remove(&mut self, e: Entity);
    fn has(&self, e: Entity) -> bool;
    fn changed(&self, e: Entity) -> Option<u64>;
    fn on_initialize(&mut self, e: Entity);
    fn on_start(&mut self, e: Entity);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct PackedStorage<T: Component> {
    entities: Vec<Entity>,
    values: Vec<T>,
    indices: HashMap<Entity, usize>,
    changed: HashMap<Entity, u64>,
}

impl<T: Component> Default for PackedStorage<T> {
    fn default() -> Self {
        Self {
            entities: vec![],
            values: vec![],
            indices: HashMap::new(),
            changed: HashMap::new(),
        }
    }
}

impl<T: Component> PackedStorage<T> {
    fn insert(&mut self, e: Entity, mut v: T, t: u64) -> Result<(), EcsError> {
        if self.indices.contains_key(&e) {
            return Err(EcsError::Duplicate);
        }
        v.on_add(e);
        self.indices.insert(e, self.values.len());
        self.entities.push(e);
        self.values.push(v);
        self.changed.insert(e, t);
        Ok(())
    }
    fn get(&self, e: Entity) -> Option<&T> {
        self.indices.get(&e).map(|i| &self.values[*i])
    }
    fn get_mut(&mut self, e: Entity, t: u64) -> Option<&mut T> {
        let i = *self.indices.get(&e)?;
        self.changed.insert(e, t);
        self.values[i].on_change(e);
        Some(&mut self.values[i])
    }
    fn remove_typed(&mut self, e: Entity) {
        if let Some(i) = self.indices.remove(&e) {
            let mut v = self.values.swap_remove(i);
            self.entities.swap_remove(i);
            v.on_stop(e);
            v.on_remove(e);
            self.changed.remove(&e);
            if i < self.entities.len() {
                self.indices.insert(self.entities[i], i);
            }
        }
    }
}

impl<T: Component> ErasedStorage for PackedStorage<T> {
    fn remove(&mut self, e: Entity) {
        self.remove_typed(e)
    }
    fn has(&self, e: Entity) -> bool {
        self.indices.contains_key(&e)
    }
    fn changed(&self, e: Entity) -> Option<u64> {
        self.changed.get(&e).copied()
    }
    fn on_initialize(&mut self, e: Entity) {
        if let Some(&i) = self.indices.get(&e) {
            self.values[i].on_initialize(e);
        }
    }
    fn on_start(&mut self, e: Entity) {
        if let Some(&i) = self.indices.get(&e) {
            self.values[i].on_start(e);
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

struct SparseStorage<T: Component> {
    values: HashMap<Entity, T>,
    changed: HashMap<Entity, u64>,
}

impl<T: Component> Default for SparseStorage<T> {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            changed: HashMap::new(),
        }
    }
}

impl<T: Component> ErasedStorage for SparseStorage<T> {
    fn remove(&mut self, e: Entity) {
        if let Some(mut v) = self.values.remove(&e) {
            v.on_stop(e);
            v.on_remove(e);
        }
        self.changed.remove(&e);
    }
    fn has(&self, e: Entity) -> bool {
        self.values.contains_key(&e)
    }
    fn changed(&self, e: Entity) -> Option<u64> {
        self.changed.get(&e).copied()
    }
    fn on_initialize(&mut self, e: Entity) {
        if let Some(v) = self.values.get_mut(&e) {
            v.on_initialize(e);
        }
    }
    fn on_start(&mut self, e: Entity) {
        if let Some(v) = self.values.get_mut(&e) {
            v.on_start(e);
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
pub struct Resources {
    inner: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    pub fn insert<T: Any + Send + Sync>(&mut self, v: T) {
        self.inner.insert(TypeId::of::<T>(), Box::new(v));
    }
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.inner.get(&TypeId::of::<T>())?.downcast_ref()
    }
    pub fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T> {
        self.inner.get_mut(&TypeId::of::<T>())?.downcast_mut()
    }
}

#[derive(Default)]
pub struct World {
    allocator: EntityAllocator,
    lifecycle: HashMap<Entity, Lifecycle>,
    storages: HashMap<TypeId, Box<dyn ErasedStorage>>,
    kinds: HashMap<TypeId, StorageKind>,
    dynamic_component_types: HashSet<DynamicId>,
    dynamic_components: HashMap<DynamicId, HashMap<Entity, DynamicComponentState>>,
    relation_types: HashSet<DynamicId>,
    relations: HashMap<DynamicId, HashMap<Entity, HashSet<Entity>>>,
    tick: u64,
    pub resources: Resources,
}

impl World {
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn advance_tick(&mut self) {
        self.tick += 1
    }
    pub fn spawn(&mut self) -> Entity {
        let e = self.allocator.allocate();
        self.lifecycle.insert(e, Lifecycle::Allocated);
        e
    }
    pub fn is_alive(&self, e: Entity) -> bool {
        self.allocator.valid(e)
    }
    pub fn despawn(&mut self, e: Entity) -> Result<(), EcsError> {
        if !self.allocator.free(e) {
            return Err(EcsError::Stale(e));
        }
        self.lifecycle.insert(e, Lifecycle::Stopping);
        for s in self.storages.values_mut() {
            s.remove(e)
        }
        for components in self.dynamic_components.values_mut() {
            components.remove(&e);
        }
        for relations in self.relations.values_mut() {
            relations.remove(&e);
            for targets in relations.values_mut() {
                targets.remove(&e);
            }
        }
        self.lifecycle.insert(e, Lifecycle::Removed);
        Ok(())
    }
    pub fn register_dynamic_component(&mut self, id: DynamicId) -> bool {
        self.dynamic_components.entry(id.clone()).or_default();
        self.dynamic_component_types.insert(id)
    }
    pub fn set_dynamic_component(
        &mut self,
        entity: Entity,
        id: &DynamicId,
        value: serde_json::Value,
    ) -> Result<(), EcsError> {
        if !self.is_alive(entity) {
            return Err(EcsError::Stale(entity));
        }
        if !self.dynamic_component_types.contains(id) {
            return Err(EcsError::UnregisteredDynamicComponent(
                id.as_str().to_string(),
            ));
        }
        self.dynamic_components
            .get_mut(id)
            .expect("registered dynamic component storage")
            .insert(
                entity,
                DynamicComponentState {
                    value,
                    changed_tick: self.tick,
                },
            );
        Ok(())
    }
    pub fn dynamic_component(
        &self,
        entity: Entity,
        id: &DynamicId,
    ) -> Option<&DynamicComponentState> {
        self.dynamic_components.get(id)?.get(&entity)
    }
    pub fn remove_dynamic_component(
        &mut self,
        entity: Entity,
        id: &DynamicId,
    ) -> Result<bool, EcsError> {
        if !self.dynamic_component_types.contains(id) {
            return Err(EcsError::UnregisteredDynamicComponent(
                id.as_str().to_string(),
            ));
        }
        Ok(self
            .dynamic_components
            .get_mut(id)
            .and_then(|components| components.remove(&entity))
            .is_some())
    }
    pub fn dynamic_entity_snapshot(&self, entity: Entity) -> BTreeMap<String, serde_json::Value> {
        self.dynamic_components
            .iter()
            .filter_map(|(id, components)| {
                components
                    .get(&entity)
                    .map(|state| (id.as_str().to_string(), state.value.clone()))
            })
            .collect()
    }
    pub fn register_relation(&mut self, kind: DynamicId) -> bool {
        self.relations.entry(kind.clone()).or_default();
        self.relation_types.insert(kind)
    }
    pub fn add_relation(
        &mut self,
        kind: &DynamicId,
        source: Entity,
        target: Entity,
    ) -> Result<bool, EcsError> {
        if !self.relation_types.contains(kind) {
            return Err(EcsError::UnregisteredRelation(kind.as_str().to_string()));
        }
        if !self.is_alive(source) {
            return Err(EcsError::Stale(source));
        }
        if !self.is_alive(target) {
            return Err(EcsError::Stale(target));
        }
        Ok(self
            .relations
            .get_mut(kind)
            .expect("registered relation storage")
            .entry(source)
            .or_default()
            .insert(target))
    }
    pub fn remove_relation(
        &mut self,
        kind: &DynamicId,
        source: Entity,
        target: Entity,
    ) -> Result<bool, EcsError> {
        if !self.relation_types.contains(kind) {
            return Err(EcsError::UnregisteredRelation(kind.as_str().to_string()));
        }
        Ok(self
            .relations
            .get_mut(kind)
            .and_then(|sources| sources.get_mut(&source))
            .is_some_and(|targets| targets.remove(&target)))
    }
    pub fn relation_targets(&self, kind: &DynamicId, source: Entity) -> Vec<Entity> {
        let mut targets: Vec<_> = self
            .relations
            .get(kind)
            .and_then(|sources| sources.get(&source))
            .into_iter()
            .flatten()
            .copied()
            .filter(|entity| self.is_alive(*entity))
            .collect();
        targets.sort_unstable();
        targets
    }
    pub fn relation_snapshot(&self) -> Vec<EntityRelation> {
        let mut relations = Vec::new();
        for (kind, sources) in &self.relations {
            for (source, targets) in sources {
                for target in targets {
                    if self.is_alive(*source) && self.is_alive(*target) {
                        relations.push(EntityRelation {
                            kind: kind.clone(),
                            source: *source,
                            target: *target,
                        });
                    }
                }
            }
        }
        relations.sort_by(|left, right| {
            (&left.kind, left.source, left.target).cmp(&(&right.kind, right.source, right.target))
        });
        relations
    }
    pub fn register<T: Component>(&mut self, kind: StorageKind) {
        self.kinds.insert(TypeId::of::<T>(), kind);
        self.storages
            .entry(TypeId::of::<T>())
            .or_insert_with(|| match kind {
                StorageKind::Packed => Box::<PackedStorage<T>>::default(),
                StorageKind::Sparse => Box::<SparseStorage<T>>::default(),
            });
    }
    pub fn insert<T: Component>(&mut self, e: Entity, v: T) -> Result<(), EcsError> {
        if !self.is_alive(e) {
            return Err(EcsError::Stale(e));
        }
        let kind = *self
            .kinds
            .get(&TypeId::of::<T>())
            .unwrap_or(&StorageKind::Packed);
        self.register::<T>(kind);
        let s = self.storages.get_mut(&TypeId::of::<T>()).unwrap();
        match kind {
            StorageKind::Packed => s
                .as_any_mut()
                .downcast_mut::<PackedStorage<T>>()
                .unwrap()
                .insert(e, v, self.tick),
            StorageKind::Sparse => {
                let s = s.as_any_mut().downcast_mut::<SparseStorage<T>>().unwrap();
                if s.values.contains_key(&e) {
                    return Err(EcsError::Duplicate);
                }
                let mut v = v;
                v.on_add(e);
                s.values.insert(e, v);
                s.changed.insert(e, self.tick);
                Ok(())
            }
        }
    }
    pub fn get<T: Component>(&self, e: Entity) -> Option<&T> {
        let s = self.storages.get(&TypeId::of::<T>())?;
        if let Some(p) = s.as_any().downcast_ref::<PackedStorage<T>>() {
            p.get(e)
        } else {
            s.as_any()
                .downcast_ref::<SparseStorage<T>>()?
                .values
                .get(&e)
        }
    }
    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<&mut T> {
        let t = self.tick;
        let s = self.storages.get_mut(&TypeId::of::<T>())?;
        if s.as_any().is::<PackedStorage<T>>() {
            s.as_any_mut()
                .downcast_mut::<PackedStorage<T>>()?
                .get_mut(e, t)
        } else {
            let s = s.as_any_mut().downcast_mut::<SparseStorage<T>>()?;
            s.changed.insert(e, t);
            s.values.get_mut(&e)
        }
    }
    pub fn contains<T: Component>(&self, e: Entity) -> bool {
        self.storages
            .get(&TypeId::of::<T>())
            .is_some_and(|s| s.has(e))
    }
    pub fn changed_since<T: Component>(&self, e: Entity, t: u64) -> bool {
        self.storages
            .get(&TypeId::of::<T>())
            .and_then(|s| s.changed(e))
            .is_some_and(|x| x >= t)
    }
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.lifecycle
            .iter()
            .filter_map(|(e, l)| (*l != Lifecycle::Removed).then_some(*e))
    }
    pub fn query_iter<T: Component>(&self) -> impl Iterator<Item = Entity> + '_ {
        if let Some(s) = self.storages.get(&TypeId::of::<T>()) {
            if let Some(p) = s.as_any().downcast_ref::<PackedStorage<T>>() {
                return Box::new(p.entities.iter().copied().filter(|e| self.is_alive(*e)))
                    as Box<dyn Iterator<Item = Entity> + '_>;
            }
        }
        Box::new(self.entities().filter(|e| self.contains::<T>(*e)))
            as Box<dyn Iterator<Item = Entity> + '_>
    }

    pub fn query<T: Component>(&self) -> Vec<Entity> {
        self.query_iter::<T>().collect()
    }

    pub fn query2_iter<A: Component, B: Component>(&self) -> impl Iterator<Item = Entity> + '_ {
        self.query_iter::<A>().filter(|e| self.contains::<B>(*e))
    }

    pub fn query2<A: Component, B: Component>(&self) -> Vec<Entity> {
        self.query2_iter::<A, B>().collect()
    }

    pub fn query_with<T: Component, F: FnMut(Entity, &T)>(&self, mut f: F) {
        if let Some(s) = self.storages.get(&TypeId::of::<T>()) {
            if let Some(p) = s.as_any().downcast_ref::<PackedStorage<T>>() {
                for (i, &e) in p.entities.iter().enumerate() {
                    if self.is_alive(e) {
                        f(e, &p.values[i]);
                    }
                }
                return;
            }
        }
        for e in self.query_iter::<T>() {
            if let Some(comp) = self.get::<T>(e) {
                f(e, comp);
            }
        }
    }

    pub fn query2_with<A: Component, B: Component, F: FnMut(Entity, &A, &B)>(&self, mut f: F) {
        for e in self.query2_iter::<A, B>() {
            if let (Some(a), Some(b)) = (self.get::<A>(e), self.get::<B>(e)) {
                f(e, a, b);
            }
        }
    }
    pub fn initialize(&mut self, e: Entity) -> Result<(), EcsError> {
        if !self.is_alive(e) {
            return Err(EcsError::Stale(e));
        }
        self.lifecycle.insert(e, Lifecycle::Initialized);
        for s in self.storages.values_mut() {
            s.on_initialize(e);
        }
        self.lifecycle.insert(e, Lifecycle::Started);
        for s in self.storages.values_mut() {
            s.on_start(e);
        }
        self.lifecycle.insert(e, Lifecycle::Active);
        Ok(())
    }
    pub fn remove_component<T: Component>(&mut self, e: Entity) -> bool {
        if !self.is_alive(e) {
            return false;
        }
        if let Some(s) = self.storages.get_mut(&TypeId::of::<T>()) {
            if s.has(e) {
                s.remove(e);
                return true;
            }
        }
        false
    }
}

#[allow(clippy::type_complexity)]
pub enum Command {
    Spawn(Vec<Box<dyn FnOnce(&mut World, Entity) + Send>>),
    Despawn(Entity),
    Run(Box<dyn FnOnce(&mut World) + Send>),
    TryRun(Box<dyn FnOnce(&mut World) -> Result<(), EcsError> + Send>),
}

#[derive(Default)]
pub struct CommandBuffer {
    commands: Vec<Command>,
}

impl CommandBuffer {
    pub fn spawn<F: FnOnce(&mut World, Entity) + Send + 'static>(&mut self, f: F) {
        self.commands.push(Command::Spawn(vec![Box::new(f)]))
    }
    pub fn despawn(&mut self, e: Entity) {
        self.commands.push(Command::Despawn(e))
    }
    pub fn run<F: FnOnce(&mut World) + Send + 'static>(&mut self, f: F) {
        self.commands.push(Command::Run(Box::new(f)))
    }
    pub fn try_run<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World) -> Result<(), EcsError> + Send + 'static,
    {
        self.commands.push(Command::TryRun(Box::new(f)))
    }
    pub fn apply(mut self, world: &mut World) -> Result<(), EcsError> {
        for c in self.commands.drain(..) {
            match c {
                Command::Spawn(fs) => {
                    let e = world.spawn();
                    for f in fs {
                        f(world, e)
                    }
                    world.initialize(e)?
                }
                Command::Despawn(e) => world.despawn(e)?,
                Command::Run(f) => f(world),
                Command::TryRun(f) => f(world)?,
            }
        }
        Ok(())
    }
}

pub type SharedWorld = Arc<RwLock<World>>;
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug)]
    #[allow(dead_code)]
    struct Pos(i32);
    impl Component for Pos {}
    #[test]
    fn generation_prevents_stale_access() {
        let mut w = World::default();
        let a = w.spawn();
        w.insert(a, Pos(1)).unwrap();
        w.despawn(a).unwrap();
        let b = w.spawn();
        assert_eq!(a.index, b.index);
        assert_ne!(a.generation, b.generation);
        assert!(w.get::<Pos>(a).is_none())
    }
    #[test]
    fn command_buffer_applies() {
        let mut w = World::default();
        let mut c = CommandBuffer::default();
        c.spawn(|w, e| {
            w.insert(e, Pos(9)).unwrap();
        });
        c.apply(&mut w).unwrap();
        assert_eq!(w.query::<Pos>().len(), 1)
    }
    #[test]
    fn component_removal() {
        let mut w = World::default();
        let e = w.spawn();
        w.insert(e, Pos(42)).unwrap();
        assert!(w.contains::<Pos>(e));
        assert!(w.remove_component::<Pos>(e));
        assert!(!w.contains::<Pos>(e));
        assert!(!w.remove_component::<Pos>(e));
    }
    #[test]
    fn zero_allocation_query_iterators() {
        let mut w = World::default();
        let e1 = w.spawn();
        w.insert(e1, Pos(10)).unwrap();
        let e2 = w.spawn();
        w.insert(e2, Pos(20)).unwrap();

        let mut count = 0;
        let mut sum = 0;
        w.query_with::<Pos, _>(|_e, pos| {
            count += 1;
            sum += pos.0;
        });
        assert_eq!(count, 2);
        assert_eq!(sum, 30);
    }

    #[test]
    fn dynamic_components_require_stable_registration() {
        let mut world = World::default();
        let entity = world.spawn();
        let health = DynamicId::new("game.health").unwrap();

        assert!(matches!(
            world.set_dynamic_component(entity, &health, serde_json::json!({ "blood": 100 })),
            Err(EcsError::UnregisteredDynamicComponent(_))
        ));
        assert!(world.register_dynamic_component(health.clone()));
        world
            .set_dynamic_component(entity, &health, serde_json::json!({ "blood": 100 }))
            .unwrap();

        assert_eq!(
            world.dynamic_component(entity, &health).unwrap().value,
            serde_json::json!({ "blood": 100 })
        );
    }

    #[test]
    fn despawn_cleans_dynamic_components_and_relations() {
        let mut world = World::default();
        let parent = world.spawn();
        let child = world.spawn();
        let body_part = DynamicId::new("game.bodyPart").unwrap();
        let attached_to = DynamicId::new("game.attachedTo").unwrap();
        world.register_dynamic_component(body_part.clone());
        world.register_relation(attached_to.clone());
        world
            .set_dynamic_component(child, &body_part, serde_json::json!({ "zone": "arm" }))
            .unwrap();
        world.add_relation(&attached_to, child, parent).unwrap();

        world.despawn(parent).unwrap();

        assert!(world.relation_targets(&attached_to, child).is_empty());
        assert!(world.dynamic_component(child, &body_part).is_some());
        world.despawn(child).unwrap();
        assert!(world.dynamic_component(child, &body_part).is_none());
    }

    #[test]
    fn dynamic_identifiers_reject_unsafe_names() {
        assert!(DynamicId::new("game.health").is_ok());
        assert!(DynamicId::new("../filesystem").is_err());
        assert!(DynamicId::new("").is_err());
        assert!(DynamicId::new("9component").is_err());
    }
}
