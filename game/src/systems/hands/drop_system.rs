use crate::components::hands::{HandsComponent, ItemComponent};
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn drop_item(world: &mut World, user: Entity) -> Option<Entity> {
    if let Some(hands) = world.get_mut::<HandsComponent>(user) {
        if let Some(item) = hands.item_in_hand.take() {
            if let Some(item_comp) = world.get_mut::<ItemComponent>(item) {
                item_comp.in_container = None;
            }
            info!("User {:?} dropped item {:?}", user, item);
            return Some(item);
        }
    }
    None
}
