use honknet_core::Entity;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerComponent {
    pub capacity: u32,
    pub contents: Vec<Entity>,
}

impl Component for ContainerComponent {}

impl Default for ContainerComponent {
    fn default() -> Self {
        Self {
            capacity: 10,
            contents: Vec::new(),
        }
    }
}
