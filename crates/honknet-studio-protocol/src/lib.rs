use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioRequest {
    pub id: u64,
    pub payload: StudioCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StudioCommand {
    OpenProject {
        path: String,
    },
    SaveDocument {
        uri: String,
        content: String,
    },
    RunDiagnostics,
    LaunchPreviewRuntime {
        target: String,
    },
    StopPreviewRuntime,
    ApplyEditorCommand {
        command: String,
        params: serde_json::Value,
    },
    Undo,
    Redo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioResponse {
    pub request_id: u64,
    pub success: bool,
    pub error: Option<String>,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StudioEvent {
    ProjectOpened {
        path: String,
        engine_version: String,
    },
    DocumentChanged {
        uri: String,
        dirty: bool,
    },
    DiagnosticsUpdated {
        errors_count: usize,
        warnings_count: usize,
    },
    RuntimeStateChanged {
        running: bool,
        tps: f32,
    },
    LogMessage {
        level: String,
        message: String,
    },
}
