use bitflags::bitflags;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
pub const PROTOCOL_VERSION: u16 = 1;
pub const MTU: usize = 1200;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Channel {
    ReliableOrdered = 0,
    ReliableUnordered = 1,
    UnreliableSequenced = 2,
    Unreliable = 3,
    Control = 4,
    BulkTransfer = 5,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PacketFlags: u8 {
        const RELIABLE = 1;
        const FRAGMENT = 2;
        const COMPRESSED = 4;
        const HANDSHAKE = 8;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientHelloPayload {
    pub protocol_version: u16,
    pub engine_version: String,
    pub content_hash: String,
    pub client_id: u64,
    pub auth_token: Option<String>,
}

impl NetworkMessage for ClientHelloPayload {
    const ID: u16 = 100;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerWelcomePayload {
    pub protocol_version: u16,
    pub server_tick: u64,
    pub peer_id: u64,
    pub tick_rate: u32,
    pub session_token: String,
}

impl NetworkMessage for ServerWelcomePayload {
    const ID: u16 = 101;
}

#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub version: u16,
    pub kind: u16,
    pub channel: Channel,
    pub flags: PacketFlags,
    pub sequence: u32,
    pub ack: u32,
    pub ack_bits: u32,
    pub tick: u64,
    pub fragment_id: u16,
    pub fragment_index: u16,
    pub fragment_count: u16,
    pub payload_len: u16,
}

impl Header {
    pub const SIZE: usize = 38;
    pub fn encode(self, out: &mut Vec<u8>) {
        out.extend(self.version.to_le_bytes());
        out.extend(self.kind.to_le_bytes());
        out.push(self.channel as u8);
        out.push(self.flags.bits());
        out.extend(self.sequence.to_le_bytes());
        out.extend(self.ack.to_le_bytes());
        out.extend(self.ack_bits.to_le_bytes());
        out.extend(self.tick.to_le_bytes());
        out.extend(self.fragment_id.to_le_bytes());
        out.extend(self.fragment_index.to_le_bytes());
        out.extend(self.fragment_count.to_le_bytes());
        out.extend(self.payload_len.to_le_bytes());
        out.extend(crc32fast::hash(out).to_le_bytes());
    }
    pub fn decode(b: &[u8]) -> Result<Self, ProtocolError> {
        if b.len() < Self::SIZE {
            return Err(ProtocolError::Truncated);
        }
        let expected = u32::from_le_bytes(b[34..38].try_into().unwrap());
        if crc32fast::hash(&b[..34]) != expected {
            return Err(ProtocolError::Checksum);
        }
        let ch = match b[4] {
            0 => Channel::ReliableOrdered,
            1 => Channel::ReliableUnordered,
            2 => Channel::UnreliableSequenced,
            3 => Channel::Unreliable,
            4 => Channel::Control,
            5 => Channel::BulkTransfer,
            _ => return Err(ProtocolError::Channel),
        };
        Ok(Self {
            version: u16::from_le_bytes(b[0..2].try_into().unwrap()),
            kind: u16::from_le_bytes(b[2..4].try_into().unwrap()),
            channel: ch,
            flags: PacketFlags::from_bits_retain(b[5]),
            sequence: u32::from_le_bytes(b[6..10].try_into().unwrap()),
            ack: u32::from_le_bytes(b[10..14].try_into().unwrap()),
            ack_bits: u32::from_le_bytes(b[14..18].try_into().unwrap()),
            tick: u64::from_le_bytes(b[18..26].try_into().unwrap()),
            fragment_id: u16::from_le_bytes(b[26..28].try_into().unwrap()),
            fragment_index: u16::from_le_bytes(b[28..30].try_into().unwrap()),
            fragment_count: u16::from_le_bytes(b[30..32].try_into().unwrap()),
            payload_len: u16::from_le_bytes(b[32..34].try_into().unwrap()),
        })
    }
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("truncated packet")]
    Truncated,
    #[error("header checksum")]
    Checksum,
    #[error("invalid channel")]
    Channel,
    #[error("protocol mismatch")]
    Version,
    #[error("payload codec: {0}")]
    Codec(String),
    #[error("decompression limit exceeded")]
    DecompressionLimit,
}

pub const fn const_message_id(name: &str) -> u16 {
    let b = name.as_bytes();
    let mut h = 0x811cu16;
    let mut i = 0;
    while i < b.len() {
        h = (h ^ b[i] as u16).wrapping_mul(0x0193);
        i += 1
    }
    h
}

pub trait NetworkMessage: Serialize + DeserializeOwned + Send + Sync + 'static {
    const ID: u16;
}

