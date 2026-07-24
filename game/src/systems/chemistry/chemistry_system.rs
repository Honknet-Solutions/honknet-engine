use crate::components::chemistry::{MetabolismComponent, ReagentHolderComponent, ReagentId};
use crate::components::health::{
    BodyComponent, BodyPartComponent, PhysiologyComponent, WoundComponent, WoundKind,
};
use honknet_core::Entity;
use honknet_ecs::World;

pub fn chemistry_system(world: &mut World, delta_time: f32) {
    for entity in world.query::<ReagentHolderComponent>() {
        if !world.contains::<BodyComponent>(entity) {
            continue;
        }
        let rate = world
            .get::<MetabolismComponent>(entity)
            .map_or(0.5, |metabolism| metabolism.rate.max(0.0));
        let mut brute_healing = 0.0;
        let mut burn_healing = 0.0;
        let mut toxin_change = 0.0;
        let mut stabilization = 0.0;
        if let Some(holder) = world.get_mut::<ReagentHolderComponent>(entity) {
            for reagent in &mut holder.reagents {
                let metabolized = (rate * delta_time.max(0.0)).min(reagent.volume);
                reagent.volume -= metabolized;
                match reagent.id {
                    ReagentId::Bicaridine => brute_healing += metabolized * 4.0,
                    ReagentId::Kelotane => burn_healing += metabolized * 4.0,
                    ReagentId::Dylovene => toxin_change -= metabolized * 2.0,
                    ReagentId::Inaprovline => stabilization += metabolized * 0.1,
                    ReagentId::Plasma => toxin_change += metabolized * 2.0,
                    ReagentId::Acid => {
                        burn_healing -= metabolized * 3.0;
                        toxin_change += metabolized;
                    }
                    ReagentId::Water => {}
                }
            }
            holder.reagents.retain(|reagent| reagent.volume > 0.01);
        }
        heal_damage(world, entity, WoundKind::Bruise, brute_healing);
        heal_damage(world, entity, WoundKind::Cut, brute_healing * 0.5);
        heal_damage(world, entity, WoundKind::Burn, burn_healing);
        if burn_healing < 0.0 {
            damage_part(world, entity, WoundKind::Burn, -burn_healing);
        }
        if let Some(metabolism) = world.get_mut::<MetabolismComponent>(entity) {
            metabolism.toxin_load = (metabolism.toxin_load + toxin_change).max(0.0);
            metabolism.stabilization =
                (metabolism.stabilization + stabilization - delta_time * 0.01).max(0.0);
        }
        if let Some(physiology) = world.get_mut::<PhysiologyComponent>(entity) {
            physiology.shock =
                (physiology.shock + toxin_change.max(0.0) - stabilization * 10.0).max(0.0);
            physiology.hypoxia_seconds =
                (physiology.hypoxia_seconds - stabilization * 2.0).max(0.0);
        }
    }
}

fn heal_damage(world: &mut World, owner: Entity, kind: WoundKind, amount: f32) {
    if amount <= 0.0 {
        return;
    }
    let wound = world
        .query::<WoundComponent>()
        .into_iter()
        .filter_map(|entity| {
            world
                .get::<WoundComponent>(entity)
                .filter(|wound| wound.owner == owner && wound.kind == kind)
                .map(|wound| (entity, wound.body_part, wound.damage))
        })
        .max_by(|left, right| left.2.total_cmp(&right.2));
    let Some((wound_entity, part_entity, damage)) = wound else {
        return;
    };
    let healed = amount.min(damage);
    if let Some(wound) = world.get_mut::<WoundComponent>(wound_entity) {
        wound.damage -= healed;
        if wound.damage <= 0.01 {
            wound.bleeding_rate = 0.0;
        }
    }
    if let Some(part) = world.get_mut::<BodyPartComponent>(part_entity) {
        match kind {
            WoundKind::Burn => part.burn_damage = (part.burn_damage - healed).max(0.0),
            WoundKind::Bruise | WoundKind::Cut => {
                part.brute_damage = (part.brute_damage - healed).max(0.0);
            }
        }
    }
}

fn damage_part(world: &mut World, owner: Entity, kind: WoundKind, amount: f32) {
    let part = world
        .get::<BodyComponent>(owner)
        .and_then(|body| body.parts.values().next().copied());
    let Some(part) = part else {
        return;
    };
    if let Some(component) = world.get_mut::<BodyPartComponent>(part) {
        match kind {
            WoundKind::Burn => component.burn_damage += amount,
            WoundKind::Bruise | WoundKind::Cut => component.brute_damage += amount,
        }
    }
}
