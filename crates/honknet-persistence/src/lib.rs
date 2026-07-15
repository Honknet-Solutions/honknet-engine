use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("invalid persistence key: {0}")]
    InvalidKey(String),
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

    /// Writes an object through a temporary file and keeps the previous valid
    /// value as a `.bak` recovery copy. The key is always resolved below the
    /// configured root and cannot contain `..`, absolute prefixes, or platform
    /// path separators hidden inside a segment.
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), PersistenceError> {
        let path = self.path_for(key)?;
        let parent = path
            .parent()
            .ok_or_else(|| PersistenceError::InvalidKey(key.to_owned()))?;
        fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;

        let temporary = path.with_extension("json.tmp");
        let backup = path.with_extension("json.bak");
        let data = serde_json::to_vec_pretty(value)?;

        {
            let mut file =
                fs::File::create(&temporary).map_err(|source| io_error(&temporary, source))?;
            file.write_all(&data)
                .map_err(|source| io_error(&temporary, source))?;
            file.sync_all()
                .map_err(|source| io_error(&temporary, source))?;
        }

        if path.is_file() {
            let _ = fs::remove_file(&backup);
            fs::copy(&path, &backup).map_err(|source| io_error(&backup, source))?;
        }

        replace_file(&temporary, &path)?;
        sync_directory(parent)?;
        Ok(())
    }

    /// Loads the primary object. A corrupted or partially replaced primary
    /// automatically falls back to the last valid backup.
    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, PersistenceError> {
        let path = self.path_for(key)?;
        let backup = path.with_extension("json.bak");

        match read_json(&path) {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => read_json(&backup),
            Err(primary_error) => match read_json(&backup) {
                Ok(Some(value)) => Ok(Some(value)),
                Ok(None) | Err(_) => Err(primary_error),
            },
        }
    }

    pub fn remove(&self, key: &str) -> Result<(), PersistenceError> {
        let path = self.path_for(key)?;
        for candidate in [
            path.clone(),
            path.with_extension("json.tmp"),
            path.with_extension("json.bak"),
        ] {
            if candidate.exists() {
                fs::remove_file(&candidate).map_err(|source| io_error(&candidate, source))?;
            }
        }
        Ok(())
    }

    fn path_for(&self, key: &str) -> Result<PathBuf, PersistenceError> {
        if key.trim().is_empty() {
            return Err(PersistenceError::InvalidKey(key.to_owned()));
        }

        let source = Path::new(key);
        if source.is_absolute() {
            return Err(PersistenceError::InvalidKey(key.to_owned()));
        }

        let mut relative = PathBuf::new();
        for component in source.components() {
            let Component::Normal(segment) = component else {
                return Err(PersistenceError::InvalidKey(key.to_owned()));
            };
            let segment = segment
                .to_str()
                .ok_or_else(|| PersistenceError::InvalidKey(key.to_owned()))?;
            if segment.is_empty()
                || !segment.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
            {
                return Err(PersistenceError::InvalidKey(key.to_owned()));
            }
            relative.push(segment);
        }

        if relative.as_os_str().is_empty() {
            return Err(PersistenceError::InvalidKey(key.to_owned()));
        }

        Ok(self.root.join(relative).with_extension("json"))
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, PersistenceError> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|source| io_error(path, source))?;
    Ok(Some(serde_json::from_str(&text)?))
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), PersistenceError> {
    #[cfg(windows)]
    if destination.exists() {
        fs::remove_file(destination).map_err(|error| io_error(destination, error))?;
    }

    fs::rename(source, destination).map_err(|error| io_error(destination, error))
}

fn sync_directory(path: &Path) -> Result<(), PersistenceError> {
    #[cfg(unix)]
    {
        let directory = fs::File::open(path).map_err(|source| io_error(path, source))?;
        directory
            .sync_all()
            .map_err(|source| io_error(path, source))?;
    }
    Ok(())
}

fn io_error(path: &Path, source: std::io::Error) -> PersistenceError {
    PersistenceError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde::{Deserialize, Serialize};

    use super::{JsonStore, PersistenceError};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct State {
        value: u32,
    }

    fn temp_store() -> (std::path::PathBuf, JsonStore) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("honknet-json-store-{suffix}"));
        (root.clone(), JsonStore::new(root))
    }

    #[test]
    fn saves_loads_and_recovers_from_backup() {
        let (root, store) = temp_store();
        store.save("world/main", &State { value: 1 }).unwrap();
        store.save("world/main", &State { value: 2 }).unwrap();
        assert_eq!(
            store.load::<State>("world/main").unwrap(),
            Some(State { value: 2 })
        );

        fs::write(root.join("world/main.json"), b"broken").unwrap();
        assert_eq!(
            store.load::<State>("world/main").unwrap(),
            Some(State { value: 1 })
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_path_traversal() {
        let (_root, store) = temp_store();
        assert!(matches!(
            store.save("../escape", &State { value: 1 }),
            Err(PersistenceError::InvalidKey(_))
        ));
    }
}
