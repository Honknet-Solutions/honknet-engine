use crate::components::inventory::slots::EquipmentSlot;
use honknet_core::Entity;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryComponent {
    pub slots: HashMap<EquipmentSlot, Option<Entity>>,
}

impl Component for InventoryComponent {}

impl Default for InventoryComponent {
    fn default() -> Self {
        let mut slots = HashMap::new();
        slots.insert(EquipmentSlot::Head, None);
        slots.insert(EquipmentSlot::Mask, None);
        slots.insert(EquipmentSlot::Jumpsuit, None);
        slots.insert(EquipmentSlot::OuterClothing, None);
        slots.insert(EquipmentSlot::Gloves, None);
        slots.insert(EquipmentSlot::Shoes, None);
        slots.insert(EquipmentSlot::Belt, None);
        slots.insert(EquipmentSlot::PocketLeft, None);
        slots.insert(EquipmentSlot::PocketRight, None);
        slots.insert(EquipmentSlot::IdCard, None);
        Self { slots }
    }
}
