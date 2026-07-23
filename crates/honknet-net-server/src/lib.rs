use honknet_net_core::*;
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;

#[derive(Default)]
pub struct ServerMetrics {
    pub active_connections: usize,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Default)]
pub struct WsServer {
    pub metrics: ServerMetrics,
    pub clients: HashMap<u64, ClientConnection>,
}

pub struct ClientConnection {
    pub id: u64,
    pub state: ConnectionState,
    pub send_queue: VecDeque<Vec<u8>>,
    pub tx: mpsc::Sender<Vec<u8>>,
}

impl WsServer {
    pub fn new() -> Self {
        Self {
            metrics: ServerMetrics {
                active_connections: 0,
                bytes_sent: 0,
                bytes_received: 0,
            },
            clients: HashMap::new(),
        }
    }

    pub fn handle_connection(&mut self, id: u64, tx: mpsc::Sender<Vec<u8>>) {
        let client = ClientConnection {
            id,
            state: ConnectionState::ProtocolHello,
            send_queue: VecDeque::new(),
            tx,
        };
        self.clients.insert(id, client);
        self.metrics.active_connections += 1;
    }

    pub fn disconnect_client(&mut self, id: u64) {
        if self.clients.remove(&id).is_some() {
            self.metrics.active_connections -= 1;
        }
    }

    pub fn update(&mut self) {
        // Backpressure, rate limits, flush per-client queues.
        for client in self.clients.values_mut() {
            while let Some(msg) = client.send_queue.pop_front() {
                let _ = client.tx.try_send(msg);
            }
        }
    }
}
