use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::error;

pub type AssetId = u32;
pub type StateId = u32;

#[derive(Debug, Error)]
pub enum RsiError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsiMeta {
    pub version: u32,
    pub size: [u32; 2],
    pub license: Option<String>,
    pub copyright: Option<String>,
    pub states: Vec<RsiState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsiState {
    pub name: String,
    pub directions: u32,
    pub delays: Option<Vec<Vec<f32>>>,
    pub flags: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsiDirection {
    South,
    North,
    East,
    West,
    SouthEast,
    SouthWest,
    NorthEast,
    NorthWest,
}

pub struct RsiReader {
    path: std::path::PathBuf,
    meta: Option<RsiMeta>,
}

impl RsiReader {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            meta: None,
        }
    }

    pub fn read_meta(&mut self) -> Result<&RsiMeta, RsiError> {
        if self.meta.is_none() {
            let meta_path = self.path.join("meta.json");
            let file = std::fs::File::open(meta_path)?;
            let meta: RsiMeta = serde_json::from_reader(file)?;
            self.meta = Some(meta);
        }
        Ok(self.meta.as_ref().unwrap())
    }

    pub fn validate(&mut self) -> Result<(), RsiError> {
        let meta = self.read_meta()?.clone();
        for state in &meta.states {
            if ![1, 4, 8].contains(&state.directions) {
                return Err(RsiError::Validation(format!(
                    "Invalid directions count {} for state {}",
                    state.directions, state.name
                )));
            }
            if let Some(delays) = &state.delays {
                if delays.len() != state.directions as usize {
                    return Err(RsiError::Validation(format!(
                        "Delays array size does not match directions for state {}",
                        state.name
                    )));
                }
            }
            let img_path = self.path.join(format!("{}.png", state.name));
            if !img_path.exists() {
                return Err(RsiError::Validation(format!(
                    "Missing PNG file for state {}",
                    state.name
                )));
            }
            let img = image::open(&img_path)?;
            if img.width() % meta.size[0] != 0 || img.height() % meta.size[1] != 0 {
                return Err(RsiError::Validation(format!(
                    "PNG dimensions are not a multiple of state size for state {}",
                    state.name
                )));
            }
        }
        Ok(())
    }

    pub fn read_frame(
        &self,
        state: &str,
        dir: RsiDirection,
        frame: u32,
    ) -> Result<DynamicImage, RsiError> {
        let meta = self
            .meta
            .as_ref()
            .ok_or_else(|| RsiError::Validation("Meta not loaded".to_string()))?;
        let state_info = meta
            .states
            .iter()
            .find(|s| s.name == state)
            .ok_or_else(|| RsiError::Validation("State not found".to_string()))?;
        let img_path = self.path.join(format!("{state}.png"));
        let img = image::open(img_path)?;

        let cols = img.width() / meta.size[0];
        let dir_index = match dir {
            RsiDirection::South => 0,
            RsiDirection::North => 1,
            RsiDirection::East => 2,
            RsiDirection::West => 3,
            RsiDirection::SouthEast => 4,
            RsiDirection::SouthWest => 5,
            RsiDirection::NorthEast => 6,
            RsiDirection::NorthWest => 7,
        };

        if dir_index >= state_info.directions {
            return Err(RsiError::Validation("Direction out of bounds".to_string()));
        }

        let _frames_per_dir = cols; // Simplified, assuming single row or matching structure
        let x = (frame % cols) * meta.size[0];
        let y = dir_index * meta.size[1]; // assuming vertically stacked directions

        Ok(img.crop_imm(x, y, meta.size[0], meta.size[1]))
    }

    pub fn list_states(&self) -> Result<Vec<String>, RsiError> {
        let meta = self
            .meta
            .as_ref()
            .ok_or_else(|| RsiError::Validation("Meta not loaded".to_string()))?;
        Ok(meta.states.iter().map(|s| s.name.clone()).collect())
    }
}

pub struct AtlasEntry {
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct AtlasPage {
    pub id: u32,
    pub texture_id: u32,
    pub width: u32,
    pub height: u32,
}
