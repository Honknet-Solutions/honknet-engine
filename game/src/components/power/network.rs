use honknet_ecs::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PowerChannel {
    Equipment,
    Lighting,
    Environment,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PowerNetworkMemberComponent {
    pub network: u32,
    pub channel: PowerChannel,
}

impl Component for PowerNetworkMemberComponent {}

impl Default for PowerNetworkMemberComponent {
    fn default() -> Self {
        Self {
            network: 0,
            channel: PowerChannel::Equipment,
        }
    }
}
