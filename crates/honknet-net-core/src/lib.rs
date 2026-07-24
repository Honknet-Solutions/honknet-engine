use bitflags::bitflags;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
pub const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CONTENT_VERSION: &str = BUILD_VERSION;
pub const CONTENT_MANIFEST_ID: &str = concat!("honknet-", env!("CARGO_PKG_VERSION"));
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

pub const ENVELOPE_HEADER_SIZE: usize = 17;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NetworkPacketEnvelope {
    pub protocol_version: u16,
    pub message_id: u16,
    pub flags: u8,
    pub tick: u64,
    pub payload_len: u32,
}

impl NetworkPacketEnvelope {
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend(self.protocol_version.to_le_bytes());
        out.extend(self.message_id.to_le_bytes());
        out.push(self.flags);
        out.extend(self.tick.to_le_bytes());
        out.extend(self.payload_len.to_le_bytes());
    }

    pub fn decode(bytes: &[u8]) -> Result<(Self, &[u8]), ProtocolError> {
        if bytes.len() < ENVELOPE_HEADER_SIZE {
            return Err(ProtocolError::Truncated);
        }
        let protocol_version = u16::from_le_bytes(bytes[0..2].try_into().unwrap());
        let message_id = u16::from_le_bytes(bytes[2..4].try_into().unwrap());
        let flags = bytes[4];
        let tick = u64::from_le_bytes(bytes[5..13].try_into().unwrap());
        let payload_len = u32::from_le_bytes(bytes[13..17].try_into().unwrap()) as usize;

        if bytes.len() < ENVELOPE_HEADER_SIZE + payload_len {
            return Err(ProtocolError::Truncated);
        }
        let payload = &bytes[ENVELOPE_HEADER_SIZE..ENVELOPE_HEADER_SIZE + payload_len];

        Ok((
            Self {
                protocol_version,
                message_id,
                flags,
                tick,
                payload_len: payload_len as u32,
            },
            payload,
        ))
    }
}

pub fn wrap_envelope(message_id: u16, flags: u8, tick: u64, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(ENVELOPE_HEADER_SIZE + payload.len());
    let env = NetworkPacketEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_id,
        flags,
        tick,
        payload_len: payload.len() as u32,
    };
    env.encode(&mut out);
    out.extend(payload);
    out
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientHelloPayload {
    pub protocol_version: u16,
    pub engine_version: String,
    pub content_version: String,
    pub content_manifest_hash: String,
    pub supported_compression: Vec<String>,
    pub auth_token: Option<String>,
    pub reconnect_token: Option<String>,
}

