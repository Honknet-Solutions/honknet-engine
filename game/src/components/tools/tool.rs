use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolType {
    Crowbar,
    Welder,
    Screwdriver,
    Wrench,
    Scalpel,
    Hemostat,
    Retractor,
    Cautery,
    SurgicalSaw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolComponent {
    pub tool_type: ToolType,
    pub use_delay: f32,
}

impl Component for ToolComponent {}

impl Default for ToolComponent {
    fn default() -> Self {
        Self {
            tool_type: ToolType::Crowbar,
            use_delay: 1.0,
        }
    }
}
