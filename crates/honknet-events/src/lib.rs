use honknet_core::Entity;
use parking_lot::Mutex;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Broadcast,
    Entity(Entity),
}

pub struct EventEnvelope {
    pub delivery: Delivery,
    pub predicted: bool,
    pub cancelled: bool,
    payload: Box<dyn Any + Send>,
}

impl EventEnvelope {
    pub fn new<T: Any + Send>(delivery: Delivery, predicted: bool, value: T) -> Self {
        Self {
            delivery,
            predicted,
            cancelled: false,
            payload: Box::new(value),
        }
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.payload.downcast_ref()
    }
}

#[derive(Default, Clone)]
pub struct EventBus {
    queues: Arc<Mutex<HashMap<TypeId, Vec<EventEnvelope>>>>,
}

impl EventBus {
    pub fn send<T: Any + Send>(&self, delivery: Delivery, predicted: bool, value: T) {
        self.queues
            .lock()
            .entry(TypeId::of::<T>())
            .or_default()
            .push(EventEnvelope::new(delivery, predicted, value));
    }
    pub fn drain<T: Any + Send>(&self) -> Vec<EventEnvelope> {
        self.queues
            .lock()
            .remove(&TypeId::of::<T>())
            .unwrap_or_default()
    }
    pub fn len<T: Any>(&self) -> usize {
        self.queues
            .lock()
            .get(&TypeId::of::<T>())
            .map_or(0, Vec::len)
    }
}
