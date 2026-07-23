use honknet_core::Entity;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

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
