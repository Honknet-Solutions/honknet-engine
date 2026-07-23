use honknet_ecs::World;
pub mod prelude {
    pub use honknet_core::{CVarRegistry, CVarValue, EngineClock, Entity, Tick};
    pub use honknet_ecs::{CommandBuffer, Component, Resources, StorageKind, World};
    pub use honknet_events::{Delivery, EventBus};
    pub use honknet_macros::{Component, NetworkMessage, Reflect};
    pub use honknet_map::{Chunk, Grid, Map, TileDef};
    pub use honknet_math::{Aabb, Transform2, Vec2};
    pub use honknet_net_core::{Channel, NetworkMessage};
    pub use honknet_physics::{Body, BodyType, Fixture, PhysicsWorld, Shape};
    pub use honknet_replication::{ComponentState, EntityState, ReplicationMode, Replicator};
    pub use honknet_ui::{Layout, Node, Style, UiTree, Widget};
    pub use serde::{Deserialize, Serialize};
}

pub trait GamePlugin: Send {
    fn name(&self) -> &'static str;
    fn configure(&mut self, _world: &mut World) {}
    fn startup(&mut self, _world: &mut World) {}
    fn shutdown(&mut self, _world: &mut World) {}
}

pub trait GameModule: Send + Sync {
    fn name(&self) -> &'static str;
    fn register_components(&self, _world: &mut World) {}
    fn register_prototypes(&self, _manager: &honknet_prototypes::PrototypeManager) {}
    fn initialize_server(&self, _world: &mut World) {}
}

#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<Box<dyn GamePlugin>>,
}

impl PluginManager {
    pub fn add<P: GamePlugin + 'static>(&mut self, p: P) {
        self.plugins.push(Box::new(p))
    }
    pub fn configure(&mut self, w: &mut World) {
        for p in &mut self.plugins {
            p.configure(w)
        }
    }
    pub fn startup(&mut self, w: &mut World) {
        for p in &mut self.plugins {
            p.startup(w)
        }
    }
    pub fn shutdown(&mut self, w: &mut World) {
        for p in self.plugins.iter_mut().rev() {
            p.shutdown(w)
        }
    }
}
