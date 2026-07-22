use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
const MAGIC: [u8; 4] = *b"HNK1";
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("truncated data")]
    Truncated,
    #[error("bad magic")]
    BadMagic,
    #[error("checksum mismatch")]
    Checksum,
    #[error("codec: {0}")]
    Codec(String),
}

pub fn encode_versioned<T: Serialize>(version: u32, value: &T) -> Result<Vec<u8>, CodecError> {
    let payload = bincode::serde::encode_to_vec(value, bincode::config::standard())
        .map_err(|e| CodecError::Codec(e.to_string()))?;
    let mut out = Vec::with_capacity(payload.len() + 16);
    out.extend(MAGIC);
    out.extend(version.to_le_bytes());
    out.extend((payload.len() as u32).to_le_bytes());
    out.extend(crc32fast::hash(&payload).to_le_bytes());
    out.extend(payload);
    Ok(out)
}

pub fn decode_versioned<T: DeserializeOwned>(bytes: &[u8]) -> Result<(u32, T), CodecError> {
    if bytes.len() < 16 {
        return Err(CodecError::Truncated);
    }
    if bytes[0..4] != MAGIC {
        return Err(CodecError::BadMagic);
    }
    let v = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let crc = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let payload = bytes.get(16..16 + len).ok_or(CodecError::Truncated)?;
    if crc32fast::hash(payload) != crc {
        return Err(CodecError::Checksum);
    }
    let (value, _) = bincode::serde::decode_from_slice(payload, bincode::config::standard())
        .map_err(|e| CodecError::Codec(e.to_string()))?;
    Ok((v, value))
}

pub fn put_var_u64(mut v: u64, out: &mut Vec<u8>) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8)
}

pub fn get_var_u64(input: &mut &[u8]) -> Result<u64, CodecError> {
    let mut v = 0u64;
    for shift in (0..64).step_by(7) {
        let b = *input.first().ok_or(CodecError::Truncated)?;
        *input = &input[1..];
        v |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err(CodecError::Codec("varint overflow".into()))
}

pub struct BitWriter {
    bytes: Vec<u8>,
    bit: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit: 0,
        }
    }
    pub fn push(&mut self, value: bool) {
        if self.bit == 0 {
            self.bytes.push(0)
        }
        if value {
            *self.bytes.last_mut().unwrap() |= 1 << self.bit
        }
        self.bit = (self.bit + 1) % 8
    }
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct X {
        a: u32,
    }
    #[test]
    fn envelope_roundtrip() {
        let b = encode_versioned(3, &X { a: 7 }).unwrap();
        assert_eq!(decode_versioned::<X>(&b).unwrap(), (3, X { a: 7 }));
    }
}
