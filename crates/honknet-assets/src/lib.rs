use std::{fs, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("failed to read {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("failed to parse RSI metadata {path}: {source}")]
    RsiMetadata { path: PathBuf, source: serde_json::Error },
    #[error("invalid RSI at {path}: {message}")]
    InvalidRsi { path: PathBuf, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetManifest {
    pub version: u32,
    pub entries: Vec<AssetManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetManifestEntry {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

impl AssetManifest {
    pub fn build(root: impl AsRef<Path>, url_prefix: &str) -> Result<Self, AssetError> {
        let root = root.as_ref();
        let mut entries = Vec::new();
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let data = fs::read(path).map_err(|source| AssetError::Read {
                path: path.to_path_buf(),
                source,
            })?;
            let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
            let mut hasher = Sha256::new();
            hasher.update(&data);
            entries.push(AssetManifestEntry {
                path: format!("{}/{}", url_prefix.trim_end_matches('/'), relative),
                bytes: data.len() as u64,
                sha256: format!("{:x}", hasher.finalize()),
            });
        }
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self { version: 1, entries })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RsiMetadata {
    pub version: u32,
    pub size: RsiSize,
    pub states: Vec<RsiStateMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RsiSize {
    Object { x: u32, y: u32 },
    Array([u32; 2]),
}

impl RsiSize {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Object { x, y } => (*x, *y),
            Self::Array([x, y]) => (*x, *y),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RsiStateMetadata {
    pub name: String,
    #[serde(default = "one_direction")]
    pub directions: u8,
    #[serde(default)]
    pub delays: Vec<Vec<f32>>,
}

fn one_direction() -> u8 { 1 }

pub fn load_rsi(path: impl AsRef<Path>) -> Result<RsiMetadata, AssetError> {
    let path = path.as_ref();
    let meta_path = path.join("meta.json");
    let text = fs::read_to_string(&meta_path).map_err(|source| AssetError::Read {
        path: meta_path.clone(),
        source,
    })?;
    let metadata: RsiMetadata = serde_json::from_str(&text).map_err(|source| AssetError::RsiMetadata {
        path: meta_path.clone(),
        source,
    })?;
    let (width, height) = metadata.size.dimensions();
    if width == 0 || height == 0 {
        return Err(AssetError::InvalidRsi {
            path: path.to_path_buf(),
            message: "frame size must be non-zero".to_owned(),
        });
    }
    for state in &metadata.states {
        if !matches!(state.directions, 1 | 4 | 8) {
            return Err(AssetError::InvalidRsi {
                path: path.to_path_buf(),
                message: format!("state {} has unsupported direction count {}", state.name, state.directions),
            });
        }
        if !path.join(format!("{}.png", state.name)).is_file() {
            return Err(AssetError::InvalidRsi {
                path: path.to_path_buf(),
                message: format!("missing state image {}.png", state.name),
            });
        }
        if !state.delays.is_empty() && state.delays.len() != state.directions as usize {
            return Err(AssetError::InvalidRsi {
                path: path.to_path_buf(),
                message: format!("state {} delay direction count mismatch", state.name),
            });
        }
    }
    Ok(metadata)
}
