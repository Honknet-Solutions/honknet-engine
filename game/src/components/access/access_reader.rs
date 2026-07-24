use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessReaderComponent {
    pub required_tags: Vec<String>,
}

impl Component for AccessReaderComponent {}
