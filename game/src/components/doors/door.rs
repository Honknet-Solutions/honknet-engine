use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorState {
    Closed,
    Opening,
    Open,
    Closing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorComponent {
    pub state: DoorState,
    pub auto_close: bool,
    pub auto_close_delay: f32,
    pub timer: f32,
}

impl Component for DoorComponent {}

impl Default for DoorComponent {
    fn default() -> Self {
        Self {
            state: DoorState::Closed,
            auto_close: true,
            auto_close_delay: 3.0,
            timer: 0.0,
        }
    }
}
