use std::time::Duration;

use anyhow::Result;
use tokio::time::{self, MissedTickBehavior};
use tracing::info;

use crate::server_state::SharedServerState;

pub async fn run(state: SharedServerState, tick_rate: u64) -> Result<()> {
    let tick_duration = Duration::from_secs_f64(1.0 / tick_rate as f64);

    let delta_seconds = tick_duration.as_secs_f32();

    let mut interval = time::interval(tick_duration);

    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    info!(tick_rate, ?tick_duration, "Starting server tick loop");

    loop {
        interval.tick().await;

        let mut state = state.write().await;

        state.advance_tick(delta_seconds);
    }
}
