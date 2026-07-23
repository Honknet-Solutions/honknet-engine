use crate::components::doors::{DoorBoltComponent, DoorComponent, DoorState};
use crate::components::hands::{HandsComponent, ItemComponent};
use crate::components::interaction::InteractionComponent;
use crate::systems::access::check_user_access;
use crate::systems::hands::pick_up_item;
use honknet_core::Entity;
use honknet_ecs::World;
use honknet_math::Vec2;
use tracing::info;

pub fn interaction_system(
    world: &mut World,
    user: Entity,
    target: Entity,
    user_pos: Vec2,
    target_pos: Vec2,
) -> bool {
    let reach = world
        .get::<InteractionComponent>(user)
        .map(|i| i.reach_distance)
        .unwrap_or(2.5);

    if (user_pos - target_pos).length() > reach {
        info!("Entity {:?} is too far to interact with {:?}", user, target);
        return false;
    }

    // Toggle Door if target is a Door
    if let Some(door) = world.get::<DoorComponent>(target).cloned() {
        // 1. Check if door is bolted
        if let Some(bolt) = world.get::<DoorBoltComponent>(target) {
            if bolt.is_bolted {
                info!("Door {:?} is bolted shut and cannot be opened!", target);
                return false;
            }
        }

        // 2. Check ID Card access requirements
        if !check_user_access(world, user, target) {
            info!("Access Denied: User {:?} lacks required ID card access for door {:?}", user, target);
            return false;
        }

        if let Some(door_mut) = world.get_mut::<DoorComponent>(target) {
            door_mut.state = match door.state {
                DoorState::Closed => DoorState::Open,
                DoorState::Open => DoorState::Closed,
                DoorState::Opening => DoorState::Open,
                DoorState::Closing => DoorState::Closed,
            };
            door_mut.timer = 0.0;
            info!("Entity {:?} toggled door {:?} to state {:?}", user, target, door_mut.state);
            return true;
        }
    }

    // Pickup Item if target has ItemComponent and user has HandsComponent
    if world.contains::<ItemComponent>(target) && world.contains::<HandsComponent>(user) {
        return pick_up_item(world, user, target);
    }

    false
}
