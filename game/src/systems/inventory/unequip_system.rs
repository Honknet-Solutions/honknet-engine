use crate::components::hands::HandsComponent;
use crate::components::inventory::{EquipmentSlot, InventoryComponent};
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn unequip_item(world: &mut World, user: Entity, slot: EquipmentSlot) -> Option<Entity> {
    let has_free_hand = world
        .get::<HandsComponent>(user)
        .map(|h| h.item_in_hand.is_none())
        .unwrap_or(false);

    if has_free_hand {
        if let Some(inv) = world.get_mut::<InventoryComponent>(user) {
            if let Some(Some(item)) = inv.slots.get_mut(&slot).map(|s| s.take()) {
                if let Some(hands) = world.get_mut::<HandsComponent>(user) {
                    hands.item_in_hand = Some(item);
                }
                info!(
                    "Unequipped item {:?} from slot {:?} to hand for user {:?}",
                    item, slot, user
                );
                return Some(item);
            }
        }
    }
    None
}
