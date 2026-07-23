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
        world.register::<components::InventoryComponent>(StorageKind::Packed);
        world.register::<components::InteractionComponent>(StorageKind::Packed);
        world.register::<components::ExamineComponent>(StorageKind::Packed);
        world.register::<components::ContainerComponent>(StorageKind::Packed);
        world.register::<components::MobStateComponent>(StorageKind::Packed);
        world.register::<components::HealthComponent>(StorageKind::Packed);
        world.register::<components::TargetZoneComponent>(StorageKind::Packed);
        world.register::<components::IdCardComponent>(StorageKind::Packed);
        world.register::<components::AccessReaderComponent>(StorageKind::Packed);
        world.register::<components::DoorBoltComponent>(StorageKind::Packed);
        world.register::<components::ToolComponent>(StorageKind::Packed);
        world.register::<components::CableComponent>(StorageKind::Packed);
        world.register::<components::SmesComponent>(StorageKind::Packed);
        world.register::<components::ApcComponent>(StorageKind::Packed);
        world.register::<components::PoweredComponent>(StorageKind::Packed);
        world.register::<components::TileAtmosphereComponent>(StorageKind::Packed);
        world.register::<components::PipeComponent>(StorageKind::Packed);
        world.register::<components::ReagentHolderComponent>(StorageKind::Packed);
        world.register::<components::ChemDispenserComponent>(StorageKind::Packed);
        world.register::<components::CombatIntentComponent>(StorageKind::Packed);
        world.register::<components::MeleeWeaponComponent>(StorageKind::Packed);
        info!("SpaceStation15 registered core TGStation gameplay components");
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

    #[test]
    fn test_inventory_equip_and_unequip() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let user = world.spawn();
        world.insert(user, HandsComponent::default()).unwrap();
        world.insert(user, InventoryComponent::default()).unwrap();

        let jumpsuit = world.spawn();
        world.insert(jumpsuit, ItemComponent::default()).unwrap();

        // 1. Pick up jumpsuit in hand
        pick_up_item(&mut world, user, jumpsuit);
        assert_eq!(world.get::<HandsComponent>(user).unwrap().item_in_hand, Some(jumpsuit));

        // 2. Equip jumpsuit from hand to Jumpsuit slot
        let equipped = equip_item(&mut world, user, EquipmentSlot::Jumpsuit);
        assert!(equipped);
        assert_eq!(world.get::<HandsComponent>(user).unwrap().item_in_hand, None);
        assert_eq!(world.get::<InventoryComponent>(user).unwrap().slots.get(&EquipmentSlot::Jumpsuit), Some(&Some(jumpsuit)));

        // 3. Unequip jumpsuit from Jumpsuit slot back to hand
        let unequipped = unequip_item(&mut world, user, EquipmentSlot::Jumpsuit);
        assert_eq!(unequipped, Some(jumpsuit));
        assert_eq!(world.get::<HandsComponent>(user).unwrap().item_in_hand, Some(jumpsuit));
        assert_eq!(world.get::<InventoryComponent>(user).unwrap().slots.get(&EquipmentSlot::Jumpsuit), Some(&None));
    }

    #[test]
    fn test_id_card_access_control() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let user = world.spawn();
        world.insert(user, HandsComponent::default()).unwrap();
        world.insert(user, InventoryComponent::default()).unwrap();
        world.insert(user, InteractionComponent::default()).unwrap();

        let restricted_door = world.spawn();
        world.insert(restricted_door, DoorComponent::default()).unwrap();
        world.insert(restricted_door, AccessReaderComponent {
            required_tags: vec!["Captain".to_string()],
        }).unwrap();

        // 1. Without Captain ID card: Access Denied
        let denied = interaction_system(&mut world, user, restricted_door, Vec2::ZERO, Vec2::ZERO);
        assert!(!denied);
        assert_eq!(world.get::<DoorComponent>(restricted_door).unwrap().state, DoorState::Closed);

        // 2. Equip Captain ID Card into IdCard slot: Access Granted
        let captain_id = world.spawn();
        world.insert(captain_id, ItemComponent::default()).unwrap();
        world.insert(captain_id, IdCardComponent {
            owner_name: "Captain".to_string(),
            job_title: "Captain".to_string(),
            access_tags: vec!["Captain".to_string()],
        }).unwrap();

        pick_up_item(&mut world, user, captain_id);
        equip_item(&mut world, user, EquipmentSlot::IdCard);

        let granted = interaction_system(&mut world, user, restricted_door, Vec2::ZERO, Vec2::ZERO);
        assert!(granted);
        assert_eq!(world.get::<DoorComponent>(restricted_door).unwrap().state, DoorState::Open);
    }

    #[test]
    fn test_bolted_door_denial() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let user = world.spawn();
        world.insert(user, InteractionComponent::default()).unwrap();

        let door = world.spawn();
        world.insert(door, DoorComponent::default()).unwrap();
        world.insert(door, DoorBoltComponent { is_bolted: true }).unwrap();

        // Bolted door cannot be opened via normal interaction
        let toggled = interaction_system(&mut world, user, door, Vec2::ZERO, Vec2::ZERO);
        assert!(!toggled);
        assert_eq!(world.get::<DoorComponent>(door).unwrap().state, DoorState::Closed);
    }

    #[test]
    fn test_power_grid_apc_depletion() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let apc = world.spawn();
        world.insert(apc, ApcComponent { cell_charge: 10.0, ..Default::default() }).unwrap();

        let device = world.spawn();
        world.insert(device, PoweredComponent::default()).unwrap();

        // 1. Tick power grid -> depletes APC cell and turns off power
        power_grid_system(&mut world, 1.0);
        assert_eq!(world.get::<ApcComponent>(apc).unwrap().cell_charge, 0.0);
        assert!(!world.get::<ApcComponent>(apc).unwrap().equipment_powered);
        assert!(!world.get::<PoweredComponent>(device).unwrap().is_powered);
    }

    #[test]
    fn test_linda_plasma_fire() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let tile = world.spawn();
        world.insert(tile, TileAtmosphereComponent {
            air: GasMix {
                oxygen: 50.0,
                plasma: 50.0,
                temperature: 400.0, // Above ignition point 373.15K
                ..Default::default()
            },
            ..Default::default()
        }).unwrap();

        // Tick atmos -> triggers plasma combustion
        atmosphere_system(&mut world, 1.0);
        let air = &world.get::<TileAtmosphereComponent>(tile).unwrap().air;
        assert!(air.plasma < 50.0);
        assert!(air.temperature > 400.0);
    }

    #[test]
    fn test_chemistry_bicaridine_healing() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let patient = world.spawn();
        world.insert(patient, HealthComponent { current: 50.0, max: 100.0 }).unwrap();
        world.insert(patient, ReagentHolderComponent {
            max_volume: 50.0,
            reagents: vec![ReagentVolume { id: ReagentId::Bicaridine, volume: 10.0 }],
        }).unwrap();

        // Tick chemistry -> metabolizes Bicaridine and heals patient HP
        chemistry_system(&mut world, 1.0);
        assert!(world.get::<HealthComponent>(patient).unwrap().current > 50.0);
    }

    #[test]
    fn test_combat_harm_intent_attack() {
        let mut world = World::default();
        let module = SpaceStation15Module::new();
        module.register_components(&mut world);

        let attacker = world.spawn();
        world.insert(attacker, CombatIntentComponent { intent: CombatIntent::Harm }).unwrap();
        world.insert(attacker, TargetZoneComponent { active_zone: TargetZone::Head }).unwrap();

        let victim = world.spawn();
        world.insert(victim, HealthComponent { current: 100.0, max: 100.0 }).unwrap();

        // Attack victim with Harm intent
        let hit = attack_entity(&mut world, attacker, victim);
        assert!(hit);
        assert!(world.get::<HealthComponent>(victim).unwrap().current < 100.0);
    }
}
