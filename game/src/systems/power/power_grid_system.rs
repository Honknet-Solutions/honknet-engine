use crate::components::power::{ApcComponent, PoweredComponent, SmesComponent};
use honknet_ecs::World;
use tracing::info;

pub fn power_grid_system(world: &mut World, delta_time: f32) {
    // 1. Drain APC cells for powered devices
    let apcs = world.query::<ApcComponent>();
    let powered_devices = world.query::<PoweredComponent>();

    for apc_e in apcs {
        if let Some(apc) = world.get_mut::<ApcComponent>(apc_e) {
            let total_draw = powered_devices.len() as f32 * 50.0 * delta_time;
            if apc.cell_charge >= total_draw {
                apc.cell_charge -= total_draw;
                apc.equipment_powered = true;
            } else {
                apc.cell_charge = 0.0;
                apc.equipment_powered = false;
                info!("APC {:?} cell depleted! Area power turned off.", apc_e);
            }
        }
    }

    // 2. Update PoweredComponent status based on APC equipment state
    let is_area_powered = world
        .query::<ApcComponent>()
        .first()
        .and_then(|e| world.get::<ApcComponent>(*e))
        .map(|apc| apc.equipment_powered)
        .unwrap_or(true);

    for dev_e in powered_devices {
        if let Some(dev) = world.get_mut::<PoweredComponent>(dev_e) {
            if dev.requires_power {
                dev.is_powered = is_area_powered;
            }
        }
    }

    // 3. Discharge SMES to recharge APCs
    let smes_entities = world.query::<SmesComponent>();
    for smes_e in smes_entities {
        if let Some(smes) = world.get_mut::<SmesComponent>(smes_e) {
            let transfer = (smes.output_rate * delta_time).min(smes.charge);
            if transfer > 0.0 {
                smes.charge -= transfer;
                // Recharge APCs
                for apc_e in world.query::<ApcComponent>() {
                    if let Some(apc) = world.get_mut::<ApcComponent>(apc_e) {
                        apc.cell_charge = (apc.cell_charge + transfer).min(apc.cell_capacity);
                    }
                }
            }
        }
    }
}
