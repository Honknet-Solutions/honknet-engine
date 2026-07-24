use super::TargetZone;
use crate::components::tools::ToolType;
use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurgeryStep {
    Incision,
    ClampBleeders,
    Retract,
    Repair,
    Close,
    Complete,
}

impl SurgeryStep {
    pub fn required_tool(self) -> Option<ToolType> {
        match self {
            Self::Incision => Some(ToolType::Scalpel),
            Self::ClampBleeders => Some(ToolType::Hemostat),
            Self::Retract => Some(ToolType::Retractor),
            Self::Repair => Some(ToolType::Hemostat),
            Self::Close => Some(ToolType::Cautery),
            Self::Complete => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurgeryComponent {
    pub zone: TargetZone,
    pub step: SurgeryStep,
    pub incision_open: bool,
}

impl Component for SurgeryComponent {}
