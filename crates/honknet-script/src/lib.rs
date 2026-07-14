use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub type ScriptEntityId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EngineToScript {
    Initialize {
        engine_version: String,
        module_path: String,
    },
    Tick {
        tick: u64,
        delta_seconds: f32,
        events: Vec<ScriptEvent>,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEvent {
    pub name: String,
    #[serde(default)]
    pub entity: Option<ScriptEntityId>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ScriptToEngine {
    Ready {
        module_id: String,
    },
    TickResult {
        tick: u64,
        commands: Vec<ScriptCommand>,
    },
    Log {
        level: String,
        message: String,
    },
    Error {
        message: String,
        stack: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "data")]
pub enum ScriptCommand {
    Log {
        level: String,
        message: String,
    },
    EmitSystemMessage {
        text: String,
    },
    Spawn {
        prototype: String,
        x: f32,
        y: f32,
        z: i32,
    },
    Delete {
        entity: ScriptEntityId,
    },
    EmitEvent {
        name: String,
        entity: Option<ScriptEntityId>,
        payload: Value,
    },
    SetComponent {
        entity: ScriptEntityId,
        component: String,
        state: Value,
    },
    RemoveComponent {
        entity: ScriptEntityId,
        component: String,
    },
    OpenUi {
        player: ScriptEntityId,
        target: ScriptEntityId,
        key: String,
        state: Value,
    },
    PlaySound {
        path: String,
        x: f32,
        y: f32,
        z: i32,
    },
}

#[derive(Debug, Error)]
pub enum ScriptHostError {
    #[error("failed to spawn script host: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("script host pipe unavailable")]
    PipeUnavailable,
    #[error("failed to encode script message: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("script host closed unexpectedly")]
    Closed,
}

pub struct NodeScriptHost {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl NodeScriptHost {
    pub fn spawn(
        node_executable: impl AsRef<Path>,
        host_entrypoint: impl AsRef<Path>,
    ) -> Result<Self, ScriptHostError> {
        let mut child = Command::new(node_executable.as_ref())
            .arg(host_entrypoint.as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let input = child.stdin.take().ok_or(ScriptHostError::PipeUnavailable)?;
        let output = child
            .stdout
            .take()
            .ok_or(ScriptHostError::PipeUnavailable)?;
        Ok(Self {
            child,
            input,
            output: BufReader::new(output),
        })
    }

    pub fn request(&mut self, message: &EngineToScript) -> Result<ScriptToEngine, ScriptHostError> {
        serde_json::to_writer(&mut self.input, message)?;
        self.input.write_all(b"\n")?;
        self.input.flush()?;
        let mut line = String::new();
        if self.output.read_line(&mut line)? == 0 {
            return Err(ScriptHostError::Closed);
        }
        Ok(serde_json::from_str(&line)?)
    }

    pub fn shutdown(mut self) -> Result<(), ScriptHostError> {
        let _ = self.request(&EngineToScript::Shutdown);
        let _ = self.child.wait();
        Ok(())
    }
}
