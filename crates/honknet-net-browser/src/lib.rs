use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportEvent {
    Connected,
    Disconnected(String),
    Data(Vec<u8>),
    Error(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransportStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub rtt_ms: f32,
}

pub trait ClientTransport {
    fn connect(&mut self, endpoint: &str);
    fn disconnect(&mut self);
    fn poll(&mut self) -> Vec<TransportEvent>;
    fn send_reliable(&mut self, bytes: &[u8]);
    fn send_unreliable(&mut self, bytes: &[u8]);
    fn stats(&self) -> TransportStats;
}

#[cfg(target_arch = "wasm32")]
pub struct BrowserTransport {
    socket: Option<web_sys::WebSocket>,
    incoming: VecDeque<TransportEvent>,
    stats: TransportStats,
}

#[cfg(target_arch = "wasm32")]
impl BrowserTransport {
    pub fn new() -> Self {
        Self {
            socket: None,
            incoming: VecDeque::new(),
            stats: TransportStats::default(),
        }
    }

    pub fn push_incoming(&mut self, event: TransportEvent) {
        if let TransportEvent::Data(ref d) = event {
            self.stats.bytes_received += d.len() as u64;
            self.stats.packets_received += 1;
        }
        self.incoming.push_back(event);
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for BrowserTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl ClientTransport for BrowserTransport {
    fn connect(&mut self, endpoint: &str) {
        if let Ok(ws) = web_sys::WebSocket::new(endpoint) {
            ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
            self.socket = Some(ws);
            self.incoming.push_back(TransportEvent::Connected);
        } else {
            self.incoming
                .push_back(TransportEvent::Error("Failed to create WebSocket".into()));
        }
    }

    fn disconnect(&mut self) {
        if let Some(ws) = &self.socket {
            let _ = ws.close();
            self.socket = None;
            self.incoming
                .push_back(TransportEvent::Disconnected("Closed by client".into()));
        }
    }

    fn poll(&mut self) -> Vec<TransportEvent> {
        self.incoming.drain(..).collect()
    }

    fn send_reliable(&mut self, bytes: &[u8]) {
        if let Some(ws) = &self.socket {
            if ws.ready_state() == web_sys::WebSocket::OPEN {
                if ws.send_with_u8_array(bytes).is_ok() {
                    self.stats.bytes_sent += bytes.len() as u64;
                    self.stats.packets_sent += 1;
                }
            }
        }
    }

    fn send_unreliable(&mut self, bytes: &[u8]) {
        self.send_reliable(bytes);
    }

    fn stats(&self) -> TransportStats {
        self.stats.clone()
    }
}
