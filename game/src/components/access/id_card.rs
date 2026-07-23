use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdCardComponent {
    pub owner_name: String,
    pub job_title: String,
    pub access_tags: Vec<String>,
}

impl Component for IdCardComponent {}

impl Default for IdCardComponent {
    fn default() -> Self {
        Self {
            owner_name: "Unknown Crewmember".to_string(),
            job_title: "Passenger".to_string(),
            access_tags: Vec::new(),
        }
    }
}
