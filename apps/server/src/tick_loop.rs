use std::time::{Duration, Instant};

use anyhow::Result;
use honknet_script::{EngineToScript, ScriptToEngine};
use tokio::time::{self, MissedTickBehavior};
use tracing::{error, info, warn};

use crate::app_state::AppState;

pub async fn run(state: AppState, tick_rate: u64) -> Result<()> {
    let tick_duration = Duration::from_secs_f64(1.0 / tick_rate as f64);
    let tick_budget_micros = tick_duration.as_micros().try_into().unwrap_or(u64::MAX);
    let delta_seconds = tick_duration.as_secs_f32();
    let autosave_seconds = env_u64("HONKNET_AUTOSAVE_SECONDS", 300).max(1);
    let autosave_ticks = autosave_seconds.saturating_mul(tick_rate).max(1);
    let script_timeout =
        Duration::from_millis(env_u64("HONKNET_SCRIPT_MAX_TICK_MS", 12).max(1));
    let max_script_commands = env_usize("HONKNET_SCRIPT_MAX_COMMANDS", 4_096).max(1);
    let mut interval = time::interval(tick_duration);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    info!(
        tick_rate,
        ?tick_duration,
        autosave_seconds,
        ?script_timeout,
        max_script_commands,
        "Starting server tick loop"
    );

    loop {
        interval.tick().await;
        let tick_started = Instant::now();

        let (tick, script_events, script_world) = {
            let mut game = state.game.write().await;
            game.advance_tick(delta_seconds);
            let tick = game.current_tick();
            let events = game.take_script_events();
            let world = game.take_script_world_delta();
            (tick, events, world)
        };

        let script_response = tokio::task::block_in_place(|| {
            let mut guard = match state.script.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("Script host mutex was poisoned; recovering");
                    poisoned.into_inner()
                }
            };

            guard.as_mut().map(|host| {
                host.request_timeout(
                    &EngineToScript::Tick {
                        tick,
                        delta_seconds,
                        events: script_events,
                        world: script_world,
                    },
                    script_timeout,
                )
            })
        });

        match script_response {
            Some(Ok(ScriptToEngine::TickResult {
                tick: response_tick,
                commands,
            })) => {
                if response_tick != tick {
                    state.metrics.script_failure();
                    warn!(
                        tick,
                        response_tick, "Discarding out-of-order script response"
                    );
                } else if commands.len() > max_script_commands {
                    state.metrics.script_failure();
                    error!(
                        tick,
                        command_count = commands.len(),
                        max_script_commands,
                        "Script command budget exceeded; disabling game scripts"
                    );
                    disable_script_host(&state);
                } else {
                    let outgoing = state.game.write().await.apply_script_commands(commands);
                    for message in outgoing {
                        let _ = state.events.send(message);
                    }
                }
            }
            Some(Ok(ScriptToEngine::Log { level, message })) => {
                info!(%level, %message, "Script host log");
            }
            Some(Ok(ScriptToEngine::Error { message, stack })) => {
                state.metrics.script_failure();
                error!(%message, ?stack, "Script host error");
            }
            Some(Ok(other)) => {
                state.metrics.script_failure();
                warn!(?other, "Unexpected script host response during tick");
            }
            Some(Err(error)) => {
                state.metrics.script_failure();
                error!(%error, "Script host request failed; disabling game scripts");
                disable_script_host(&state);
            }
            None => {}
        }

        if tick % autosave_ticks == 0 {
            if state.request_background_save() {
                info!(tick, "Autosave queued");
            } else {
                warn!(tick, "Autosave skipped because a previous save is still running or persistence is disabled");
            }
        }

        let elapsed_micros = tick_started.elapsed().as_micros().try_into().unwrap_or(u64::MAX);
        state
            .metrics
            .tick_completed(elapsed_micros, tick_budget_micros);
        if elapsed_micros > tick_budget_micros {
            warn!(
                tick,
                elapsed_micros,
                tick_budget_micros, "Server tick exceeded its time budget"
            );
        }
    }
}

fn disable_script_host(state: &AppState) {
    if let Ok(mut guard) = state.script.lock() {
        if let Some(host) = guard.as_mut() {
            host.terminate();
        }
        *guard = None;
    }
}

fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(fallback)
}
