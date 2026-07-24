use crate::components::access::{AccessReaderComponent, IdCardComponent};
use crate::components::hands::HandsComponent;
use crate::components::inventory::{EquipmentSlot, InventoryComponent};
use honknet_core::Entity;
use honknet_ecs::World;

pub fn check_user_access(world: &World, user: Entity, target: Entity) -> bool {
    let required_tags = match world.get::<AccessReaderComponent>(target) {
        Some(reader) if !reader.required_tags.is_empty() => &reader.required_tags,
        _ => return true, // No access restrictions
    };

    // Check ID card in user's equipped IdCard slot or held item
    let mut id_card_entity = None;
    if let Some(inv) = world.get::<InventoryComponent>(user) {
        if let Some(Some(card)) = inv.slots.get(&EquipmentSlot::IdCard) {
            id_card_entity = Some(*card);
        }
    }
    if id_card_entity.is_none() {
        if let Some(hands) = world.get::<HandsComponent>(user) {
            if let Some(held) = hands.item_in_hand {
                if world.contains::<IdCardComponent>(held) {
                    id_card_entity = Some(held);
                }
            }
        }
    }

    if let Some(card_e) = id_card_entity {
        if let Some(card) = world.get::<IdCardComponent>(card_e) {
            return required_tags
                .iter()
                .any(|tag| card.access_tags.contains(tag));
        }
    }

    false
}
