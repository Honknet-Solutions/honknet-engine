use honknet_core::Entity;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtmosConnectionComponent {
    pub first: Entity,
    pub second: Entity,
    pub conductance: f32,
    pub barrier: Option<Entity>,
}

impl Component for AtmosConnectionComponent {}

impl Default for AtmosConnectionComponent {
    fn default() -> Self {
        Self {
            first: Entity::new(0, 0),
            second: Entity::new(0, 0),
            conductance: 0.25,
            barrier: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtmosZoneComponent {
    pub members: Vec<Entity>,
    pub revision: u64,
}

impl Component for AtmosZoneComponent {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BreathingEnvironmentComponent {
    pub atmosphere: Entity,
}

impl Component for BreathingEnvironmentComponent {}
