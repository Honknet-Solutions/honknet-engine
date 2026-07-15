use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use honknet_core::{EntityId, SpatialHash, System, Transform, World};

use crate::{
    components::{ColliderComponent, DoorComponent, PlayerInputComponent},
    game_map::GameMap,
};

pub struct InputTimeoutSystem {
    timeout: Duration,
    scratch: Vec<EntityId>,
}

impl InputTimeoutSystem {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            scratch: Vec::new(),
        }
    }
}

impl System for InputTimeoutSystem {
    fn name(&self) -> &'static str {
        "input_timeout"
    }

    fn update(&mut self, world: &mut World, _delta_seconds: f32) {
        let now = Instant::now();
        world.query_ids_into::<PlayerInputComponent>(&mut self.scratch);
        for &id in &self.scratch {
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
    spatial: Arc<RwLock<SpatialHash>>,
    scratch: Vec<EntityId>,
}

impl MovementSystem {
    pub fn new(speed: f32, map: Arc<GameMap>, spatial: Arc<RwLock<SpatialHash>>) -> Self {
        Self {
            speed,
            map,
            spatial,
            scratch: Vec::new(),
        }
    }
}

impl System for MovementSystem {
    fn name(&self) -> &'static str {
        "movement"
    }

    fn update(&mut self, world: &mut World, delta_seconds: f32) {
        world.query_ids3_into::<PlayerInputComponent, ColliderComponent, Transform>(
            &mut self.scratch,
        );

        for &entity_id in &self.scratch {
            let command = {
                let Some(input) = world.get_component_mut::<PlayerInputComponent>(entity_id) else {
                    continue;
                };
                input.pop_next()
            };
            let Some(command) = command else {
                continue;
            };

            let Some(collider) = world.get_component::<ColliderComponent>(entity_id).copied()
            else {
                continue;
            };
            let Some(transform) = world.get_component::<Transform>(entity_id).cloned() else {
                continue;
            };

            let distance = self.speed * delta_seconds;
            let mut next_x = transform.position.x;
            let mut next_y = transform.position.y;

            let candidate_x = next_x + command.movement.x * distance;
            let blocked_x = {
                let spatial = self
                    .spatial
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                is_blocked(
                    world,
                    &self.map,
                    &spatial,
                    entity_id,
                    &transform,
                    &collider,
                    candidate_x,
                    next_y,
                )
            };
            if !blocked_x {
                next_x = candidate_x;
            }

            let candidate_y = next_y + command.movement.y * distance;
            let blocked_y = {
                let spatial = self
                    .spatial
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                is_blocked(
                    world,
                    &self.map,
                    &spatial,
                    entity_id,
                    &transform,
                    &collider,
                    next_x,
                    candidate_y,
                )
            };
            if !blocked_y {
                next_y = candidate_y;
            }

            if let Some(current) = world.get_component_mut::<Transform>(entity_id) {
                current.position.x = next_x;
                current.position.y = next_y;
            }
            self.spatial
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert_circle(
                    entity_id,
                    self.map.map_hash,
                    transform.z,
                    next_x,
                    next_y,
                    collider.radius,
                );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn is_blocked(
    world: &World,
    map: &GameMap,
    spatial: &SpatialHash,
    moving_entity: EntityId,
    transform: &Transform,
    collider: &ColliderComponent,
    x: f32,
    y: f32,
) -> bool {
    if map.circle_collides(transform.grid_id.as_deref(), x, y, collider.radius) {
        return true;
    }

    for other_id in spatial.query_circle(map.map_hash, transform.z, x, y, collider.radius + 1.0) {
        if other_id == moving_entity {
            continue;
        }
        let Some(other_collider) = world.get_component::<ColliderComponent>(other_id) else {
            continue;
        };
        if collider.sensor
            || other_collider.sensor
            || collider.collision_mask & other_collider.collision_layer == 0
            || other_collider.collision_mask & collider.collision_layer == 0
        {
            continue;
        }
        if world
            .get_component::<DoorComponent>(other_id)
            .is_some_and(|door| door.open)
        {
            continue;
        }
        let Some(other_transform) = world.get_component::<Transform>(other_id) else {
            continue;
        };
        if other_transform.map_id != transform.map_id || other_transform.z != transform.z {
            continue;
        }
        let dx = x - other_transform.position.x;
        let dy = y - other_transform.position.y;
        let combined = collider.radius + other_collider.radius;
        if dx * dx + dy * dy < combined * combined {
            return true;
        }
    }

    false
}
