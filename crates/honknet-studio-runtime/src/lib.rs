use anyhow::Result;
use honknet_runtime::{EngineRuntime, EngineRuntimeConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimePlaybackState {
    Stopped,
    Running,
    Paused,
}

pub struct EmbeddedStudioRuntime {
    pub engine: Option<EngineRuntime>,
    pub playback_state: RuntimePlaybackState,
    pub step_ticks_remaining: u64,
}

impl Default for EmbeddedStudioRuntime {
    fn default() -> Self {
        Self {
            engine: None,
            playback_state: RuntimePlaybackState::Stopped,
            step_ticks_remaining: 0,
        }
    }
}

impl EmbeddedStudioRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self) -> Result<()> {
        let engine = EngineRuntime::new(EngineRuntimeConfig::default())?;
        self.engine = Some(engine);
        self.playback_state = RuntimePlaybackState::Running;
        Ok(())
    }

    pub fn pause(&mut self) {
        if self.playback_state == RuntimePlaybackState::Running {
            self.playback_state = RuntimePlaybackState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.playback_state == RuntimePlaybackState::Paused {
            self.playback_state = RuntimePlaybackState::Running;
        }
    }

    pub fn step(&mut self, ticks: u64) {
        self.step_ticks_remaining = ticks;
        self.playback_state = RuntimePlaybackState::Running;
    }

    pub fn stop(&mut self) {
        self.engine = None;
        self.playback_state = RuntimePlaybackState::Stopped;
    }

    pub fn tick(&mut self, dt_seconds: f32) -> Result<()> {
        if self.playback_state == RuntimePlaybackState::Stopped {
            return Ok(());
        }

        if self.playback_state == RuntimePlaybackState::Paused && self.step_ticks_remaining == 0 {
            return Ok(());
        }

        if let Some(ref mut engine) = self.engine {
            engine.tick(dt_seconds)?;
        }

        if self.step_ticks_remaining > 0 {
            self.step_ticks_remaining -= 1;
            if self.step_ticks_remaining == 0 {
                self.playback_state = RuntimePlaybackState::Paused;
            }
        }

        Ok(())
    }
}
