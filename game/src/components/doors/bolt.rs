use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorBoltComponent {
    pub is_bolted: bool,
}

impl Component for DoorBoltComponent {}

impl Default for DoorBoltComponent {
    fn default() -> Self {
        Self { is_bolted: false }
    }
}
