use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(u64);

impl EntityId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

pub trait Component: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> Component for T
where
    T: Any + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct Entity {
    id: EntityId,
    components: HashMap<TypeId, Box<dyn Component>>,
}

impl Entity {
    pub(crate) fn new(id: EntityId) -> Self {
        Self {
            id,
            components: HashMap::new(),
        }
    }

    pub const fn id(&self) -> EntityId {
        self.id
    }

    pub fn insert<T>(&mut self, component: T) -> bool
    where
        T: Component,
    {
        self.components
            .insert(TypeId::of::<T>(), Box::new(component))
            .is_some()
    }

    pub fn remove<T>(&mut self) -> bool
    where
        T: Component,
    {
        self.components.remove(&TypeId::of::<T>()).is_some()
    }

    pub fn contains<T>(&self) -> bool
    where
        T: Component,
    {
        self.components.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T>(&self) -> Option<&T>
    where
        T: Component,
    {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|component| component.as_ref().as_any().downcast_ref::<T>())
    }

    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Component,
    {
        self.components
            .get_mut(&TypeId::of::<T>())
            .and_then(|component| component.as_mut().as_any_mut().downcast_mut::<T>())
    }
}
