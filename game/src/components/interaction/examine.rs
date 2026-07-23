use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamineComponent {
    pub description: String,
    pub detailed_info: String,
}

impl Component for ExamineComponent {}

impl Default for ExamineComponent {
    fn default() -> Self {
        Self {
            description: "An object on Space Station 15.".to_string(),
            detailed_info: "It looks sturdy.".to_string(),
        }
    }
}
