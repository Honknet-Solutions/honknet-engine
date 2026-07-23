use crate::components::hands::{HandsComponent, ItemComponent};
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn pick_up_item(world: &mut World, user: Entity, item: Entity) -> bool {
    if let Some(hands) = world.get_mut::<HandsComponent>(user) {
        if hands.item_in_hand.is_none() {
            hands.item_in_hand = Some(item);
            if let Some(item_comp) = world.get_mut::<ItemComponent>(item) {
                item_comp.in_container = Some(user);
            }
            info!("User {:?} picked up item {:?}", user, item);
            return true;
        }
    }
    false
}
