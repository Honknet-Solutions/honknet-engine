use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use honknet_core::{EntityId, System, Transform, World};

use crate::{
    components::{ColliderComponent, DoorComponent, PlayerInputComponent},
    game_map::GameMap,
};

pub struct InputTimeoutSystem {
    timeout: Duration,
}

impl InputTimeoutSystem {
    pub const fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl System for InputTimeoutSystem {
    fn name(&self) -> &'static str {
        "input_timeout"
    }

    fn update(&mut self, world: &mut World, _delta_seconds: f32) {
        let now = Instant::now();
        let ids = world.entity_ids().collect::<Vec<_>>();

        for id in ids {
            let Some(input) = world.get_component_mut::<PlayerInputComponent>(id) else {
                continue;
            };

            let Some(last_received_at) = input.last_received_at else {
                continue;
            };

            if now.duration_since(last_received_at) >= self.timeout {
                input.stop();
            }
        }
    }
}

pub struct MovementSystem {
    speed: f32,
    map: Arc<GameMap>,
}

impl MovementSystem {
    pub fn new(speed: f32, map: Arc<GameMap>) -> Self {
        Self { speed, map }
    }
}

impl System for MovementSystem {
    fn name(&self) -> &'static str {
        "movement"
    }

    fn update(&mut self, world: &mut World, delta_seconds: f32) {
        let ids = world.entity_ids().collect::<Vec<_>>();

        for entity_id in ids {
            let command = {
                let Some(input) = world.get_component_mut::<PlayerInputComponent>(entity_id) else {
                    continue;
                };

                input.pop_next()
            };

            let Some(command) = command else {
                continue;
            };

            let Some(radius) = world
                .get_component::<ColliderComponent>(entity_id)
                .map(|collider| collider.radius)
            else {
                continue;
            };

            let Some(position) = world
                .get_component::<Transform>(entity_id)
                .map(|transform| (transform.position.x, transform.position.y, transform.z))
            else {
                continue;
            };

            let distance = self.speed * delta_seconds;
            let mut next_x = position.0;
            let mut next_y = position.1;

            let candidate_x = next_x + command.movement.x * distance;
            if !is_blocked(
                world,
                &self.map,
                entity_id,
                candidate_x,
                next_y,
                position.2,
                radius,
            ) {
                next_x = candidate_x;
            }

            let candidate_y = next_y + command.movement.y * distance;
            if !is_blocked(
                world,
                &self.map,
                entity_id,
                next_x,
                candidate_y,
                position.2,
                radius,
            ) {
                next_y = candidate_y;
            }

            if let Some(transform) = world.get_component_mut::<Transform>(entity_id) {
                transform.position.x = next_x;
                transform.position.y = next_y;
            }
        }
    }
}

fn is_blocked(
    world: &World,
    map: &GameMap,
    moving_entity: EntityId,
    x: f32,
    y: f32,
    z: i32,
    radius: f32,
) -> bool {
    if map.circle_collides(x, y, radius) {
        return true;
    }

    for (entity_id, entity) in world.iter() {
        if entity_id == moving_entity {
            continue;
        }

        let Some(door) = entity.get::<DoorComponent>() else {
            continue;
        };

        if door.open {
            continue;
        }

        let Some(transform) = entity.get::<Transform>() else {
            continue;
        };

        if transform.z != z {
            continue;
        }

        let nearest_x = x.clamp(transform.position.x - 0.45, transform.position.x + 0.45);
        let nearest_y = y.clamp(transform.position.y - 0.45, transform.position.y + 0.45);
        let dx = x - nearest_x;
        let dy = y - nearest_y;

        if dx * dx + dy * dy < radius * radius {
            return true;
        }
    }

    false
}
