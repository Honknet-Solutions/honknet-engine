use futures_util::{SinkExt, StreamExt};
use honknet_net_core::*;
use std::collections::{HashMap, VecDeque};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::info;

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

    pub fn send_to(&mut self, id: u64, payload: Vec<u8>) {
        if let Some(client) = self.clients.get_mut(&id) {
            client.send_queue.push_back(payload);
        }
    }

    pub fn update(&mut self) {
        for client in self.clients.values_mut() {
            while let Some(msg) = client.send_queue.pop_front() {
                let _ = client.tx.try_send(msg);
            }
        }
    }

    pub async fn bind_and_listen(&mut self, addr: &str) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        info!("Honknet WebSocket Server listening on {}", addr);

        let mut peer_id_gen = 1000u64;

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            info!("Accepted TCP connection from {}", peer_addr);

            let ws_stream = tokio_tungstenite::accept_async(stream).await?;
            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(256);
            let peer_id = peer_id_gen;
            peer_id_gen += 1;

            self.handle_connection(peer_id, tx);

            // Spawn writer loop
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if ws_sender
                        .send(tokio_tungstenite::tungstenite::Message::Binary(msg.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });

            // Spawn reader loop
            tokio::spawn(async move {
                while let Some(Ok(msg)) = ws_receiver.next().await {
                    if msg.is_close() {
                        break;
                    }
                }
            });
        }
    }
}
