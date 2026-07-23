use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetZone {
    Head,
    Chest,
    Groin,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetZoneComponent {
    pub active_zone: TargetZone,
}

impl Component for TargetZoneComponent {}

impl Default for TargetZoneComponent {
    fn default() -> Self {
        Self {
            active_zone: TargetZone::Chest,
        }
    }
}
