use anyhow::Result;
use ss15_protocol::{EntitySnapshot, ServerMessage, Vec2};
use tracing::info;

const TICK_RATE: u64 = 30;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Space Station 15 authoritative server");
    info!(tick_rate = TICK_RATE, "Server tick configured");

    let snapshot = ServerMessage::Snapshot {
        tick: 0,
        entities: vec![EntitySnapshot {
            net_id: 1,
            prototype: "debug.player".to_string(),
            position: Vec2 { x: 0.0, y: 0.0 },
        }],
    };

    info!(message = %serde_json::to_string(&snapshot)?, "Generated debug snapshot");
    info!("Server scaffold is ready. WebSocket transport comes next.");

    Ok(())
}
