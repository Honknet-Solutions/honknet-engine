use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MobState {
    Alive,
    Critical,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobStateComponent {
    pub state: MobState,
}

impl Component for MobStateComponent {}

impl Default for MobStateComponent {
    fn default() -> Self {
        Self { state: MobState::Alive }
    }
}
