use ss15_protocol::{EntitySnapshot, NetPosition, ServerMessage};

pub fn initial_snapshot() -> ServerMessage {
    ServerMessage::Snapshot {
        tick: 0,
        entities: vec![EntitySnapshot {
            net_id: 1,
            prototype: "debug.player".to_string(),
            position: NetPosition {
                x: 0.0,
                y: 0.0,
                z: 0,
            },
        }],
    }
}
