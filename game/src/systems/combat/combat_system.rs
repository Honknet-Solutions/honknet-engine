use crate::components::combat::{CombatIntent, CombatIntentComponent, MeleeWeaponComponent};
use crate::components::hands::HandsComponent;
use crate::components::health::{BodyComponent, HealthComponent, TargetZoneComponent};
use crate::systems::health::apply_local_damage;
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn attack_damage(world: &World, attacker: Entity) -> (f32, f32) {
    let mut brute = 5.0;
    let mut burn = 0.0;
    if let Some(hands) = world.get::<HandsComponent>(attacker) {
        if let Some(weapon_entity) = hands.item_in_hand {
            if let Some(weapon) = world.get::<MeleeWeaponComponent>(weapon_entity) {
                brute = weapon.brute_damage;
                burn = weapon.burn_damage;
            }
        }
    }
    (brute, burn)
}

pub fn attack_entity(world: &mut World, attacker: Entity, target: Entity) -> bool {
    attack_entity_with_damage(world, attacker, target, None)
}

pub fn attack_entity_with_damage(
    world: &mut World,
    attacker: Entity,
    target: Entity,
    damage_override: Option<(f32, f32)>,
) -> bool {
    let intent = world
        .get::<CombatIntentComponent>(attacker)
        .map(|i| i.intent)
        .unwrap_or(CombatIntent::Help);

    let target_zone = world
        .get::<TargetZoneComponent>(attacker)
        .map(|z| z.active_zone)
        .unwrap_or(crate::components::health::TargetZone::Chest);

    match intent {
        CombatIntent::Harm => {
            // Check held weapon
            let (mut brute, mut burn) = attack_damage(world, attacker);
            if let Some((override_brute, override_burn)) = damage_override {
                brute = override_brute.max(0.0);
                burn = override_burn.max(0.0);
            }

            if world.contains::<BodyComponent>(target) {
                if apply_local_damage(world, target, target_zone, brute, burn).is_ok() {
                    info!(
                        "Attacker {:?} hit target {:?} in zone {:?} for {} brute damage",
                        attacker, target, target_zone, brute
                    );
                    return true;
                }
            } else if let Some(health) = world.get_mut::<HealthComponent>(target) {
                health.current = (health.current - brute - burn).max(0.0);
                info!(
                    "Attacker {:?} hit target {:?} in zone {:?} for {} brute damage! New HP: {}",
                    attacker, target, target_zone, brute, health.current
                );
                return true;
            }
        }
        CombatIntent::Disarm => {
            info!(
                "Attacker {:?} attempted to disarm target {:?}",
                attacker, target
            );
            return true;
        }
        CombatIntent::Help => {
            info!("Attacker {:?} helped target {:?}", attacker, target);
            return true;
        }
        CombatIntent::Grab => {
            info!("Attacker {:?} grabbed target {:?}", attacker, target);
            return true;
        }
    }
    false
}
