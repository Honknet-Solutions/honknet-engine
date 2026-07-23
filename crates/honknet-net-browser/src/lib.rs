// Browser transport definitions

pub struct TransportEvent; // placeholder
pub struct TransportStats; // placeholder

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
    // Basic fields
    socket: Option<web_sys::WebSocket>,
}

#[cfg(target_arch = "wasm32")]
impl BrowserTransport {
    pub fn new() -> Self {
        Self { socket: None }
    }
}

#[cfg(target_arch = "wasm32")]
impl ClientTransport for BrowserTransport {
    fn connect(&mut self, endpoint: &str) {
        if let Ok(ws) = web_sys::WebSocket::new(endpoint) {
            ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
            self.socket = Some(ws);
        }
    }

    fn disconnect(&mut self) {
        if let Some(ws) = &self.socket {
            let _ = ws.close();
            self.socket = None;
        }
    }

    fn poll(&mut self) -> Vec<TransportEvent> {
        vec![] // placeholder
    }

    fn send_reliable(&mut self, bytes: &[u8]) {
        if let Some(ws) = &self.socket {
            if ws.ready_state() == web_sys::WebSocket::OPEN {
                let _ = ws.send_with_u8_array(bytes);
            }
        }
    }

    fn send_unreliable(&mut self, bytes: &[u8]) {
        self.send_reliable(bytes);
    }

    fn stats(&self) -> TransportStats {
        TransportStats
    }
}
