use crate::components::combat::{CombatIntent, CombatIntentComponent, MeleeWeaponComponent};
use crate::components::hands::HandsComponent;
use crate::components::health::{HealthComponent, TargetZoneComponent};
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn attack_entity(world: &mut World, attacker: Entity, target: Entity) -> bool {
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
            let mut brute = 5.0; // Unarmed punch
            let mut burn = 0.0;

            if let Some(hands) = world.get::<HandsComponent>(attacker) {
                if let Some(weapon_e) = hands.item_in_hand {
                    if let Some(weapon) = world.get::<MeleeWeaponComponent>(weapon_e) {
                        brute = weapon.brute_damage;
                        burn = weapon.burn_damage;
                    }
                }
            }

            if let Some(health) = world.get_mut::<HealthComponent>(target) {
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
