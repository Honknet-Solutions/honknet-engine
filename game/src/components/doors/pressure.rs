use honknet_core::Entity;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DoorPressureComponent {
    pub first_atmosphere: Entity,
    pub second_atmosphere: Entity,
    pub maximum_safe_delta_kpa: f32,
}

impl Component for DoorPressureComponent {}
