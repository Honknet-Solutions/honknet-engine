use async_trait::async_trait;
use honknet_net_core::{
    acked, update_ack, Channel, FragmentAssembler, Header, PacketFlags, ProtocolError,
    PROTOCOL_VERSION,
};
use parking_lot::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::{net::UdpSocket, sync::mpsc};
pub type PeerId = u64;
#[derive(Debug, Clone)]
pub enum TransportEvent {
    Connected(PeerId, SocketAddr),
    Data(PeerId, Channel, u16, Vec<u8>),
    Disconnected(PeerId, String),
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("peer missing")]
    PeerMissing,
    #[error("closed")]
    Closed,
}

#[async_trait]
pub trait NetworkTransport: Send + Sync {
    async fn poll(&self) -> Result<Vec<TransportEvent>, TransportError>;
    async fn send(
        &self,
        peer: PeerId,
        channel: Channel,
        kind: u16,
        payload: &[u8],
    ) -> Result<(), TransportError>;
    async fn disconnect(&self, peer: PeerId, reason: &str) -> Result<(), TransportError>;
}

struct Sent {
    at: Instant,
    packets: Vec<Vec<u8>>,
}

struct Peer {
    addr: SocketAddr,
    next: u32,
    recv_latest: u32,
    recv_bits: u32,
    sent: HashMap<u32, Sent>,
    assembler: FragmentAssembler,
    last_seen: Instant,
}

pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    peers: Arc<Mutex<HashMap<PeerId, Peer>>>,
    by_addr: Arc<Mutex<HashMap<SocketAddr, PeerId>>>,
    events: Arc<Mutex<VecDeque<TransportEvent>>>,
    next_peer: Arc<Mutex<PeerId>>,
    resend: Duration,
}

impl UdpTransport {
    pub async fn bind(addr: SocketAddr) -> Result<Arc<Self>, TransportError> {
        let s = Arc::new(Self {
            socket: Arc::new(UdpSocket::bind(addr).await?),
            peers: Default::default(),
            by_addr: Default::default(),
            events: Default::default(),
            next_peer: Arc::new(Mutex::new(1)),
            resend: Duration::from_millis(120),
        });
        let x = s.clone();
        tokio::spawn(async move { x.receive_loop().await });
        let x = s.clone();
        tokio::spawn(async move { x.resend_loop().await });
        Ok(s)
    }
    async fn receive_loop(self: Arc<Self>) {
        let mut buf = vec![0u8; 65536];
        loop {
            let Ok((n, addr)) = self.socket.recv_from(&mut buf).await else {
                break;
            };
            if n < Header::SIZE {
                continue;
            }
            let Ok(h) = Header::decode(&buf[..Header::SIZE]) else {
                continue;
            };
            if h.version != PROTOCOL_VERSION {
                continue;
            }
            let peer = if let Some(p) = self.by_addr.lock().get(&addr).copied() {
                p
            } else {
                let mut id = self.next_peer.lock();
                let p = *id;
                *id += 1;
                self.by_addr.lock().insert(addr, p);
                self.peers.lock().insert(
                    p,
                    Peer {
                        addr,
                        next: 1,
                        recv_latest: 0,
                        recv_bits: 0,
                        sent: HashMap::new(),
                        assembler: Default::default(),
                        last_seen: Instant::now(),
                    },
                );
                self.events
                    .lock()
                    .push_back(TransportEvent::Connected(p, addr));
                p
            };
            let payload = &buf[Header::SIZE..n.min(Header::SIZE + h.payload_len as usize)];
            let mut peers = self.peers.lock();
            let Some(state) = peers.get_mut(&peer) else {
                continue;
            };
            state.last_seen = Instant::now();
            state.sent.retain(|seq, _| !acked(*seq, h.ack, h.ack_bits));
            update_ack(&mut state.recv_latest, &mut state.recv_bits, h.sequence);
            if let Some(full) = state.assembler.push(h, payload) {
                self.events
                    .lock()
                    .push_back(TransportEvent::Data(peer, h.channel, h.kind, full));
            }
        }
    }
    async fn resend_loop(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_millis(30));
        loop {
            interval.tick().await;
            let now = Instant::now();
            let packets: Vec<_> = {
                let mut peers = self.peers.lock();
                peers
                    .values_mut()
                    .flat_map(|p| {
                        p.sent
                            .values_mut()
                            .filter(|s| now.duration_since(s.at) >= self.resend)
                            .flat_map(|s| {
                                s.at = now;
                                s.packets
                                    .iter()
                                    .cloned()
                                    .map(|x| (p.addr, x))
                                    .collect::<Vec<_>>()
                            })
                    })
                    .collect()
            };
            for (addr, p) in packets {
                let _ = self.socket.send_to(&p, addr).await;
            }
        }
    }
    pub async fn connect(&self, addr: SocketAddr) -> PeerId {
        if let Some(p) = self.by_addr.lock().get(&addr).copied() {
            return p;
        }
        let mut id = self.next_peer.lock();
        let p = *id;
        *id += 1;
        self.by_addr.lock().insert(addr, p);
        self.peers.lock().insert(
            p,
            Peer {
                addr,
                next: 1,
                recv_latest: 0,
                recv_bits: 0,
                sent: HashMap::new(),
                assembler: Default::default(),
                last_seen: Instant::now(),
            },
        );
        p
    }
}

