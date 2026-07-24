use crate::components::chemistry::reagent::ReagentVolume;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReagentHolderComponent {
    pub max_volume: f32,
    pub reagents: Vec<ReagentVolume>,
}

impl Component for ReagentHolderComponent {}

impl Default for ReagentHolderComponent {
    fn default() -> Self {
        Self {
            max_volume: 50.0, // 50uL standard beaker
            reagents: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolismComponent {
    pub rate: f32,
    pub toxin_load: f32,
    pub stabilization: f32,
}

impl Component for MetabolismComponent {}

impl Default for MetabolismComponent {
    fn default() -> Self {
        Self {
            rate: 0.5,
            toxin_load: 0.0,
            stabilization: 0.0,
        }
    }
}
