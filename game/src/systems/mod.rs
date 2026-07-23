use crate::components::{
    ContainerComponent, DoorComponent, DoorState, ExamineComponent, HandsComponent,
    HealthComponent, InteractionComponent, ItemComponent, MobState, MobStateComponent,
};
use honknet_core::Entity;
use honknet_ecs::World;
use honknet_math::Vec2;
use tracing::info;

pub fn door_system(world: &mut World, delta_time: f32) {
    let entities = world.query::<DoorComponent>();
    let mut to_close = Vec::new();

    for e in entities {
        if let Some(door) = world.get_mut::<DoorComponent>(e) {
            if door.state == DoorState::Open && door.auto_close {
                door.timer += delta_time;
                if door.timer >= door.auto_close_delay {
                    to_close.push(e);
                }
            }
        }
    }

    for e in to_close {
        if let Some(door) = world.get_mut::<DoorComponent>(e) {
            door.state = DoorState::Closed;
            door.timer = 0.0;
            info!("Door {:?} auto-closed", e);
        }
    }
}

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
    if let Some(door) = world.get_mut::<DoorComponent>(target) {
        door.state = match door.state {
            DoorState::Closed => DoorState::Open,
            DoorState::Open => DoorState::Closed,
            DoorState::Opening => DoorState::Open,
            DoorState::Closing => DoorState::Closed,
        };
        door.timer = 0.0;
        info!("Entity {:?} toggled door {:?} to state {:?}", user, target, door.state);
        return true;
    }

    // Pickup Item if target has ItemComponent and user has HandsComponent
    if world.contains::<ItemComponent>(target) && world.contains::<HandsComponent>(user) {
        return pick_up_item(world, user, target);
    }

    false
}

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

pub fn examine_system(world: &World, target: Entity) -> Option<String> {
    if let Some(examine) = world.get::<ExamineComponent>(target) {
        let desc = format!("Examine {:?}: {}\nInfo: {}", target, examine.description, examine.detailed_info);
        info!("{}", desc);
        return Some(desc);
    }
    None
}

pub fn container_system(world: &mut World, container_entity: Entity, item: Entity) -> bool {
    if let Some(container) = world.get_mut::<ContainerComponent>(container_entity) {
        if (container.contents.len() as u32) < container.capacity {
            container.contents.push(item);
            if let Some(item_comp) = world.get_mut::<ItemComponent>(item) {
                item_comp.in_container = Some(container_entity);
            }
            info!("Item {:?} stored in container {:?}", item, container_entity);
            return true;
        }
    }
    false
}

pub fn health_system(world: &mut World) {
    let entities = world.query::<HealthComponent>();

    for e in entities {
        if let Some(health) = world.get_mut::<HealthComponent>(e) {
            if health.current <= 0.0 {
                if let Some(mob_state) = world.get_mut::<MobStateComponent>(e) {
                    if mob_state.state != MobState::Dead {
                        mob_state.state = MobState::Dead;
                        info!("Entity {:?} died", e);
                    }
                }
            } else if health.current <= 20.0 {
                if let Some(mob_state) = world.get_mut::<MobStateComponent>(e) {
                    if mob_state.state == MobState::Alive {
                        mob_state.state = MobState::Critical;
                        info!("Entity {:?} entered critical state", e);
                    }
                }
            }
        }
    }
}
