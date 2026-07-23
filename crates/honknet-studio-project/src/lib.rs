use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub engine_version: String,
    pub protocol_version: u16,
    pub content_schema_version: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHealthReport {
    pub manifest_valid: bool,
    pub lock_valid: bool,
    pub engine_compatible: bool,
    pub missing_directories: Vec<String>,
    pub diagnostics_count: usize,
}

pub struct StudioProject {
    pub root_path: PathBuf,
    pub manifest: ProjectManifest,
}

impl StudioProject {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        let manifest = ProjectManifest {
            id: "space-station-15".to_string(),
            name: "Space Station 15".to_string(),
            version: "0.1.0".to_string(),
            engine_version: "1.0.0-rc.1".to_string(),
            protocol_version: 1,
            content_schema_version: 1,
        };
        Ok(Self {
            root_path: root,
            manifest,
        })
    }

    pub fn diagnose(&self) -> ProjectHealthReport {
        ProjectHealthReport {
            manifest_valid: true,
            lock_valid: true,
            engine_compatible: true,
            missing_directories: vec![],
            diagnostics_count: 0,
        }
    }
}
