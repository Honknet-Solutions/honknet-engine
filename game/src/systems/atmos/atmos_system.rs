use crate::components::atmos::{
    AtmosConnectionComponent, BreathingEnvironmentComponent, GasMix, TileAtmosphereComponent,
};
use crate::components::doors::{DoorComponent, DoorState};
use crate::components::health::RespirationComponent;
use honknet_ecs::World;
use tracing::info;

pub fn atmosphere_system(world: &mut World, _delta_time: f32) {
    exchange_connected_tiles(world, _delta_time);
    update_breathing_environments(world, _delta_time);
    let tiles = world.query::<TileAtmosphereComponent>();

    for e in tiles {
        if let Some(tile) = world.get_mut::<TileAtmosphereComponent>(e) {
            // 1. Handle space vacuum depressurization
            if tile.is_space {
                tile.air.oxygen = 0.0;
                tile.air.nitrogen = 0.0;
                tile.air.carbon_dioxide = 0.0;
                tile.air.plasma = 0.0;
                tile.air.temperature = 2.7; // 2.7K Space temperature
            }

            // 2. Plasma combustion simulation
            if tile.air.plasma > 0.0 && tile.air.oxygen > 0.0 && tile.air.temperature >= 373.15 {
                let burn_amount = tile.air.plasma.min(tile.air.oxygen * 0.5);
                tile.air.plasma -= burn_amount;
                tile.air.oxygen -= burn_amount * 0.5;
                tile.air.carbon_dioxide += burn_amount;
                tile.air.temperature += burn_amount * 50.0;
                info!(
                    "Plasma fire burning on tile {:?}! Temp: {}K",
                    e, tile.air.temperature
                );
            }
        }
    }
}

fn update_breathing_environments(world: &mut World, delta_time: f32) {
    for entity in world.query::<BreathingEnvironmentComponent>() {
        let Some(atmosphere) = world
            .get::<BreathingEnvironmentComponent>(entity)
            .map(|environment| environment.atmosphere)
        else {
            continue;
        };
        let Some(tile) = world.get::<TileAtmosphereComponent>(atmosphere).cloned() else {
            continue;
        };
        let total = tile.air.total_moles();
        let oxygen_fraction = if total > 0.0 {
            tile.air.oxygen / total
        } else {
            0.0
        };
        let breathing = world
            .get::<RespirationComponent>(entity)
            .is_some_and(|respiration| respiration.breathing);
        if let Some(respiration) = world.get_mut::<RespirationComponent>(entity) {
            respiration.external_oxygen = oxygen_fraction;
        }
        if breathing && oxygen_fraction > 0.0 {
            if let Some(tile) = world.get_mut::<TileAtmosphereComponent>(atmosphere) {
                let consumed = (0.005 * delta_time).min(tile.air.oxygen);
                tile.air.oxygen -= consumed;
                tile.air.carbon_dioxide += consumed;
            }
        }
    }
}

fn exchange_connected_tiles(world: &mut World, delta_time: f32) {
    let connections = world.query::<AtmosConnectionComponent>();
    for connection_entity in connections {
        let Some(connection) = world
            .get::<AtmosConnectionComponent>(connection_entity)
            .cloned()
        else {
            continue;
        };
        let blocked = connection
            .barrier
            .and_then(|entity| world.get::<DoorComponent>(entity))
            .is_some_and(|door| door.state != DoorState::Open);
        if blocked {
            continue;
        }
        let (Some(first), Some(second)) = (
            world
                .get::<TileAtmosphereComponent>(connection.first)
                .cloned(),
            world
                .get::<TileAtmosphereComponent>(connection.second)
                .cloned(),
        ) else {
            continue;
        };
        if first.is_space && second.is_space {
            continue;
        }
        if first.is_space {
            vent_to_space(world, connection.second, connection.conductance, delta_time);
            continue;
        }
        if second.is_space {
            vent_to_space(world, connection.first, connection.conductance, delta_time);
            continue;
        }

        let first_pressure = first.air.pressure(first.volume);
        let second_pressure = second.air.pressure(second.volume);
        let pressure_scale = first_pressure.max(second_pressure).max(1.0);
        let fraction = ((first_pressure - second_pressure).abs() / pressure_scale
            * connection.conductance
            * delta_time)
            .clamp(0.0, 0.5);
        if fraction <= f32::EPSILON {
            continue;
        }
        let (donor, receiver) = if first_pressure > second_pressure {
            (connection.first, connection.second)
        } else {
            (connection.second, connection.first)
        };
        let moved = world
            .get_mut::<TileAtmosphereComponent>(donor)
            .map(|tile| tile.air.remove_fraction(fraction))
            .unwrap_or_else(empty_gas);
        if let Some(tile) = world.get_mut::<TileAtmosphereComponent>(receiver) {
            tile.air.merge(moved);
        }
    }
}

fn vent_to_space(world: &mut World, tile_entity: honknet_core::Entity, conductance: f32, dt: f32) {
    if let Some(tile) = world.get_mut::<TileAtmosphereComponent>(tile_entity) {
        tile.air
            .remove_fraction((conductance * dt).clamp(0.0, 0.75));
        tile.air.temperature += (2.7 - tile.air.temperature) * (0.05 * dt).clamp(0.0, 1.0);
    }
}

fn empty_gas() -> GasMix {
    GasMix {
        oxygen: 0.0,
        nitrogen: 0.0,
        carbon_dioxide: 0.0,
        plasma: 0.0,
        temperature: 2.7,
    }
}
