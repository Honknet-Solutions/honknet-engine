use honknet_core::Entity;
use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
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
        self.lifecycle.insert(e, Lifecycle::Removed);
        Ok(())
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
    pub fn query<T: Component>(&self) -> Vec<Entity> {
        self.entities().filter(|e| self.contains::<T>(*e)).collect()
    }
    pub fn query2<A: Component, B: Component>(&self) -> Vec<Entity> {
        self.entities()
            .filter(|e| self.contains::<A>(*e) && self.contains::<B>(*e))
            .collect()
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
}
