use honknet_core::Entity;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrabStrength {
    Passive,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrabComponent {
    pub target: Entity,
    pub strength: GrabStrength,
}

impl Component for GrabComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullingComponent {
    pub target: Entity,
    pub maximum_distance: f32,
}

impl Component for PullingComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarryingComponent {
    pub target: Entity,
}

impl Component for CarryingComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarriedComponent {
    pub carrier: Entity,
}

impl Component for CarriedComponent {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuckledComponent {
    pub fixture: Option<Entity>,
}

impl Component for BuckledComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuckleFixtureComponent {
    pub capacity: u8,
    pub occupants: Vec<Entity>,
}

impl Default for BuckleFixtureComponent {
    fn default() -> Self {
        Self {
            capacity: 1,
            occupants: Vec::new(),
        }
    }
}

impl Component for BuckleFixtureComponent {}
