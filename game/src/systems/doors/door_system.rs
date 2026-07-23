use crate::components::doors::{DoorComponent, DoorState};
use honknet_ecs::World;
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
