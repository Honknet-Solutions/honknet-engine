pub mod components;
pub mod systems;

use honknet_ecs::{StorageKind, World};
use honknet_prototypes::PrototypeManager;
use honknet_sdk::GameModule;
use tracing::info;

pub struct SpaceStation15Module;

impl Default for SpaceStation15Module {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceStation15Module {
    pub fn new() -> Self {
        Self
    }
}

impl GameModule for SpaceStation15Module {
    fn name(&self) -> &'static str {
        "SpaceStation15"
    }

    fn register_components(&self, world: &mut World) {
        world.register::<components::DoorComponent>(StorageKind::Packed);
        world.register::<components::HandsComponent>(StorageKind::Packed);
        world.register::<components::ItemComponent>(StorageKind::Packed);
        world.register::<components::InteractionComponent>(StorageKind::Packed);
        world.register::<components::ExamineComponent>(StorageKind::Packed);
        world.register::<components::ContainerComponent>(StorageKind::Packed);
        world.register::<components::MobStateComponent>(StorageKind::Packed);
        world.register::<components::HealthComponent>(StorageKind::Packed);
        info!("SpaceStation15 registered core gameplay components");
    }

    fn register_prototypes(&self, _manager: &PrototypeManager) {
        info!("SpaceStation15 registered prototype schemas");
    }

    fn initialize_server(&self, _world: &mut World) {
        info!("SpaceStation15 server gameplay module initialized");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use components::*;
    use honknet_math::Vec2;
    use systems::*;

    #[test]
    fn test_interaction_and_door_toggle() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let user = world.spawn();
        world.insert(user, InteractionComponent::default()).unwrap();

        let door = world.spawn();
        world.insert(door, DoorComponent::default()).unwrap();

        // 1. User interacts with door within reach
        let success = interaction_system(&mut world, user, door, Vec2::ZERO, Vec2::new(1.0, 0.0));
        assert!(success);
        assert_eq!(world.get::<DoorComponent>(door).unwrap().state, DoorState::Open);

        // 2. Door auto-closes after delay
        door_system(&mut world, 3.5);
        assert_eq!(world.get::<DoorComponent>(door).unwrap().state, DoorState::Closed);
    }

    #[test]
    fn test_hands_pickup_and_drop() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let user = world.spawn();
        world.insert(user, HandsComponent::default()).unwrap();
        world.insert(user, InteractionComponent::default()).unwrap();

        let toolbox = world.spawn();
        world.insert(toolbox, ItemComponent::default()).unwrap();

        // Pickup item
        let picked = pick_up_item(&mut world, user, toolbox);
        assert!(picked);
        assert_eq!(world.get::<HandsComponent>(user).unwrap().item_in_hand, Some(toolbox));
        assert_eq!(world.get::<ItemComponent>(toolbox).unwrap().in_container, Some(user));

        // Drop item
        let dropped = drop_item(&mut world, user);
        assert_eq!(dropped, Some(toolbox));
        assert_eq!(world.get::<HandsComponent>(user).unwrap().item_in_hand, None);
        assert_eq!(world.get::<ItemComponent>(toolbox).unwrap().in_container, None);
    }

    #[test]
    fn test_health_and_mob_state() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let player = world.spawn();
        world.insert(player, HealthComponent { current: 100.0, max: 100.0 }).unwrap();
        world.insert(player, MobStateComponent::default()).unwrap();

        // Damage player to 15 HP (Critical)
        world.get_mut::<HealthComponent>(player).unwrap().current = 15.0;
        health_system(&mut world);
        assert_eq!(world.get::<MobStateComponent>(player).unwrap().state, MobState::Critical);

        // Damage player to 0 HP (Dead)
        world.get_mut::<HealthComponent>(player).unwrap().current = 0.0;
        health_system(&mut world);
        assert_eq!(world.get::<MobStateComponent>(player).unwrap().state, MobState::Dead);
    }
}
