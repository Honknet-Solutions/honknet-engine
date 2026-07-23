use crate::components::atmos::gas::GasMix;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipeType {
    Straight,
    Manifold,
    Pump,
    Vent,
    Filter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeComponent {
    pub pipe_type: PipeType,
    pub internal_air: GasMix,
    pub max_pressure: f32,
}

impl Component for PipeComponent {}

impl Default for PipeComponent {
    fn default() -> Self {
        Self {
            pipe_type: PipeType::Straight,
            internal_air: GasMix::default(),
            max_pressure: 4500.0, // 4500 kPa max
        }
    }
}