impl NetworkMessage for ClientHelloPayload {
    const ID: u16 = 100;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerWelcomePayload {
    pub protocol_version: u16,
    pub engine_version: String,
    pub content_version: String,
    pub content_manifest_hash: String,
    pub auth_token: Option<String>,
    pub reconnect_token: Option<String>,
    pub server_tick: u64,
    pub peer_id: u64,
    pub tick_rate: u32,
    pub session_token: String,
}

impl NetworkMessage for ServerWelcomePayload {
    const ID: u16 = 101;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientInputPayload {
    pub sequence: u32,
    pub movement: honknet_math::Vec2,
}

impl NetworkMessage for ClientInputPayload {
    const ID: u16 = 102;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateAckPayload {
    pub acked_tick: u64,
}

impl NetworkMessage for StateAckPayload {
    const ID: u16 = 103;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConnectionState {
    Disconnected,
    TransportConnecting,
    ProtocolHello,
    ProtocolNegotiation,
    Authenticating,
    LoadingManifest,
    SynchronizingWorld,
    Active,
    Reconnecting,
    Closed,
    Failed,
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
    #[error("payload too large")]
    PayloadTooLarge,
    #[error("fragment limit exceeded")]
    FragmentLimitExceeded,
    #[error("rate limit exceeded")]
    RateLimitExceeded,
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

pub fn encode_message_envelope<T: NetworkMessage>(
    m: &T,
    tick: u64,
    compress: bool,
) -> Result<Vec<u8>, ProtocolError> {
    let (id, payload, compressed) = encode_message(m, compress)?;
    let flags = if compressed { 4 } else { 0 };
    Ok(wrap_envelope(id, flags, tick, &payload))
}

pub fn decode_message<T: NetworkMessage>(
    bytes: &[u8],
    compressed: bool,
    limit: usize,
) -> Result<T, ProtocolError> {
    if bytes.len() > limit {
        return Err(ProtocolError::PayloadTooLarge);
    }
    let raw = if compressed {
        zstd::bulk::decompress(bytes, limit).map_err(|_| ProtocolError::DecompressionLimit)?
    } else {
        bytes.to_vec()
    };
    if raw.len() > limit {
        return Err(ProtocolError::PayloadTooLarge);
    }
    let (v, _) = bincode::serde::decode_from_slice(&raw, bincode::config::standard())
        .map_err(|e| ProtocolError::Codec(e.to_string()))?;
    Ok(v)
}

pub fn decode_message_envelope<T: NetworkMessage>(
    bytes: &[u8],
    limit: usize,
) -> Result<(NetworkPacketEnvelope, T), ProtocolError> {
    let (env, payload) = NetworkPacketEnvelope::decode(bytes)?;
    if env.message_id != T::ID {
        return Err(ProtocolError::Codec(format!(
            "Message ID mismatch: expected {}, got {}",
            T::ID,
            env.message_id
        )));
    }
    let compressed = (env.flags & 4) != 0;
    let msg = decode_message::<T>(payload, compressed, limit)?;
    Ok((env, msg))
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

pub struct FragmentSet {
    pub fragments: Vec<Option<Vec<u8>>>,
    pub received_count: usize,
    pub total_size: usize,
    pub last_updated: std::time::Instant,
}

pub struct FragmentAssembler {
    pub sets: std::collections::HashMap<(u32, u16), FragmentSet>,
    pub max_concurrent_sets: usize,
    pub max_fragments: u16,
    pub memory_budget: usize,
    pub current_memory: usize,
    pub timeout: std::time::Duration,
}

impl Default for FragmentAssembler {
    fn default() -> Self {
        Self {
            sets: std::collections::HashMap::new(),
            max_concurrent_sets: 64,
            max_fragments: 256,
            memory_budget: 1024 * 1024 * 8, // 8MB
            current_memory: 0,
            timeout: std::time::Duration::from_secs(5),
        }
    }
}

impl FragmentAssembler {
    pub fn push(&mut self, h: Header, payload: &[u8]) -> Result<Option<Vec<u8>>, ProtocolError> {
        if !h.flags.contains(PacketFlags::FRAGMENT) {
            return Ok(Some(payload.to_vec()));
        }
        if h.fragment_count > self.max_fragments {
            return Err(ProtocolError::FragmentLimitExceeded);
        }

        let now = std::time::Instant::now();
        self.cleanup(now);

        let key = (h.sequence, h.fragment_id);

        if !self.sets.contains_key(&key) {
            if self.sets.len() >= self.max_concurrent_sets {
                return Err(ProtocolError::FragmentLimitExceeded);
            }
            if self.current_memory + payload.len() > self.memory_budget {
                return Err(ProtocolError::FragmentLimitExceeded);
            }
            self.sets.insert(
                key,
                FragmentSet {
                    fragments: vec![None; h.fragment_count as usize],
                    received_count: 0,
                    total_size: 0,
                    last_updated: now,
                },
            );
        }

        let set = self.sets.get_mut(&key).unwrap();
        set.last_updated = now;

        if let Some(slot) = set.fragments.get_mut(h.fragment_index as usize) {
            if slot.is_none() {
                if self.current_memory + payload.len() > self.memory_budget {
                    return Err(ProtocolError::FragmentLimitExceeded);
                }
                *slot = Some(payload.to_vec());
                set.received_count += 1;
                set.total_size += payload.len();
                self.current_memory += payload.len();
            }
        }

        if set.received_count == h.fragment_count as usize {
            let mut out = Vec::with_capacity(set.total_size);
            for x in &mut set.fragments {
                out.extend(x.take().unwrap());
            }
            self.current_memory -= set.total_size;
            self.sets.remove(&key);
            Ok(Some(out))
        } else {
            Ok(None)
        }
    }

    pub fn cleanup(&mut self, now: std::time::Instant) {
        let timeout = self.timeout;
        self.sets.retain(|_, set| {
            if now.duration_since(set.last_updated) > timeout {
                self.current_memory -= set.total_size;
                false
            } else {
                true
            }
        });
    }
}

pub struct RateLimiter {
    pub last_requests: std::collections::VecDeque<std::time::Instant>,
    pub max_requests: usize,
    pub window: std::time::Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: std::time::Duration) -> Self {
        Self {
            last_requests: std::collections::VecDeque::with_capacity(max_requests),
            max_requests,
            window,
        }
    }
    pub fn check(&mut self) -> Result<(), ProtocolError> {
        let now = std::time::Instant::now();
        while let Some(&t) = self.last_requests.front() {
            if now.duration_since(t) > self.window {
                self.last_requests.pop_front();
            } else {
                break;
            }
        }
        if self.last_requests.len() >= self.max_requests {
            Err(ProtocolError::RateLimitExceeded)
        } else {
            self.last_requests.push_back(now);
            Ok(())
        }
    }
}
