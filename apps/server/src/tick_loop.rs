use std::time::Duration;

use anyhow::Result;
use honknet_script::{EngineToScript, ScriptToEngine};
use tokio::time::{self, MissedTickBehavior};
use tracing::{error, info, warn};

use crate::app_state::AppState;

pub async fn run(state: AppState, tick_rate: u64) -> Result<()> {
    let tick_duration = Duration::from_secs_f64(1.0 / tick_rate as f64);
    let delta_seconds = tick_duration.as_secs_f32();
    let autosave_seconds = std::env::var("HONKNET_AUTOSAVE_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(300);
    let autosave_ticks = autosave_seconds.saturating_mul(tick_rate).max(1);
    let mut interval = time::interval(tick_duration);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    info!(
        tick_rate,
        ?tick_duration,
        autosave_seconds,
        "Starting server tick loop"
    );

    loop {
        interval.tick().await;

        let (tick, script_events) = {
            let mut game = state.game.write().await;
            game.advance_tick(delta_seconds);
            (game.current_tick(), game.take_script_events())
        };

        let script_response = {
            let mut guard = match state.script.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("Script host mutex was poisoned; recovering");
                    poisoned.into_inner()
                }
            };
            guard.as_mut().map(|host| {
                host.request(&EngineToScript::Tick {
                    tick,
                    delta_seconds,
                    events: script_events,
                })
            })
        };

        match script_response {
            Some(Ok(ScriptToEngine::TickResult {
                tick: response_tick,
                commands,
            })) => {
                if response_tick != tick {
                    warn!(
                        tick,
                        response_tick, "Discarding out-of-order script response"
                    );
                    continue;
                }
                let outgoing = state.game.write().await.apply_script_commands(commands);
                for message in outgoing {
                    let _ = state.events.send(message);
                }
            }
            Some(Ok(ScriptToEngine::Log { level, message })) => {
                info!(%level, %message, "Script host log");
            }
            Some(Ok(ScriptToEngine::Error { message, stack })) => {
                error!(%message, ?stack, "Script host error");
            }
            Some(Ok(other)) => {
                warn!(?other, "Unexpected script host response during tick");
            }
            Some(Err(error)) => {
                error!(%error, "Script host request failed; disabling game scripts");
                if let Ok(mut guard) = state.script.lock() {
                    *guard = None;
                }
            }
            None => {}
        }

        if tick % autosave_ticks == 0 {
            if let Err(error) = state.save_world().await {
                error!(%error, tick, "Autosave failed");
            } else {
                info!(tick, "Autosave completed");
            }
        }
    }
}
