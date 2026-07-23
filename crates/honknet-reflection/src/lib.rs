use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
#[derive(Debug, Clone)]
pub struct FieldDescriptor {
    pub name: &'static str,
    pub type_name: &'static str,
    pub networked: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TypeDescriptor {
    pub name: &'static str,
    pub type_id: TypeId,
    pub fields: &'static [FieldDescriptor],
    pub version: u32,
}

pub trait Reflect: Any + Send + Sync {
    fn descriptor() -> TypeDescriptor
    where
        Self: Sized;
    fn as_any(&self) -> &dyn Any;
}

pub trait ReflectSerialize: Reflect + Serialize + DeserializeOwned {}

impl<T> ReflectSerialize for T where T: Reflect + Serialize + DeserializeOwned {}

#[derive(Default, Clone)]
pub struct TypeRegistry {
    inner: Arc<RwLock<HashMap<TypeId, TypeDescriptor>>>,
}

impl TypeRegistry {
    pub fn register<T: Reflect>(&self) {
        self.inner
            .write()
            .insert(TypeId::of::<T>(), T::descriptor());
    }
    pub fn descriptor<T: 'static>(&self) -> Option<TypeDescriptor> {
        self.inner.read().get(&TypeId::of::<T>()).cloned()
    }
    pub fn all(&self) -> Vec<TypeDescriptor> {
        self.inner.read().values().cloned().collect()
    }
}
