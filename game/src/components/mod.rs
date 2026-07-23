use honknet_core::Entity;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandsComponent {
    pub active_hand_index: u8,
    pub item_in_hand: Option<Entity>,
    pub max_hands: u8,
}

impl Component for HandsComponent {}

impl Default for HandsComponent {
    fn default() -> Self {
        Self {
            active_hand_index: 0,
            item_in_hand: None,
            max_hands: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemComponent {
    pub weight: f32,
    pub size: u32,
    pub in_container: Option<Entity>,
}

impl Component for ItemComponent {}

impl Default for ItemComponent {
    fn default() -> Self {
        Self {
            weight: 1.0,
            size: 1,
            in_container: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionComponent {
    pub reach_distance: f32,
}

impl Component for InteractionComponent {}

impl Default for InteractionComponent {
    fn default() -> Self {
        Self { reach_distance: 2.5 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamineComponent {
    pub description: String,
    pub detailed_info: String,
}

impl Component for ExamineComponent {}

impl Default for ExamineComponent {
    fn default() -> Self {
        Self {
            description: "An object on Space Station 15.".to_string(),
            detailed_info: "It looks sturdy.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerComponent {
    pub capacity: u32,
    pub contents: Vec<Entity>,
}

impl Component for ContainerComponent {}

impl Default for ContainerComponent {
    fn default() -> Self {
        Self {
            capacity: 10,
            contents: Vec::new(),
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthComponent {
    pub current: f32,
    pub max: f32,
}

impl Component for HealthComponent {}

impl Default for HealthComponent {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
        }
    }
}
