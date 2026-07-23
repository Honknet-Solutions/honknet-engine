use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessReaderComponent {
    pub required_tags: Vec<String>,
}

impl Component for AccessReaderComponent {}

impl Default for AccessReaderComponent {
    fn default() -> Self {
        Self {
            required_tags: Vec::new(),
        }
    }
}
