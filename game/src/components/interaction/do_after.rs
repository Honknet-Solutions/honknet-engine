use honknet_core::Entity;
use honknet_ecs::Component;
use honknet_math::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoAfterKind {
    Bandage,
    TreatBruise,
    TreatBurn,
    Cpr,
    Carry,
    Surgery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoAfterComponent {
    pub timer_id: u64,
    pub peer: u64,
    pub sequence: u32,
    pub kind: DoAfterKind,
    pub target: Entity,
    pub supply: Option<Entity>,
    pub started_tick: u64,
    pub completes_tick: u64,
    pub start_position: Vec2,
    pub target_start_position: Vec2,
    pub max_movement: f32,
}

impl Component for DoAfterComponent {}
