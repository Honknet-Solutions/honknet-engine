use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CableType {
    HighVoltage,   // Red HV
    MediumVoltage, // Yellow MV
    LowVoltage,    // Green LV
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CableComponent {
    pub cable_type: CableType,
}

impl Component for CableComponent {}

impl Default for CableComponent {
    fn default() -> Self {
        Self {
            cable_type: CableType::MediumVoltage,
        }
    }
}
