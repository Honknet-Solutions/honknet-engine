use honknet_core::Entity;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemComponent {
    pub weight: f32,
    pub size: u32,
    pub in_container: Option<Entity>,
}

impl Component for ItemComponent {}

impl Default for ItemComponent {
    fn default() -> Self {
        Self {
            weight: 1.0,
            size: 1,
            in_container: None,
        }
    }
}
