use crate::components::interaction::ExamineComponent;
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn examine_system(world: &World, target: Entity) -> Option<String> {
    if let Some(examine) = world.get::<ExamineComponent>(target) {
        let desc = format!("Examine {:?}: {}\nInfo: {}", target, examine.description, examine.detailed_info);
        info!("{}", desc);
        return Some(desc);
    }
    None
}