pub fn encode_message<T: NetworkMessage>(
    m: &T,
    compress: bool,
) -> Result<(u16, Vec<u8>, bool), ProtocolError> {
    let raw = bincode::serde::encode_to_vec(m, bincode::config::standard())
        .map_err(|e| ProtocolError::Codec(e.to_string()))?;
    if compress && raw.len() > 256 {
        let z = zstd::bulk::compress(&raw, 3).map_err(|e| ProtocolError::Codec(e.to_string()))?;
        if z.len() < raw.len() {
            return Ok((T::ID, z, true));
        }
    }
    Ok((T::ID, raw, false))
}

pub fn decode_message<T: NetworkMessage>(
    bytes: &[u8],
    compressed: bool,
    limit: usize,
) -> Result<T, ProtocolError> {
    let raw = if compressed {
        zstd::bulk::decompress(bytes, limit).map_err(|_| ProtocolError::DecompressionLimit)?
    } else {
        bytes.to_vec()
    };
    let (v, _) = bincode::serde::decode_from_slice(&raw, bincode::config::standard())
        .map_err(|e| ProtocolError::Codec(e.to_string()))?;
    Ok(v)
}

pub fn acked(sequence: u32, ack: u32, bits: u32) -> bool {
    if sequence == ack {
        return true;
    }
    let d = ack.wrapping_sub(sequence);
    d > 0 && d <= 32 && (bits & (1 << (d - 1))) != 0
}

pub fn update_ack(latest: &mut u32, bits: &mut u32, seq: u32) {
    if seq > *latest {
        let shift = seq - *latest;
        *bits = if shift >= 32 {
            0
        } else {
            (*bits << shift) | 1 << (shift - 1)
        };
        *latest = seq
    } else {
        let d = *latest - seq;
        if d > 0 && d <= 32 {
            *bits |= 1 << (d - 1)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn fragment(
    kind: u16,
    channel: Channel,
    flags: PacketFlags,
    sequence: u32,
    ack: u32,
    ack_bits: u32,
    tick: u64,
    payload: &[u8],
    fragment_id: u16,
) -> Vec<Vec<u8>> {
    let cap = MTU - Header::SIZE;
    let count = payload.len().div_ceil(cap).max(1);
    payload
        .chunks(cap)
        .enumerate()
        .map(|(i, c)| {
            let mut out = Vec::with_capacity(Header::SIZE + c.len());
            Header {
                version: PROTOCOL_VERSION,
                kind,
                channel,
                flags: flags
                    | if count > 1 {
                        PacketFlags::FRAGMENT
                    } else {
                        PacketFlags::empty()
                    },
                sequence,
                ack,
                ack_bits,
                tick,
                fragment_id,
                fragment_index: i as u16,
                fragment_count: count as u16,
                payload_len: c.len() as u16,
            }
            .encode(&mut out);
            out.extend(c);
            out
        })
        .collect()
}

#[derive(Default)]
pub struct FragmentAssembler {
    sets: std::collections::HashMap<(u32, u16), Vec<Option<Vec<u8>>>>,
}

impl FragmentAssembler {
    pub fn push(&mut self, h: Header, payload: &[u8]) -> Option<Vec<u8>> {
        if !h.flags.contains(PacketFlags::FRAGMENT) {
            return Some(payload.to_vec());
        }
        let key = (h.sequence, h.fragment_id);
        let v = self
            .sets
            .entry(key)
            .or_insert_with(|| vec![None; h.fragment_count as usize]);
        if let Some(slot) = v.get_mut(h.fragment_index as usize) {
            *slot = Some(payload.to_vec())
        }
        if v.iter().all(Option::is_some) {
            let mut out = vec![];
            for x in v.iter_mut() {
                out.extend(x.take().unwrap())
            }
            self.sets.remove(&key);
            Some(out)
        } else {
            None
        }
    }
}
