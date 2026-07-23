use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeleeWeaponComponent {
    pub brute_damage: f32,
    pub burn_damage: f32,
    pub attack_cooldown: f32,
}

impl Component for MeleeWeaponComponent {}

impl Default for MeleeWeaponComponent {
    fn default() -> Self {
        Self {
            brute_damage: 10.0,
            burn_damage: 0.0,
            attack_cooldown: 1.0,
        }
    }
}
