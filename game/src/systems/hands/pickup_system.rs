use crate::components::hands::{HandsComponent, ItemComponent};
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn pick_up_item(world: &mut World, user: Entity, item: Entity) -> bool {
    if user == item
        || !world.is_alive(user)
        || !world.is_alive(item)
        || world
            .get::<ItemComponent>(item)
            .is_none_or(|component| component.in_container.is_some())
    {
        return false;
    }
    if let Some(hands) = world.get_mut::<HandsComponent>(user) {
        if hands.item_in_hand.is_none() {
            hands.item_in_hand = Some(item);
            world.get_mut::<ItemComponent>(item).unwrap().in_container = Some(user);
            info!("User {:?} picked up item {:?}", user, item);
            return true;
        }
    }
    false
}
