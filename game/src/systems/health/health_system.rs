use crate::components::health::{
    BloodstreamComponent, BodyComponent, BodyPartComponent, HealthComponent, MobState,
    MobStateComponent, OrganComponent, OrganKind, PhysiologyComponent, RespirationComponent,
    TargetZone, WoundComponent, WoundKind, WoundTreatment,
};
use honknet_core::Entity;
use honknet_ecs::{DynamicId, EcsError, World};
use std::collections::BTreeMap;
use tracing::info;

pub fn create_human_body(world: &mut World, owner: Entity) -> Result<(), EcsError> {
    if !world.is_alive(owner) {
        return Err(EcsError::Stale(owner));
    }
    let mut parts = BTreeMap::new();
    for (zone, max_damage) in [
        (TargetZone::Head, 60.0),
        (TargetZone::Chest, 100.0),
        (TargetZone::Groin, 80.0),
        (TargetZone::LeftArm, 50.0),
        (TargetZone::RightArm, 50.0),
        (TargetZone::LeftLeg, 60.0),
        (TargetZone::RightLeg, 60.0),
    ] {
        let part = world.spawn();
        world.insert(
            part,
            BodyPartComponent {
                owner,
                zone,
                brute_damage: 0.0,
                burn_damage: 0.0,
                max_damage,
            },
        )?;
        world.add_relation(
            &DynamicId::new("game.attachedTo").expect("registered relation ID"),
            part,
            owner,
        )?;
        parts.insert(zone, part);
    }
    let mut organs = BTreeMap::new();
    for (kind, zone, max_integrity) in [
        (OrganKind::Brain, TargetZone::Head, 100.0),
        (OrganKind::Heart, TargetZone::Chest, 100.0),
        (OrganKind::LeftLung, TargetZone::Chest, 80.0),
        (OrganKind::RightLung, TargetZone::Chest, 80.0),
    ] {
        let organ = world.spawn();
        world.insert(
            organ,
            OrganComponent {
                owner,
                kind,
                integrity: max_integrity,
                max_integrity,
            },
        )?;
        world.add_relation(
            &DynamicId::new("game.attachedTo").expect("registered relation ID"),
            organ,
            parts[&zone],
        )?;
        organs.insert(kind, organ);
    }
    world.insert(owner, BodyComponent { parts, organs })?;
    world.insert(owner, BloodstreamComponent::default())?;
    world.insert(owner, RespirationComponent::default())?;
    world.insert(owner, PhysiologyComponent::default())?;
    Ok(())
}

pub fn apply_local_damage(
    world: &mut World,
    owner: Entity,
    zone: TargetZone,
    brute: f32,
    burn: f32,
) -> Result<Entity, EcsError> {
    let part = world
        .get::<BodyComponent>(owner)
        .and_then(|body| body.parts.get(&zone).copied())
        .ok_or(EcsError::Stale(owner))?;
    let brute = brute.max(0.0);
    let burn = burn.max(0.0);
    let body_part = world
        .get_mut::<BodyPartComponent>(part)
        .ok_or(EcsError::Stale(part))?;
    body_part.brute_damage = (body_part.brute_damage + brute).min(body_part.max_damage);
    body_part.burn_damage = (body_part.burn_damage + burn).min(body_part.max_damage);

    let damage = brute + burn;
    let kind = if burn > brute {
        WoundKind::Burn
    } else if brute >= 8.0 {
        WoundKind::Cut
    } else {
        WoundKind::Bruise
    };
    let wound = world.spawn();
    world.insert(
        wound,
        WoundComponent {
            owner,
            body_part: part,
            kind,
            damage,
            bleeding_rate: if kind == WoundKind::Cut {
                damage * 0.08
            } else {
                0.0
            },
            bandaged: false,
            treatment: None,
        },
    )?;
    world.add_relation(
        &DynamicId::new("game.attachedTo").expect("registered relation ID"),
        wound,
        part,
    )?;
    Ok(wound)
}

