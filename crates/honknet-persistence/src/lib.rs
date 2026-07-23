use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQL: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("codec: {0}")]
    Codec(String),
    #[error("checksum mismatch")]
    Checksum,
}

pub trait PersistenceBackend: Send + Sync {
    fn load(&self, world: &str) -> Result<Option<Vec<u8>>, PersistenceError>;
    fn commit(&self, world: &str, data: &[u8]) -> Result<u64, PersistenceError>;
    fn checkpoint(&self, world: &str) -> Result<String, PersistenceError>;
    fn load_region(&self, world: &str, region: &str) -> Result<Option<Vec<u8>>, PersistenceError> {
        self.load(&format!("{world}--{region}"))
    }
}

#[derive(Clone)]
pub struct FileBackend {
    root: PathBuf,
    lock: Arc<Mutex<()>>,
    backups: usize,
}

impl FileBackend {
    pub fn new(root: impl Into<PathBuf>, backups: usize) -> Self {
        Self {
            root: root.into(),
            lock: Default::default(),
            backups: backups.max(1),
        }
    }
    fn dir(&self, w: &str) -> PathBuf {
        self.root.join(safe(w))
    }
}

impl PersistenceBackend for FileBackend {
    fn load(&self, w: &str) -> Result<Option<Vec<u8>>, PersistenceError> {
        let p = self.dir(w).join("checkpoint.bin");
        if !p.is_file() {
            return Ok(None);
        }
        let b = fs::read(&p)?;
        verify_blob(&b).map(Some)
    }
    fn commit(&self, w: &str, data: &[u8]) -> Result<u64, PersistenceError> {
        let _g = self.lock.lock();
        let d = self.dir(w);
        fs::create_dir_all(&d)?;
        let seq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let blob = make_blob(data);
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(d.join("journal.log"))?;
        f.write_all(&(blob.len() as u64).to_le_bytes())?;
        f.write_all(&seq.to_le_bytes())?;
        f.write_all(&blob)?;
        f.sync_all()?;
        Ok(seq)
    }
    fn checkpoint(&self, w: &str) -> Result<String, PersistenceError> {
        let _g = self.lock.lock();
        let d = self.dir(w);
        fs::create_dir_all(&d)?;
        let journal = d.join("journal.log");
        let mut latest = None;
        if journal.is_file() {
            let b = fs::read(&journal)?;
            let mut p = 0;
            while p + 16 <= b.len() {
                let len = u64::from_le_bytes(b[p..p + 8].try_into().unwrap()) as usize;
                p += 16;
                if p + len > b.len() {
                    break;
                }
                latest = Some(verify_blob(&b[p..p + len])?);
                p += len
            }
        }
        let data = latest
            .or_else(|| self.load(w).ok().flatten())
            .unwrap_or_default();
        for i in (1..self.backups).rev() {
            let from = d.join(format!("checkpoint.{}.bin", i - 1));
            let to = d.join(format!("checkpoint.{i}.bin"));
            if from.exists() {
                let _ = fs::rename(from, to);
            }
        }
        let main = d.join("checkpoint.bin");
        if main.exists() {
            let _ = fs::rename(&main, d.join("checkpoint.0.bin"));
        }
        let tmp = d.join("checkpoint.tmp");
        fs::write(&tmp, make_blob(&data))?;
        OpenOptions::new().read(true).open(&tmp)?.sync_all()?;
        fs::rename(tmp, &main)?;
        if journal.exists() {
            fs::remove_file(journal)?
        }
        Ok(hex::encode(Sha256::digest(&data)))
    }
}

pub struct SqliteBackend {
    conn: Mutex<Connection>,
}

impl SqliteBackend {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let c = Connection::open(path)?;
        c.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS worlds(key TEXT PRIMARY KEY, revision INTEGER NOT NULL, data BLOB NOT NULL, checksum TEXT NOT NULL);")?;
        Ok(Self {
            conn: Mutex::new(c),
        })
    }
}

impl PersistenceBackend for SqliteBackend {
    fn load(&self, w: &str) -> Result<Option<Vec<u8>>, PersistenceError> {
        let c = self.conn.lock();
        let mut s = c.prepare("SELECT data, checksum FROM worlds WHERE key=?1")?;
        let mut rows = s.query(params![w])?;
        if let Some(r) = rows.next()? {
            let b: Vec<u8> = r.get(0)?;
            let h: String = r.get(1)?;
            if hex::encode(Sha256::digest(&b)) != h {
                return Err(PersistenceError::Checksum);
            }
            Ok(Some(b))
        } else {
            Ok(None)
        }
    }
    fn commit(&self, w: &str, data: &[u8]) -> Result<u64, PersistenceError> {
        let c = self.conn.lock();
        let rev = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        c.execute("INSERT INTO worlds(key, revision, data, checksum) VALUES(?1,?2,?3,?4) ON CONFLICT(key) DO UPDATE SET revision=excluded.revision, data=excluded.data, checksum=excluded.checksum",
        params![w, rev as i64, data, hex::encode(Sha256::digest(data))])?;
        Ok(rev)
    }
    fn checkpoint(&self, w: &str) -> Result<String, PersistenceError> {
        let b = self.load(w)?.unwrap_or_default();
        Ok(hex::encode(Sha256::digest(b)))
    }
}

pub trait PostgresConnection: Send + Sync {
    fn execute_transaction(
        &self,
        world: &str,
        revision: u64,
        data: &[u8],
        checksum: &str,
    ) -> Result<(), PersistenceError>;
    fn query_world(&self, world: &str) -> Result<Option<Vec<u8>>, PersistenceError>;
}

fn make_blob(data: &[u8]) -> Vec<u8> {
    let mut v = Sha256::digest(data).to_vec();
    v.extend(data);
    v
}

fn verify_blob(b: &[u8]) -> Result<Vec<u8>, PersistenceError> {
    if b.len() < 32 {
        return Err(PersistenceError::Checksum);
    }
    let d = &b[32..];
    if Sha256::digest(d).as_slice() != &b[..32] {
        return Err(PersistenceError::Checksum);
    }
    Ok(d.to_vec())
}

fn safe(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn save_typed<T: Serialize>(
    b: &dyn PersistenceBackend,
    w: &str,
    v: &T,
) -> Result<u64, PersistenceError> {
    let d = serde_json::to_vec(v).map_err(|e| PersistenceError::Codec(e.to_string()))?;
    b.commit(w, &d)
}

pub fn load_typed<T: DeserializeOwned>(
    b: &dyn PersistenceBackend,
    w: &str,
) -> Result<Option<T>, PersistenceError> {
    b.load(w)?
        .map(|d| serde_json::from_slice(&d).map_err(|e| PersistenceError::Codec(e.to_string())))
        .transpose()
}
