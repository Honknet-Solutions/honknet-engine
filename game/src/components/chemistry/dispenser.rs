use crate::components::chemistry::reagent::ReagentId;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemDispenserComponent {
    pub available_reagents: Vec<ReagentId>,
    pub dispense_amount: f32,
}

impl Component for ChemDispenserComponent {}

impl Default for ChemDispenserComponent {
    fn default() -> Self {
        Self {
            available_reagents: vec![
                ReagentId::Water,
                ReagentId::Bicaridine,
                ReagentId::Kelotane,
                ReagentId::Dylovene,
                ReagentId::Inaprovline,
            ],
            dispense_amount: 10.0,
        }
    }
}
