use std::{
    collections::HashMap,
    error::Error,
    fmt::{Display, Formatter},
};

use crate::{Component, Entity, EntityId};

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

#[derive(Default)]
pub struct World {
    next_entity_id: u64,
    entities: HashMap<EntityId, Entity>,
}

impl World {
    pub fn new() -> Self {
        Self {
            next_entity_id: 1,
            entities: HashMap::new(),
        }
    }

    pub fn spawn(&mut self) -> EntityId {
        let entity_id = EntityId::new(self.next_entity_id);
        self.next_entity_id = self
            .next_entity_id
            .checked_add(1)
            .expect("entity id space exhausted");
        self.entities.insert(entity_id, Entity::new(entity_id));
        entity_id
    }

    pub fn despawn(&mut self, entity_id: EntityId) -> Option<Entity> {
        self.entities.remove(&entity_id)
    }

    pub fn entity(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.get(&entity_id)
    }

    pub fn entity_mut(&mut self, entity_id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&entity_id)
    }

    pub fn add_component<T>(
        &mut self,
        entity_id: EntityId,
        component: T,
    ) -> Result<bool, WorldError>
    where
        T: Component,
    {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(WorldError::EntityNotFound(entity_id))?;
        Ok(entity.insert(component))
    }

    pub fn remove_component<T>(&mut self, entity_id: EntityId) -> Result<bool, WorldError>
    where
        T: Component,
    {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(WorldError::EntityNotFound(entity_id))?;
        Ok(entity.remove::<T>())
    }

    pub fn get_component<T>(&self, entity_id: EntityId) -> Option<&T>
    where
        T: Component,
    {
        self.entities.get(&entity_id)?.get::<T>()
    }

    pub fn get_component_mut<T>(&mut self, entity_id: EntityId) -> Option<&mut T>
    where
        T: Component,
    {
        self.entities.get_mut(&entity_id)?.get_mut::<T>()
    }

    pub fn entity_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.keys().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &Entity)> + '_ {
        self.entities
            .iter()
            .map(|(entity_id, entity)| (*entity_id, entity))
    }
}

#[cfg(test)]
mod tests {
    use crate::{System, SystemManager, World};

    #[derive(Debug, PartialEq)]
    struct Counter(i32);

    struct CounterSystem;

    impl System for CounterSystem {
        fn name(&self) -> &'static str {
            "counter"
        }

        fn update(&mut self, world: &mut World, _delta_seconds: f32) {
            let ids = world.entity_ids().collect::<Vec<_>>();
            for id in ids {
                if let Some(counter) = world.get_component_mut::<Counter>(id) {
                    counter.0 += 1;
                }
            }
        }
    }

    #[test]
    fn ecs_components_and_systems_work() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add_component(entity, Counter(0)).unwrap();

        let mut systems = SystemManager::new();
        assert!(systems.add(CounterSystem));
        assert!(!systems.add(CounterSystem));
        systems.update(&mut world, 1.0 / 30.0);

        assert_eq!(world.get_component::<Counter>(entity), Some(&Counter(1)));
        assert!(world.remove_component::<Counter>(entity).unwrap());
    }
}
