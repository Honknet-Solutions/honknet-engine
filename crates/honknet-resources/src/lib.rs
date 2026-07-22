use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("hash mismatch for {0}")]
    Hash(String),
    #[error("invalid path")]
    Path,
}

pub trait Mount: Send + Sync {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, ResourceError>;
    fn list(&self, prefix: &str) -> Result<Vec<String>, ResourceError>;
}

pub struct DirectoryMount {
    root: PathBuf,
}

impl DirectoryMount {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    fn resolve(&self, path: &str) -> Result<PathBuf, ResourceError> {
        if path.contains("..") || path.starts_with('/') || path.contains('\\') {
            return Err(ResourceError::Path);
        }
        Ok(self.root.join(path))
    }
}

impl Mount for DirectoryMount {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, ResourceError> {
        let p = self.resolve(path)?;
        if p.is_file() {
            Ok(Some(std::fs::read(p)?))
        } else {
            Ok(None)
        }
    }
    fn list(&self, prefix: &str) -> Result<Vec<String>, ResourceError> {
        let root = self.resolve(prefix)?;
        if !root.exists() {
            return Ok(vec![]);
        }
        Ok(walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| {
                entry
                    .path()
                    .strip_prefix(&self.root)
                    .ok()
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
            })
            .collect())
    }
}

#[derive(Default)]
pub struct MemoryMount {
    files: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryMount {
    pub fn insert(&self, path: impl Into<String>, data: Vec<u8>) {
        self.files.write().insert(path.into(), data);
    }
}

impl Mount for MemoryMount {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, ResourceError> {
        Ok(self.files.read().get(path).cloned())
    }
    fn list(&self, prefix: &str) -> Result<Vec<String>, ResourceError> {
        Ok(self
            .files
            .read()
            .keys()
            .filter(|p| p.starts_with(prefix))
            .cloned()
            .collect())
    }
}

pub struct ArchiveMount {
    bytes: Arc<Vec<u8>>,
}

impl ArchiveMount {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ResourceError> {
        Ok(Self {
            bytes: Arc::new(std::fs::read(path)?),
        })
    }
}

impl Mount for ArchiveMount {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, ResourceError> {
        let mut z = zip::ZipArchive::new(Cursor::new(self.bytes.as_slice()))?;
        let res = match z.by_name(path) {
            Ok(mut f) => {
                let mut b = vec![];
                f.read_to_end(&mut b)?;
                Ok(Some(b))
            }
            Err(zip::result::ZipError::FileNotFound) => Ok(None),
            Err(e) => Err(e.into()),
        };
        res
    }
    fn list(&self, prefix: &str) -> Result<Vec<String>, ResourceError> {
        let mut z = zip::ZipArchive::new(Cursor::new(self.bytes.as_slice()))?;
        let mut out = vec![];
        for i in 0..z.len() {
            let f = z.by_index(i)?;
            if !f.is_dir() && f.name().starts_with(prefix) {
                out.push(f.name().into())
            }
        }
        Ok(out)
    }
}

pub struct RemoteCacheMount {
    base: String,
    cache: PathBuf,
    client: reqwest::blocking::Client,
}

impl RemoteCacheMount {
    pub fn new(base: impl Into<String>, cache: impl Into<PathBuf>) -> Self {
        Self {
            base: base.into(),
            cache: cache.into(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Mount for RemoteCacheMount {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, ResourceError> {
        if path.contains("..") {
            return Err(ResourceError::Path);
        }
        let local = self.cache.join(path);
        if local.is_file() {
            return Ok(Some(std::fs::read(local)?));
        }
        let url = format!("{}/{}", self.base.trim_end_matches('/'), path);
        let r = self.client.get(url).send()?;
        if r.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let b = r.error_for_status()?.bytes()?.to_vec();
        if let Some(p) = local.parent() {
            std::fs::create_dir_all(p)?
        }
        std::fs::write(local, &b)?;
        Ok(Some(b))
    }
    fn list(&self, _: &str) -> Result<Vec<String>, ResourceError> {
        Ok(vec![])
    }
}

#[derive(Default, Clone)]
pub struct Vfs {
    mounts: Arc<RwLock<Vec<Arc<dyn Mount>>>>,
}

impl Vfs {
    pub fn mount<M: Mount + 'static>(&self, m: M) {
        self.mounts.write().push(Arc::new(m));
    }
    pub fn read(&self, uri: &str) -> Result<Vec<u8>, ResourceError> {
        let path = uri.split_once("://").map(|x| x.1).unwrap_or(uri);
        for m in self.mounts.read().iter().rev() {
            if let Some(b) = m.read(path)? {
                return Ok(b);
            }
        }
        Err(ResourceError::NotFound(uri.into()))
    }
    pub fn list(&self, prefix: &str) -> Result<Vec<String>, ResourceError> {
        let mut v = vec![];
        for m in self.mounts.read().iter() {
            v.extend(m.list(prefix)?)
        }
        v.sort();
        v.dedup();
        Ok(v)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceManifest {
    pub version: u32,
    pub entries: Vec<ManifestEntry>,
    pub signature: Option<String>,
}

impl ResourceManifest {
    pub fn build(root: &Path, package: &str) -> Result<Self, ResourceError> {
        let mut entries = vec![];
        for e in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let b = std::fs::read(e.path())?;
            entries.push(ManifestEntry {
                path: e
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                size: b.len() as u64,
                sha256: hex::encode(Sha256::digest(&b)),
                package: package.into(),
            })
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self {
            version: 1,
            entries,
            signature: None,
        })
    }
    pub fn verify(&self, root: &Path) -> Result<(), ResourceError> {
        for e in &self.entries {
            let b = std::fs::read(root.join(&e.path))?;
            if hex::encode(Sha256::digest(&b)) != e.sha256 {
                return Err(ResourceError::Hash(e.path.clone()));
            }
        }
        Ok(())
    }
}

impl ResourceManifest {
    pub fn sign_hmac(&mut self, key: &[u8]) {
        use hmac::{Hmac, Mac};
        let mut clone = self.clone();
        clone.signature = None;
        let bytes = serde_json::to_vec(&clone).unwrap_or_default();
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("valid HMAC key");
        mac.update(&bytes);
        self.signature = Some(hex::encode(mac.finalize().into_bytes()));
    }
    pub fn verify_hmac(&self, key: &[u8]) -> bool {
        use hmac::{Hmac, Mac};
        let Some(signature) = &self.signature else {
            return false;
        };
        let Ok(sig) = hex::decode(signature) else {
            return false;
        };
        let mut clone = self.clone();
        clone.signature = None;
        let bytes = serde_json::to_vec(&clone).unwrap_or_default();
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(key) else {
            return false;
        };
        mac.update(&bytes);
        mac.verify_slice(&sig).is_ok()
    }
}
