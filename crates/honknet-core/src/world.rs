use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    error::Error,
    fmt::{Display, Formatter},
};

use crate::{Component, EntityId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldError {
    EntityNotFound(EntityId),
}

impl Display for WorldError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntityNotFound(entity_id) => {
                write!(formatter, "entity {} was not found", entity_id.value())
            }
        }
    }
}

impl Error for WorldError {}

trait ErasedStorage: Send + Sync {
    fn remove_entity(&mut self, entity_id: EntityId) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct ComponentStorage<T: Component> {
    sparse: HashMap<EntityId, usize>,
    entities: Vec<EntityId>,
    components: Vec<T>,
}

impl<T: Component> Default for ComponentStorage<T> {
    fn default() -> Self {
        Self {
            sparse: HashMap::new(),
            entities: Vec::new(),
            components: Vec::new(),
        }
    }
}

impl<T: Component> ComponentStorage<T> {
    fn insert(&mut self, entity_id: EntityId, component: T) -> bool {
        if let Some(&index) = self.sparse.get(&entity_id) {
            self.components[index] = component;
            return true;
        }
        let index = self.entities.len();
        self.entities.push(entity_id);
        self.components.push(component);
        self.sparse.insert(entity_id, index);
        false
    }

    fn remove(&mut self, entity_id: EntityId) -> bool {
        let Some(index) = self.sparse.remove(&entity_id) else {
            return false;
        };
        let last = self.entities.len() - 1;
        self.entities.swap_remove(index);
        self.components.swap_remove(index);
        if index != last {
            let moved = self.entities[index];
            self.sparse.insert(moved, index);
        }
        true
    }

    fn get(&self, entity_id: EntityId) -> Option<&T> {
        self.sparse
            .get(&entity_id)
            .and_then(|&index| self.components.get(index))
    }

