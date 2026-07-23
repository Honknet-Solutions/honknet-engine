use crate::components::hands::{HandsComponent, ItemComponent};
use crate::components::inventory::{EquipmentSlot, InventoryComponent};
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn equip_item(world: &mut World, user: Entity, slot: EquipmentSlot) -> bool {
    let item_in_hand = if let Some(hands) = world.get_mut::<HandsComponent>(user) {
        hands.item_in_hand.take()
    } else {
        None
    };

    if let Some(item) = item_in_hand {
        if let Some(inv) = world.get_mut::<InventoryComponent>(user) {
            if inv.slots.get(&slot) == Some(&None) {
                inv.slots.insert(slot, Some(item));
                if let Some(item_comp) = world.get_mut::<ItemComponent>(item) {
                    item_comp.in_container = Some(user);
                }
                info!("Equipped item {:?} to slot {:?} for user {:?}", item, slot, user);
                return true;
            }
        }
        // Return item to hand if equipping failed
        if let Some(hands) = world.get_mut::<HandsComponent>(user) {
            hands.item_in_hand = Some(item);
        }
    }
    false
}
