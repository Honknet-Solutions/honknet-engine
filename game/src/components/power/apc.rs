use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApcComponent {
    pub cell_charge: f32,
    pub cell_capacity: f32,
    pub equipment_powered: bool,
    pub lighting_powered: bool,
    pub environment_powered: bool,
}

impl Component for ApcComponent {}

impl Default for ApcComponent {
    fn default() -> Self {
        Self {
            cell_charge: 50000.0,
            cell_capacity: 100000.0,
            equipment_powered: true,
            lighting_powered: true,
            environment_powered: true,
        }
    }
}