    fn get_mut(&mut self, entity_id: EntityId) -> Option<&mut T> {
        let index = *self.sparse.get(&entity_id)?;
        self.components.get_mut(index)
    }
}

impl<T: Component> ErasedStorage for ComponentStorage<T> {
    fn remove_entity(&mut self, entity_id: EntityId) -> bool {
        self.remove(entity_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct World {
    next_entity_id: u64,
    alive: HashSet<EntityId>,
    storages: HashMap<TypeId, Box<dyn ErasedStorage>>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EntityRef<'a> {
    world: &'a World,
    id: EntityId,
}

impl<'a> EntityRef<'a> {
    pub const fn id(&self) -> EntityId {
        self.id
    }

    pub fn contains<T: Component>(&self) -> bool {
        self.world.get_component::<T>(self.id).is_some()
    }

    pub fn get<T: Component>(&self) -> Option<&'a T> {
        self.world.get_component::<T>(self.id)
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            next_entity_id: 1,
            alive: HashSet::new(),
            storages: HashMap::new(),
        }
    }

    pub fn spawn(&mut self) -> EntityId {
        let entity_id = EntityId::new(self.next_entity_id);
        self.next_entity_id = self
            .next_entity_id
            .checked_add(1)
            .expect("entity id space exhausted");
        self.alive.insert(entity_id);
        entity_id
    }

    pub fn despawn(&mut self, entity_id: EntityId) -> Option<EntityId> {
        if !self.alive.remove(&entity_id) {
            return None;
        }
        for storage in self.storages.values_mut() {
            storage.remove_entity(entity_id);
        }
        Some(entity_id)
    }

    pub fn contains_entity(&self, entity_id: EntityId) -> bool {
        self.alive.contains(&entity_id)
    }

    pub fn entity_count(&self) -> usize {
        self.alive.len()
    }

    pub fn component_count<T: Component>(&self) -> usize {
        self.storage::<T>()
            .map_or(0, |storage| storage.entities.len())
    }

    pub fn add_component<T: Component>(
        &mut self,
        entity_id: EntityId,
        component: T,
    ) -> Result<bool, WorldError> {
        if !self.alive.contains(&entity_id) {
            return Err(WorldError::EntityNotFound(entity_id));
        }
        Ok(self
            .storage_mut_or_insert::<T>()
            .insert(entity_id, component))
    }

    pub fn remove_component<T: Component>(
        &mut self,
        entity_id: EntityId,
    ) -> Result<bool, WorldError> {
        if !self.alive.contains(&entity_id) {
            return Err(WorldError::EntityNotFound(entity_id));
        }
        Ok(self
            .storage_mut::<T>()
            .is_some_and(|storage| storage.remove(entity_id)))
    }

    pub fn get_component<T: Component>(&self, entity_id: EntityId) -> Option<&T> {
        if !self.alive.contains(&entity_id) {
            return None;
        }
        self.storage::<T>()?.get(entity_id)
    }

    pub fn get_component_mut<T: Component>(&mut self, entity_id: EntityId) -> Option<&mut T> {
        if !self.alive.contains(&entity_id) {
            return None;
        }
        self.storage_mut::<T>()?.get_mut(entity_id)
    }

    pub fn entity_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.alive.iter().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, EntityRef<'_>)> + '_ {
        self.alive
            .iter()
            .copied()
            .map(|id| (id, EntityRef { world: self, id }))
    }

    pub fn query_ids<T: Component>(&self) -> Vec<EntityId> {
        let mut output = Vec::new();
        self.query_ids_into::<T>(&mut output);
        output
    }

    pub fn query_ids_into<T: Component>(&self, output: &mut Vec<EntityId>) {
        output.clear();
        if let Some(storage) = self.storage::<T>() {
            output.extend_from_slice(&storage.entities);
        }
    }

    pub fn query_ids2<A: Component, B: Component>(&self) -> Vec<EntityId> {
        let mut output = Vec::new();
        self.query_ids2_into::<A, B>(&mut output);
        output
    }

    pub fn query_ids2_into<A: Component, B: Component>(&self, output: &mut Vec<EntityId>) {
        output.clear();
        let Some(a) = self.storage::<A>() else {
            return;
        };
        let Some(b) = self.storage::<B>() else {
            return;
        };

        let (small_entities, large_sparse) = if a.entities.len() <= b.entities.len() {
            (&a.entities, &b.sparse)
        } else {
            (&b.entities, &a.sparse)
        };
        output.reserve(small_entities.len());
        output.extend(
            small_entities
                .iter()
                .copied()
                .filter(|entity_id| large_sparse.contains_key(entity_id)),
        );
    }

    pub fn query_ids3<A: Component, B: Component, C: Component>(&self) -> Vec<EntityId> {
        let mut output = Vec::new();
        self.query_ids3_into::<A, B, C>(&mut output);
        output
    }

    pub fn query_ids3_into<A: Component, B: Component, C: Component>(
        &self,
        output: &mut Vec<EntityId>,
    ) {
        self.query_ids2_into::<A, B>(output);
        let Some(c) = self.storage::<C>() else {
            output.clear();
            return;
        };
        output.retain(|entity_id| c.sparse.contains_key(entity_id));
    }

    fn storage<T: Component>(&self) -> Option<&ComponentStorage<T>> {
        self.storages
            .get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref::<ComponentStorage<T>>()
    }

    fn storage_mut<T: Component>(&mut self) -> Option<&mut ComponentStorage<T>> {
        self.storages
            .get_mut(&TypeId::of::<T>())?
            .as_any_mut()
            .downcast_mut::<ComponentStorage<T>>()
    }

    fn storage_mut_or_insert<T: Component>(&mut self) -> &mut ComponentStorage<T> {
        self.storages
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::<ComponentStorage<T>>::default())
            .as_any_mut()
            .downcast_mut::<ComponentStorage<T>>()
            .expect("component storage type mismatch")
    }
}

#[cfg(test)]
mod tests {
    use crate::{System, SystemManager, World};

    #[derive(Debug, PartialEq)]
    struct Counter(i32);

    #[derive(Debug, PartialEq)]
    struct Marker;

    struct CounterSystem;

    impl System for CounterSystem {
        fn name(&self) -> &'static str {
            "counter"
        }

        fn update(&mut self, world: &mut World, _delta_seconds: f32) {
            for id in world.query_ids::<Counter>() {
                if let Some(counter) = world.get_component_mut::<Counter>(id) {
                    counter.0 += 1;
                }
            }
        }
    }

    #[test]
    fn sparse_components_and_systems_work() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add_component(entity, Counter(0)).unwrap();
        world.add_component(entity, Marker).unwrap();

        let mut systems = SystemManager::new();
        assert!(systems.add(CounterSystem));
        assert!(!systems.add(CounterSystem));
        systems.update(&mut world, 1.0 / 30.0);

        assert_eq!(world.get_component::<Counter>(entity), Some(&Counter(1)));
        assert_eq!(world.query_ids2::<Counter, Marker>(), vec![entity]);
        assert!(world.remove_component::<Counter>(entity).unwrap());
        assert_eq!(world.component_count::<Counter>(), 0);
    }

    #[test]
    fn despawn_removes_all_components() {
        let mut world = World::new();
        let first = world.spawn();
        let second = world.spawn();
        world.add_component(first, Counter(1)).unwrap();
        world.add_component(second, Counter(2)).unwrap();
        world.add_component(first, Marker).unwrap();

        assert_eq!(world.despawn(first), Some(first));
        assert!(world.get_component::<Counter>(first).is_none());
        assert!(world.get_component::<Marker>(first).is_none());
        assert_eq!(world.get_component::<Counter>(second), Some(&Counter(2)));
    }
}
