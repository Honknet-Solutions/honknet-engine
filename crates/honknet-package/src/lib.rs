use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub name: String,
    pub version: Version,
    #[serde(default)]
    pub dependencies: BTreeMap<String, VersionReq>,
    pub checksum: Option<String>,
    pub signature: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub packages: BTreeMap<String, LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    pub version: Version,
    pub checksum: String,
    pub source: String,
}

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest: {0}")]
    Manifest(String),
    #[error("dependency conflict for {0}")]
    Conflict(String),
    #[error("package not found {0}")]
    NotFound(String),
    #[error("checksum mismatch {0}")]
    Checksum(String),
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
}

pub trait Registry {
    fn versions(&self, name: &str) -> Result<Vec<PackageManifest>, PackageError>;
    fn download(&self, p: &PackageManifest) -> Result<Vec<u8>, PackageError>;
}

pub struct LocalRegistry {
    pub root: PathBuf,
}

impl Registry for LocalRegistry {
    fn versions(&self, name: &str) -> Result<Vec<PackageManifest>, PackageError> {
        let d = self.root.join(name);
        if !d.is_dir() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for e in fs::read_dir(d)? {
            let p = e?.path().join("package.toml");
            if p.is_file() {
                out.push(
                    toml::from_str(&fs::read_to_string(p)?)
                        .map_err(|e| PackageError::Manifest(e.to_string()))?,
                )
            }
        }
        Ok(out)
    }
    fn download(&self, p: &PackageManifest) -> Result<Vec<u8>, PackageError> {
        fs::read(
            self.root
                .join(&p.name)
                .join(p.version.to_string())
                .join("package.hnpkg"),
        )
        .map_err(Into::into)
    }
}

pub struct HttpRegistry {
    pub base: String,
    client: reqwest::blocking::Client,
}

impl HttpRegistry {
    pub fn new(base: String) -> Self {
        Self {
            base,
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Registry for HttpRegistry {
    fn versions(&self, name: &str) -> Result<Vec<PackageManifest>, PackageError> {
        Ok(self
            .client
            .get(format!(
                "{}/v1/{name}/index",
                self.base.trim_end_matches('/')
            ))
            .send()?
            .error_for_status()?
            .json()?)
    }
    fn download(&self, p: &PackageManifest) -> Result<Vec<u8>, PackageError> {
        Ok(self
            .client
            .get(format!(
                "{}/v1/{}/{}/download",
                self.base.trim_end_matches('/'),
                p.name,
                p.version
            ))
            .send()?
            .error_for_status()?
            .bytes()?
            .to_vec())
    }
}

pub fn resolve(root: &PackageManifest, registry: &dyn Registry) -> Result<LockFile, PackageError> {
    let mut constraints: HashMap<String, Vec<VersionReq>> = HashMap::new();
    for (n, r) in &root.dependencies {
        constraints.entry(n.clone()).or_default().push(r.clone())
    }
    let mut lock = LockFile::default();
    let mut pending: Vec<String> = constraints.keys().cloned().collect();
    let mut visited = HashSet::new();
    while let Some(name) = pending.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let reqs = constraints.get(&name).cloned().unwrap_or_default();
        let mut candidates = registry.versions(&name)?;
        candidates.sort_by(|a, b| b.version.cmp(&a.version));
        let selected = candidates
            .into_iter()
            .find(|p| reqs.iter().all(|r| r.matches(&p.version)))
            .ok_or_else(|| PackageError::Conflict(name.clone()))?;
        for (n, r) in &selected.dependencies {
            constraints.entry(n.clone()).or_default().push(r.clone());
            pending.push(n.clone())
        }
        lock.packages.insert(
            name,
            LockedPackage {
                version: selected.version,
                checksum: selected.checksum.unwrap_or_default(),
                source: selected.source.unwrap_or_else(|| "registry".into()),
            },
        );
    }
    Ok(lock)
}

pub fn install(
    package: &PackageManifest,
    data: &[u8],
    cache: &Path,
) -> Result<PathBuf, PackageError> {
    let hash = hex::encode(Sha256::digest(data));
    if package.checksum.as_deref().is_some_and(|x| x != hash) {
        return Err(PackageError::Checksum(package.name.clone()));
    }
    let path = cache.join(&package.name).join(package.version.to_string());
    fs::create_dir_all(&path)?;
    fs::write(path.join("package.hnpkg"), data)?;
    fs::write(
        path.join("package.toml"),
        toml::to_string_pretty(package).map_err(|e| PackageError::Manifest(e.to_string()))?,
    )?;
    Ok(path)
}
