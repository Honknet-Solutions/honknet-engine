use std::collections::HashSet;

use crate::World;

pub trait System: Send + Sync {
    fn name(&self) -> &'static str;
    fn update(&mut self, world: &mut World, delta_seconds: f32);
}

#[derive(Default)]
pub struct SystemManager {
    systems: Vec<Box<dyn System>>,
    names: HashSet<&'static str>,
}

impl SystemManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<S>(&mut self, system: S) -> bool
    where
        S: System + 'static,
    {
        if !self.names.insert(system.name()) {
            return false;
        }

        self.systems.push(Box::new(system));
        true
    }

    pub fn update(&mut self, world: &mut World, delta_seconds: f32) {
        for system in &mut self.systems {
            system.update(world, delta_seconds);
        }
    }
}
