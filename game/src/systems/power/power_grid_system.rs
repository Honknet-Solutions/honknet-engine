use crate::components::power::{
    ApcComponent, PowerChannel, PowerNetworkMemberComponent, PoweredComponent, SmesComponent,
};
use honknet_core::Entity;
use honknet_ecs::World;
use std::collections::HashMap;
use tracing::info;

pub fn power_grid_system(world: &mut World, delta_time: f32) {
    let delta_time = delta_time.max(0.0);
    charge_apcs_from_smes(world, delta_time);

    let mut demand = HashMap::<(u32, PowerChannel), f32>::new();
    let devices = world.query::<PoweredComponent>();
    for entity in &devices {
        let Some(device) = world.get::<PoweredComponent>(*entity) else {
            continue;
        };
        if !device.requires_power {
            continue;
        }
        let member = world
            .get::<PowerNetworkMemberComponent>(*entity)
            .copied()
            .unwrap_or_default();
        *demand.entry((member.network, member.channel)).or_default() +=
            device.idle_power_draw.max(0.0) * delta_time;
    }

    let mut channel_available = HashMap::<(u32, PowerChannel), bool>::new();
    for (&key, &required) in &demand {
        channel_available.insert(key, drain_network_channel(world, key, required));
    }

    for entity in devices {
        let requires_power = world
            .get::<PoweredComponent>(entity)
            .is_some_and(|device| device.requires_power);
        let member = world
            .get::<PowerNetworkMemberComponent>(entity)
            .copied()
            .unwrap_or_default();
        let powered = !requires_power
            || channel_available
                .get(&(member.network, member.channel))
                .copied()
                .unwrap_or_else(|| network_has_charge(world, member.network, member.channel));
        if let Some(device) = world.get_mut::<PoweredComponent>(entity) {
            device.is_powered = powered;
        }
    }

    update_apc_channel_flags(world);
}

fn charge_apcs_from_smes(world: &mut World, delta_time: f32) {
    for smes_entity in world.query::<SmesComponent>() {
        let network = world
            .get::<PowerNetworkMemberComponent>(smes_entity)
            .map_or(0, |member| member.network);
        let apcs = apcs_in_network(world, network);
        let capacity = apcs
            .iter()
            .filter_map(|entity| world.get::<ApcComponent>(*entity))
            .map(|apc| (apc.cell_capacity - apc.cell_charge).max(0.0))
            .sum::<f32>();
        let available = world
            .get::<SmesComponent>(smes_entity)
            .map(|smes| {
                (smes.output_rate.max(0.0) * delta_time)
                    .min(smes.charge)
                    .min(capacity)
            })
            .unwrap_or(0.0);
        if available <= 0.0 {
            continue;
        }
        if let Some(smes) = world.get_mut::<SmesComponent>(smes_entity) {
            smes.charge -= available;
        }
        let mut remaining = available;
        for apc_entity in apcs {
            let Some(apc) = world.get_mut::<ApcComponent>(apc_entity) else {
                continue;
            };
            let accepted = remaining.min((apc.cell_capacity - apc.cell_charge).max(0.0));
            apc.cell_charge += accepted;
            remaining -= accepted;
            if remaining <= f32::EPSILON {
                break;
            }
        }
    }
}

fn drain_network_channel(
    world: &mut World,
    (network, _channel): (u32, PowerChannel),
    required: f32,
) -> bool {
    let apcs = apcs_in_network(world, network);
    let available = apcs
        .iter()
        .filter_map(|entity| world.get::<ApcComponent>(*entity))
        .map(|apc| apc.cell_charge)
        .sum::<f32>();
    if available + f32::EPSILON < required {
        for entity in apcs {
            if let Some(apc) = world.get_mut::<ApcComponent>(entity) {
                apc.cell_charge = 0.0;
            }
        }
        info!("Power network {network} depleted");
        return false;
    }
    let mut remaining = required;
    for entity in apcs {
        let Some(apc) = world.get_mut::<ApcComponent>(entity) else {
            continue;
        };
        let consumed = remaining.min(apc.cell_charge);
        apc.cell_charge -= consumed;
        remaining -= consumed;
        if remaining <= f32::EPSILON {
            break;
        }
    }
    true
}

fn network_has_charge(world: &World, network: u32, _channel: PowerChannel) -> bool {
    apcs_in_network(world, network)
        .into_iter()
        .filter_map(|entity| world.get::<ApcComponent>(entity))
        .any(|apc| apc.cell_charge > 0.0)
}

fn apcs_in_network(world: &World, network: u32) -> Vec<Entity> {
    world
        .query::<ApcComponent>()
        .into_iter()
        .filter(|entity| {
            world
                .get::<PowerNetworkMemberComponent>(*entity)
                .map_or(network == 0, |member| member.network == network)
        })
        .collect()
}

fn update_apc_channel_flags(world: &mut World) {
    for entity in world.query::<ApcComponent>() {
        let network = world
            .get::<PowerNetworkMemberComponent>(entity)
            .map_or(0, |member| member.network);
        let equipment = network_has_charge(world, network, PowerChannel::Equipment);
        let lighting = network_has_charge(world, network, PowerChannel::Lighting);
        let environment = network_has_charge(world, network, PowerChannel::Environment);
        if let Some(apc) = world.get_mut::<ApcComponent>(entity) {
            apc.equipment_powered = equipment;
            apc.lighting_powered = lighting;
            apc.environment_powered = environment;
        }
    }
}
