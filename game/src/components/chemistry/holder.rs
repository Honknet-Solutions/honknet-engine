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
