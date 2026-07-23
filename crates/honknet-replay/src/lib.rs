use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
use thiserror::Error;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    pub engine_version: String,
    pub protocol: u16,
    pub content_hash: String,
    pub initial_state: Vec<u8>,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplayEvent {
    Input {
        tick: u64,
        client: u64,
        data: Vec<u8>,
    },
    Replicated {
        tick: u64,
        data: Vec<u8>,
    },
    RandomSeed {
        tick: u64,
        stream: u64,
        seed: u64,
    },
    Marker {
        tick: u64,
        name: String,
    },
}

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("codec: {0}")]
    Codec(String),
    #[error("bad replay")]
    Bad,
}

pub struct ReplayRecorder {
    file: File,
    index: Vec<(u64, u64)>,
}

impl ReplayRecorder {
    pub fn create(path: &Path, h: &ReplayHeader) -> Result<Self, ReplayError> {
        let mut f = File::create(path)?;
        f.write_all(b"HNRP")?;
        write_record(&mut f, h)?;
        Ok(Self {
            file: f,
            index: vec![],
        })
    }
    pub fn push(&mut self, e: &ReplayEvent) -> Result<(), ReplayError> {
        let tick = match e {
            ReplayEvent::Input { tick, .. }
            | ReplayEvent::Replicated { tick, .. }
            | ReplayEvent::RandomSeed { tick, .. }
            | ReplayEvent::Marker { tick, .. } => *tick,
        };
        let pos = self.file.stream_position()?;
        self.index.push((tick, pos));
        write_record(&mut self.file, e)
    }
    pub fn finish(mut self) -> Result<(), ReplayError> {
        let pos = self.file.stream_position()?;
        write_record(&mut self.file, &self.index)?;
        self.file.write_all(&pos.to_le_bytes())?;
        self.file.sync_all()?;
        Ok(())
    }
}

pub struct ReplayReader {
    file: File,
    pub header: ReplayHeader,
    index: Vec<(u64, u64)>,
    events_start: u64,
    events_end: u64,
}

impl ReplayReader {
    pub fn open(path: &Path) -> Result<Self, ReplayError> {
        let mut f = File::open(path)?;
        let mut m = [0; 4];
        f.read_exact(&mut m)?;
        if &m != b"HNRP" {
            return Err(ReplayError::Bad);
        }
        let h = read_record(&mut f)?;
        let start = f.stream_position()?;
        f.seek(SeekFrom::End(-8))?;
        let mut b = [0; 8];
        f.read_exact(&mut b)?;
        let pos = u64::from_le_bytes(b);
        f.seek(SeekFrom::Start(pos))?;
        let index = read_record(&mut f)?;
        f.seek(SeekFrom::Start(start))?;
        Ok(Self {
            file: f,
            header: h,
            index,
            events_start: start,
            events_end: pos,
        })
    }
    pub fn seek_tick(&mut self, tick: u64) -> Result<(), ReplayError> {
        let pos = self
            .index
            .iter()
            .rev()
            .find(|(t, _)| *t <= tick)
            .map(|x| x.1)
            .unwrap_or(self.events_start);
        self.file.seek(SeekFrom::Start(pos))?;
        Ok(())
    }
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<ReplayEvent>, ReplayError> {
        let p = self.file.stream_position()?;
        let end = self.events_end;
        if p >= end {
            return Ok(None);
        }
        read_record(&mut self.file).map(Some)
    }
    pub fn verify_determinism<F: FnMut(&ReplayEvent) -> Vec<u8>>(
        &mut self,
        mut apply: F,
    ) -> Result<String, ReplayError> {
        self.file.seek(SeekFrom::Start(self.events_start))?;
        let mut h = Sha256::new();
        while let Some(e) = self.next()? {
            h.update(apply(&e))
        }
        Ok(hex::encode(h.finalize()))
    }
}

fn write_record<T: Serialize>(f: &mut File, v: &T) -> Result<(), ReplayError> {
    let b = bincode::serde::encode_to_vec(v, bincode::config::standard())
        .map_err(|e| ReplayError::Codec(e.to_string()))?;
    f.write_all(&(b.len() as u32).to_le_bytes())?;
    f.write_all(&b)?;
    Ok(())
}

fn read_record<T: for<'a> Deserialize<'a>>(f: &mut File) -> Result<T, ReplayError> {
    let mut l = [0; 4];
    f.read_exact(&mut l)?;
    let mut b = vec![0; u32::from_le_bytes(l) as usize];
    f.read_exact(&mut b)?;
    bincode::serde::decode_from_slice(&b, bincode::config::standard())
        .map(|x| x.0)
        .map_err(|e| ReplayError::Codec(e.to_string()))
}
