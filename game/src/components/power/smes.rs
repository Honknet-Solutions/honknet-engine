use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmesComponent {
    pub charge: f32,
    pub max_capacity: f32,
    pub input_rate: f32,
    pub output_rate: f32,
}

impl Component for SmesComponent {}

impl Default for SmesComponent {
    fn default() -> Self {
        Self {
            charge: 500000.0,
            max_capacity: 1000000.0,
            input_rate: 20000.0,
            output_rate: 20000.0,
        }
    }
}
