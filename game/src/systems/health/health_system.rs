use crate::components::health::{HealthComponent, MobState, MobStateComponent};
use honknet_ecs::World;
use tracing::info;

pub fn health_system(world: &mut World) {
    let entities = world.query::<HealthComponent>();

    for e in entities {
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
