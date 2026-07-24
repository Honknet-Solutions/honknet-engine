use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthComponent {
    pub current: f32,
    pub max: f32,
}

impl Component for HealthComponent {}

impl Default for HealthComponent {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
        }
    }
}
