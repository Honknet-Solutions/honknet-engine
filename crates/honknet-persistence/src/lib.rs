use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct JsonStore {
    root: PathBuf,
}

impl JsonStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), PersistenceError> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| PersistenceError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let temporary = path.with_extension("tmp");
        let data = serde_json::to_vec_pretty(value)?;
        let mut file = fs::File::create(&temporary).map_err(|source| PersistenceError::Io {
            path: temporary.clone(),
            source,
        })?;
        file.write_all(&data)
            .map_err(|source| PersistenceError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.sync_all().map_err(|source| PersistenceError::Io {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, &path).map_err(|source| PersistenceError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(())
    }

    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, PersistenceError> {
        let path = self.path_for(key);
        if !path.is_file() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(|source| PersistenceError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(Some(serde_json::from_str(&text)?))
    }

    pub fn remove(&self, key: &str) -> Result<(), PersistenceError> {
        let path = self.path_for(key);
        if path.exists() {
            fs::remove_file(&path).map_err(|source| PersistenceError::Io { path, source })?;
        }
        Ok(())
    }

    fn path_for(&self, key: &str) -> PathBuf {
        let safe = key
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '/') {
                    character
                } else {
                    '_'
                }
            })
            .collect::<String>();
        self.root.join(Path::new(&safe)).with_extension("json")
    }
}
