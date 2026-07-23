use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatIntent {
    Help,
    Disarm,
    Grab,
    Harm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatIntentComponent {
    pub intent: CombatIntent,
}

impl Component for CombatIntentComponent {}

impl Default for CombatIntentComponent {
    fn default() -> Self {
        Self {
            intent: CombatIntent::Help,
        }
    }
}