pub fn bandage_wound(world: &mut World, wound: Entity) -> Result<(), EcsError> {
    let wound = world
        .get_mut::<WoundComponent>(wound)
        .ok_or(EcsError::Stale(wound))?;
    wound.bandaged = true;
    wound.bleeding_rate = 0.0;
    wound.treatment = Some(WoundTreatment::Bandage);
    Ok(())
}

pub fn bandage_most_severe_wound(world: &mut World, owner: Entity) -> bool {
    treat_most_severe_wound(world, owner, WoundTreatment::Bandage)
}

pub fn treat_most_severe_wound(
    world: &mut World,
    owner: Entity,
    treatment: WoundTreatment,
) -> bool {
    let expected_kind = match treatment {
        WoundTreatment::Bandage => WoundKind::Cut,
        WoundTreatment::BruisePack => WoundKind::Bruise,
        WoundTreatment::BurnGel => WoundKind::Burn,
    };
    let wound = world
        .query::<WoundComponent>()
        .into_iter()
        .filter_map(|entity| {
            world
                .get::<WoundComponent>(entity)
                .filter(|wound| {
                    wound.owner == owner
                        && wound.kind == expected_kind
                        && wound.treatment != Some(treatment)
                })
                .map(|wound| (entity, wound.damage))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(entity, _)| entity);
    let Some(wound_entity) = wound else {
        return false;
    };
    if treatment == WoundTreatment::Bandage {
        return bandage_wound(world, wound_entity).is_ok();
    }
    let Some(wound) = world.get::<WoundComponent>(wound_entity).cloned() else {
        return false;
    };
    let reduction = match treatment {
        WoundTreatment::BruisePack => wound.damage * 0.5,
        WoundTreatment::BurnGel => wound.damage * 0.4,
        WoundTreatment::Bandage => 0.0,
    };
    if let Some(part) = world.get_mut::<BodyPartComponent>(wound.body_part) {
        match treatment {
            WoundTreatment::BruisePack => {
                part.brute_damage = (part.brute_damage - reduction).max(0.0);
            }
            WoundTreatment::BurnGel => {
                part.burn_damage = (part.burn_damage - reduction).max(0.0);
            }
            WoundTreatment::Bandage => {}
        }
    }
    if let Some(wound) = world.get_mut::<WoundComponent>(wound_entity) {
        wound.damage = (wound.damage - reduction).max(0.0);
        wound.treatment = Some(treatment);
    }
    true
}

pub fn cpr_pulse(world: &mut World, patient: Entity) -> bool {
    let Some(blood) = world.get_mut::<BloodstreamComponent>(patient) else {
        return false;
    };
    if blood.volume <= blood.max_volume * 0.2 {
        return false;
    }
    blood.oxygen_saturation = (blood.oxygen_saturation + 0.15).min(1.0);
    if let Some(physiology) = world.get_mut::<PhysiologyComponent>(patient) {
        physiology.hypoxia_seconds = (physiology.hypoxia_seconds - 5.0).max(0.0);
    }
    true
}

pub fn physiology_system(world: &mut World, delta_seconds: f32) {
    let wounds = world.query::<WoundComponent>();
    let mut bleeding = BTreeMap::<Entity, f32>::new();
    for wound in wounds {
        if let Some(wound) = world.get::<WoundComponent>(wound) {
            if !wound.bandaged {
                *bleeding.entry(wound.owner).or_default() += wound.bleeding_rate;
            }
        }
    }
    for (owner, rate) in bleeding {
        if let Some(blood) = world.get_mut::<BloodstreamComponent>(owner) {
            blood.volume = (blood.volume - rate * delta_seconds).max(0.0);
        }
    }

    let bodies = world.query::<BodyComponent>();
    for owner in bodies {
        let Some(body) = world.get::<BodyComponent>(owner) else {
            continue;
        };
        let lungs_functional = [OrganKind::LeftLung, OrganKind::RightLung]
            .into_iter()
            .filter_map(|kind| body.organs.get(&kind))
            .filter_map(|organ| world.get::<OrganComponent>(*organ))
            .any(OrganComponent::functional);
        let heart_functional = body
            .organs
            .get(&OrganKind::Heart)
            .and_then(|organ| world.get::<OrganComponent>(*organ))
            .is_some_and(OrganComponent::functional);
        let respiration = world
            .get::<RespirationComponent>(owner)
            .cloned()
            .unwrap_or_default();
        let can_oxygenate =
            respiration.breathing && respiration.external_oxygen >= 0.16 && lungs_functional;
        if let Some(blood) = world.get_mut::<BloodstreamComponent>(owner) {
            let change = if can_oxygenate { 0.12 } else { -0.08 };
            blood.oxygen_saturation =
                (blood.oxygen_saturation + change * delta_seconds).clamp(0.0, 1.0);
            if !heart_functional {
                blood.oxygen_saturation = (blood.oxygen_saturation - 0.12 * delta_seconds).max(0.0);
            }
        }

        let pain = world
            .query::<WoundComponent>()
            .into_iter()
            .filter_map(|wound| world.get::<WoundComponent>(wound))
            .filter(|wound| wound.owner == owner)
            .map(|wound| {
                let treatment = if wound.bandaged { 0.35 } else { 1.0 };
                wound.damage
                    * treatment
                    * match wound.kind {
                        WoundKind::Bruise => 0.7,
                        WoundKind::Cut => 1.0,
                        WoundKind::Burn => 1.25,
                    }
            })
            .sum::<f32>();
        let (blood_ratio, oxygen) = world
            .get::<BloodstreamComponent>(owner)
            .map(|blood| {
                (
                    blood.volume / blood.max_volume.max(1.0),
                    blood.oxygen_saturation,
                )
            })
            .unwrap_or((1.0, 1.0));
        if let Some(physiology) = world.get_mut::<PhysiologyComponent>(owner) {
            physiology.pain = pain;
            let circulatory_shock = (1.0 - blood_ratio).max(0.0) * 100.0;
            let hypoxic_shock = (0.6 - oxygen).max(0.0) * 120.0;
            physiology.shock = (pain * 0.45 + circulatory_shock + hypoxic_shock).min(200.0);
            physiology.hypoxia_seconds = if oxygen < 0.35 {
                physiology.hypoxia_seconds + delta_seconds
            } else {
                (physiology.hypoxia_seconds - delta_seconds * 0.5).max(0.0)
            };
            physiology.conscious = blood_ratio > 0.4 && oxygen > 0.3 && physiology.shock < 80.0;
        }
    }
}

pub fn health_system(world: &mut World) {
    for entity in world.query::<BloodstreamComponent>() {
        let (ratio, oxygen) = world
            .get::<BloodstreamComponent>(entity)
            .map(|blood| {
                (
                    blood.volume / blood.max_volume.max(1.0),
                    blood.oxygen_saturation,
                )
            })
            .unwrap_or((1.0, 1.0));
        let physiology = world
            .get::<PhysiologyComponent>(entity)
            .cloned()
            .unwrap_or_default();
        let brain_dead = world
            .get::<BodyComponent>(entity)
            .and_then(|body| body.organs.get(&OrganKind::Brain))
            .and_then(|organ| world.get::<OrganComponent>(*organ))
            .is_some_and(|brain| !brain.functional());
        if let Some(state) = world.get_mut::<MobStateComponent>(entity) {
            state.state = if ratio <= 0.2 || brain_dead || physiology.hypoxia_seconds >= 180.0 {
                MobState::Dead
            } else if ratio <= 0.6 || oxygen <= 0.2 {
                MobState::Critical
            } else if !physiology.conscious {
                MobState::Unconscious
            } else {
                MobState::Alive
            };
        }
        if let Some(legacy) = world.get_mut::<HealthComponent>(entity) {
            legacy.current = ratio * 100.0;
            legacy.max = 100.0;
        }
    }

    let entities = world.query::<HealthComponent>();

    for e in entities {
        if world.contains::<BloodstreamComponent>(e) {
            continue;
        }
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
