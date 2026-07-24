use crate::components::chemistry::{ReagentHolderComponent, ReagentId};
use crate::components::health::HealthComponent;
use honknet_ecs::World;
use tracing::info;

pub fn chemistry_system(world: &mut World, delta_time: f32) {
    let entities = world.query::<ReagentHolderComponent>();

    for e in entities {
        // Metabolize chemicals in living entities (that have HealthComponent)
        if world.contains::<HealthComponent>(e) {
            let mut heal_brute = 0.0;
            let mut heal_burn = 0.0;

            if let Some(holder) = world.get_mut::<ReagentHolderComponent>(e) {
                for r in holder.reagents.iter_mut() {
                    let metabolized = (0.5 * delta_time).min(r.volume);
                    r.volume -= metabolized;

                    match r.id {
                        ReagentId::Bicaridine => heal_brute += metabolized * 4.0,
                        ReagentId::Kelotane => heal_burn += metabolized * 4.0,
                        ReagentId::Plasma => heal_brute -= metabolized * 2.0, // Toxic
                        _ => {}
                    }
                }
                holder.reagents.retain(|r| r.volume > 0.01);
            }

            if let Some(health) = world.get_mut::<HealthComponent>(e) {
                if heal_brute != 0.0 || heal_burn != 0.0 {
                    health.current = (health.current + heal_brute + heal_burn).min(health.max);
                    info!(
                        "Metabolized medicine for entity {:?}. New HP: {}",
                        e, health.current
                    );
                }
            }
        }
    }
}
