use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DoorBoltComponent {
    pub is_bolted: bool,
}

impl Component for DoorBoltComponent {}
