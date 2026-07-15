use honknet_protocol::{EntityNetId, ServerMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageTarget {
    Broadcast,
    Player(EntityNetId),
}

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub target: MessageTarget,
    pub message: ServerMessage,
}

impl OutboundMessage {
    pub fn broadcast(message: ServerMessage) -> Self {
        Self {
            target: MessageTarget::Broadcast,
            message,
        }
    }

    pub fn player(player: EntityNetId, message: ServerMessage) -> Self {
        Self {
            target: MessageTarget::Player(player),
            message,
        }
    }

    pub fn is_for(&self, player: EntityNetId) -> bool {
        match self.target {
            MessageTarget::Broadcast => true,
            MessageTarget::Player(target) => target == player,
        }
    }
}
