use honknet_studio_history::UndoRedoHistory;
use honknet_studio_project::{ProjectHealthReport, StudioProject};
use honknet_studio_protocol::{StudioCommand, StudioRequest, StudioResponse};

pub struct StudioCore {
    pub current_project: Option<StudioProject>,
    pub history: UndoRedoHistory,
}

impl Default for StudioCore {
    fn default() -> Self {
        Self {
            current_project: None,
            history: UndoRedoHistory::new(100),
        }
    }
}

impl StudioCore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_request(&mut self, request: StudioRequest) -> StudioResponse {
        match request.payload {
            StudioCommand::OpenProject { path } => match StudioProject::open(&path) {
                Ok(project) => {
                    self.current_project = Some(project);
                    StudioResponse {
                        request_id: request.id,
                        success: true,
                        error: None,
                        payload: Some(serde_json::json!({ "opened": path })),
                    }
                }
                Err(e) => StudioResponse {
                    request_id: request.id,
                    success: false,
                    error: Some(e.to_string()),
                    payload: None,
                },
            },
            StudioCommand::RunDiagnostics => {
                let report = if let Some(ref project) = self.current_project {
                    project.diagnose()
                } else {
                    ProjectHealthReport {
                        manifest_valid: false,
                        lock_valid: false,
                        engine_compatible: false,
                        missing_directories: vec![],
                        diagnostics_count: 1,
                    }
                };
                StudioResponse {
                    request_id: request.id,
                    success: true,
                    error: None,
                    payload: Some(serde_json::to_value(report).unwrap_or_default()),
                }
            }
            StudioCommand::Undo => {
                let success = self.history.undo().unwrap_or(false);
                StudioResponse {
                    request_id: request.id,
                    success,
                    error: None,
                    payload: Some(serde_json::json!({ "action": "undo" })),
                }
            }
            StudioCommand::Redo => {
                let success = self.history.redo().unwrap_or(false);
                StudioResponse {
                    request_id: request.id,
                    success,
                    error: None,
                    payload: Some(serde_json::json!({ "action": "redo" })),
                }
            }
            _ => StudioResponse {
                request_id: request.id,
                success: true,
                error: None,
                payload: Some(serde_json::json!({ "status": "processed" })),
            },
        }
    }
}
