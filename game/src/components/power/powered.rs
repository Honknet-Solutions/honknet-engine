use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoweredComponent {
    pub requires_power: bool,
    pub is_powered: bool,
    pub idle_power_draw: f32,
}

impl Component for PoweredComponent {}

impl Default for PoweredComponent {
    fn default() -> Self {
        Self {
            requires_power: true,
            is_powered: true,
            idle_power_draw: 50.0,
        }
    }
}
