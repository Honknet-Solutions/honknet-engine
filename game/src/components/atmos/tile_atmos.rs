use crate::components::atmos::gas::GasMix;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileAtmosphereComponent {
    pub air: GasMix,
    pub volume: f32,
    pub is_space: bool,
}

impl Component for TileAtmosphereComponent {}

impl Default for TileAtmosphereComponent {
    fn default() -> Self {
        Self {
            air: GasMix::default(),
            volume: 2500.0, // Standard turf volume
            is_space: false,
        }
    }
}
