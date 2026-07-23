use crate::components::atmos::TileAtmosphereComponent;
use honknet_ecs::World;
use tracing::info;

pub fn atmosphere_system(world: &mut World, _delta_time: f32) {
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
                info!("Plasma fire burning on tile {:?}! Temp: {}K", e, tile.air.temperature);
            }
        }
    }
}
