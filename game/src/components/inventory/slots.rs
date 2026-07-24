use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Head,
    Mask,
    Jumpsuit,
    OuterClothing,
    Gloves,
    Shoes,
    Belt,
    PocketLeft,
    PocketRight,
    IdCard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WearableComponent {
    pub allowed_slots: Vec<EquipmentSlot>,
}

impl Component for WearableComponent {}