#[async_trait]
impl NetworkTransport for UdpTransport {
    async fn poll(&self) -> Result<Vec<TransportEvent>, TransportError> {
        Ok(self.events.lock().drain(..).collect())
    }
    async fn send(
        &self,
        peer: PeerId,
        channel: Channel,
        kind: u16,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        let (addr, seq, ack, bits) = {
            let mut ps = self.peers.lock();
            let p = ps.get_mut(&peer).ok_or(TransportError::PeerMissing)?;
            let s = p.next;
            p.next = p.next.wrapping_add(1);
            (p.addr, s, p.recv_latest, p.recv_bits)
        };
        let reliable = matches!(
            channel,
            Channel::ReliableOrdered
                | Channel::ReliableUnordered
                | Channel::Control
                | Channel::BulkTransfer
        );
        let packets = honknet_net_core::fragment(
            kind,
            channel,
            if reliable {
                PacketFlags::RELIABLE
            } else {
                PacketFlags::empty()
            },
            seq,
            ack,
            bits,
            0,
            payload,
            seq as u16,
        );
        for x in &packets {
            self.socket.send_to(x, addr).await?;
        }
        if reliable {
            self.peers
                .lock()
                .get_mut(&peer)
                .ok_or(TransportError::PeerMissing)?
                .sent
                .insert(
                    seq,
                    Sent {
                        at: Instant::now(),
                        packets,
                    },
                );
        }
        Ok(())
    }
    async fn disconnect(&self, peer: PeerId, reason: &str) -> Result<(), TransportError> {
        let p = self
            .peers
            .lock()
            .remove(&peer)
            .ok_or(TransportError::PeerMissing)?;
        self.by_addr.lock().remove(&p.addr);
        self.events
            .lock()
            .push_back(TransportEvent::Disconnected(peer, reason.into()));
        Ok(())
    }
}

pub struct LoopbackTransport {
    tx: mpsc::Sender<TransportEvent>,
    rx: tokio::sync::Mutex<mpsc::Receiver<TransportEvent>>,
    peer: PeerId,
}

impl LoopbackTransport {
    pub fn pair() -> (Arc<Self>, Arc<Self>) {
        let (a_tx, a_rx) = mpsc::channel(1024);
        let (b_tx, b_rx) = mpsc::channel(1024);
        (
            Arc::new(Self {
                tx: b_tx,
                rx: tokio::sync::Mutex::new(a_rx),
                peer: 2,
            }),
            Arc::new(Self {
                tx: a_tx,
                rx: tokio::sync::Mutex::new(b_rx),
                peer: 1,
            }),
        )
    }
}

#[async_trait]
impl NetworkTransport for LoopbackTransport {
    async fn poll(&self) -> Result<Vec<TransportEvent>, TransportError> {
        let mut rx = self.rx.lock().await;
        let mut out = vec![];
        while let Ok(e) = rx.try_recv() {
            out.push(e)
        }
        Ok(out)
    }
    async fn send(
        &self,
        _: PeerId,
        ch: Channel,
        kind: u16,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.tx
            .send(TransportEvent::Data(self.peer, ch, kind, payload.to_vec()))
            .await
            .map_err(|_| TransportError::Closed)
    }
    async fn disconnect(&self, _: PeerId, reason: &str) -> Result<(), TransportError> {
        self.tx
            .send(TransportEvent::Disconnected(self.peer, reason.into()))
            .await
            .map_err(|_| TransportError::Closed)
    }
}

// QUIC and binary WebSocket adapters expose real encrypted/stream transports; game packet framing remains in honknet-net-core.
pub async fn quic_client_endpoint(bind: SocketAddr) -> Result<quinn::Endpoint, TransportError> {
    Ok(quinn::Endpoint::client(bind).map_err(|e| std::io::Error::other(e.to_string()))?)
}

pub async fn binary_websocket_connect(
    url: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    TransportError,
> {
    let (ws, _) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(ws)
}
