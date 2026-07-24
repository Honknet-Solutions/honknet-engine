use super::TargetZone;
use honknet_core::Entity;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const STANDARD_BLOOD_VOLUME: f32 = 560.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyComponent {
    pub parts: BTreeMap<TargetZone, Entity>,
    pub organs: BTreeMap<OrganKind, Entity>,
}

impl Component for BodyComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPartComponent {
    pub owner: Entity,
    pub zone: TargetZone,
    pub brute_damage: f32,
    pub burn_damage: f32,
    pub max_damage: f32,
}

impl Component for BodyPartComponent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OrganKind {
    Brain,
    Heart,
    LeftLung,
    RightLung,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganComponent {
    pub owner: Entity,
    pub kind: OrganKind,
    pub integrity: f32,
    pub max_integrity: f32,
}

impl OrganComponent {
    pub fn functional(&self) -> bool {
        self.integrity > self.max_integrity * 0.2
    }
}

impl Component for OrganComponent {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WoundKind {
    Bruise,
    Cut,
    Burn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WoundTreatment {
    Bandage,
    BruisePack,
    BurnGel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WoundComponent {
    pub owner: Entity,
    pub body_part: Entity,
    pub kind: WoundKind,
    pub damage: f32,
    pub bleeding_rate: f32,
    pub bandaged: bool,
    pub treatment: Option<WoundTreatment>,
}

impl Component for WoundComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalSupplyComponent {
    pub treatment: WoundTreatment,
    pub charges: u16,
}

impl Component for MedicalSupplyComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodstreamComponent {
    pub volume: f32,
    pub max_volume: f32,
    pub oxygen_saturation: f32,
}

impl Default for BloodstreamComponent {
    fn default() -> Self {
        Self {
            volume: STANDARD_BLOOD_VOLUME,
            max_volume: STANDARD_BLOOD_VOLUME,
            oxygen_saturation: 1.0,
        }
    }
}

impl Component for BloodstreamComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespirationComponent {
    pub breathing: bool,
    pub external_oxygen: f32,
}

impl Default for RespirationComponent {
    fn default() -> Self {
        Self {
            breathing: true,
            external_oxygen: 1.0,
        }
    }
}

impl Component for RespirationComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysiologyComponent {
    pub pain: f32,
    pub shock: f32,
    pub conscious: bool,
    pub hypoxia_seconds: f32,
}

impl Default for PhysiologyComponent {
    fn default() -> Self {
        Self {
            pain: 0.0,
            shock: 0.0,
            conscious: true,
            hypoxia_seconds: 0.0,
        }
    }
}

impl Component for PhysiologyComponent {}
