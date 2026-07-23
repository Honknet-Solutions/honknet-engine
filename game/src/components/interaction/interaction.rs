use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionComponent {
    pub reach_distance: f32,
}

impl Component for InteractionComponent {}

impl Default for InteractionComponent {
    fn default() -> Self {
        Self { reach_distance: 2.5 }
    }
}
