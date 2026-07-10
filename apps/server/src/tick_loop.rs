use std::time::Duration;

use anyhow::Result;
use tokio::time;
use tracing::info;

use crate::server_state::SharedServerState;

pub async fn run(state: SharedServerState, tick_rate: u64) -> Result<()> {
    let tick_duration = Duration::from_secs_f64(1.0 / tick_rate as f64);
    let mut interval = time::interval(tick_duration);

    info!(tick_rate, ?tick_duration, "Starting server tick loop");

    loop {
        interval.tick().await;

        let mut state = state.write().await;
        state.advance_tick();
    }
}
