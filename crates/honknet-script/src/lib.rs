use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread::{self, JoinHandle},
    time::Duration,
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
        world: ScriptWorldDelta,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptWorldDelta {
    /// A full delta replaces the complete script-side world cache. Incremental
    /// deltas only update the listed entities and remove explicit ids.
    #[serde(default)]
    pub full: bool,
    #[serde(default)]
    pub upserts: Vec<ScriptEntitySnapshot>,
    #[serde(default)]
    pub removals: Vec<ScriptEntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEntitySnapshot {
    pub entity: ScriptEntityId,
    pub prototype: String,
    #[serde(default)]
    pub components: BTreeMap<String, Value>,
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
    EmitPlayerMessage {
        player: ScriptEntityId,
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
    UpdateUi {
        player: ScriptEntityId,
        session_id: String,
        state: Value,
    },
    CloseUi {
        player: ScriptEntityId,
        session_id: String,
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
    #[error("script host response timed out after {0:?}")]
    Timeout(Duration),
    #[error("script host reader failed: {0}")]
    Reader(String),
}

type ReaderMessage = Result<ScriptToEngine, String>;

pub struct NodeScriptHost {
    child: Child,
    input: ChildStdin,
    responses: Receiver<ReaderMessage>,
    reader_thread: Option<JoinHandle<()>>,
}

impl NodeScriptHost {
    pub fn spawn(
        node_executable: impl AsRef<Path>,
        host_entrypoint: impl AsRef<Path>,
        readable_paths: &[PathBuf],
    ) -> Result<Self, ScriptHostError> {
        let mut command = Command::new(node_executable.as_ref());
        // Do not leak server secrets (authentication keys, database URLs,
        // deployment credentials) into gameplay code. Game modules receive
        // configuration through the typed engine API instead.
        let path = std::env::var_os("PATH");
        command.env_clear();
        if let Some(path) = path {
            command.env("PATH", path);
        }
        command.env("NODE_ENV", "production");
        command.env("NO_COLOR", "1");
        if script_permissions_enabled() {
            command.arg("--permission");
            for path in readable_paths {
                command.arg(format!("--allow-fs-read={}", path.display()));
            }
        }
        let mut child = command
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
        let (sender, responses) = mpsc::sync_channel::<ReaderMessage>(32);
        let reader_thread = thread::Builder::new()
            .name("honknet-script-reader".to_owned())
            .spawn(move || {
                let mut reader = BufReader::new(output);
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let parsed = serde_json::from_str::<ScriptToEngine>(&line)
                                .map_err(|error| error.to_string());
                            if sender.send(parsed).is_err() {
                                break;
                            }
                        }
                        Err(error) => {
                            let _ = sender.send(Err(error.to_string()));
                            break;
                        }
                    }
                }
            })?;

        Ok(Self {
            child,
            input,
            responses,
            reader_thread: Some(reader_thread),
        })
    }

    pub fn request(&mut self, message: &EngineToScript) -> Result<ScriptToEngine, ScriptHostError> {
        self.request_timeout(message, Duration::from_secs(5))
    }

    pub fn request_timeout(
        &mut self,
        message: &EngineToScript,
        timeout: Duration,
    ) -> Result<ScriptToEngine, ScriptHostError> {
        serde_json::to_writer(&mut self.input, message)?;
        self.input.write_all(b"\n")?;
        self.input.flush()?;
        match self.responses.recv_timeout(timeout) {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(error)) => Err(ScriptHostError::Reader(error)),
            Err(RecvTimeoutError::Timeout) => Err(ScriptHostError::Timeout(timeout)),
            Err(RecvTimeoutError::Disconnected) => Err(ScriptHostError::Closed),
        }
    }

    pub fn terminate(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    pub fn shutdown(mut self) -> Result<(), ScriptHostError> {
        let _ = self.request_timeout(&EngineToScript::Shutdown, Duration::from_secs(2));
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader_thread.take() {
            let _ = reader.join();
        }
        Ok(())
    }
}

fn script_permissions_enabled() -> bool {
    std::env::var("HONKNET_SCRIPT_PERMISSIONS")
        .ok()
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off" | "no"
            )
        })
        .unwrap_or(true)
}

impl Drop for NodeScriptHost {
    fn drop(&mut self) {
        let _ = self.child.kill();
        if let Some(reader) = self.reader_thread.take() {
            let _ = reader.join();
        }
    }
}
