use honknet_ecs::World;
use honknet_net_transport::{LoopbackTransport, NetworkTransport, TransportEvent};
use sha2::{Digest, Sha256};
use std::sync::Arc;
pub struct HeadlessHarness {
    pub server: Arc<LoopbackTransport>,
    pub client: Arc<LoopbackTransport>,
    pub world: World,
}

impl HeadlessHarness {
    pub fn new() -> Self {
        let (s, c) = LoopbackTransport::pair();
        Self {
            server: s,
            client: c,
            world: World::default(),
        }
    }
    pub async fn round_trip(&self, payload: &[u8]) -> bool {
        use honknet_net_core::Channel;
        self.client
            .send(1, Channel::ReliableOrdered, 7, payload)
            .await
            .is_ok()
            && self.server.poll().await.ok().is_some_and(|e| {
                e.iter().any(|x| {
                    matches!(x,
        TransportEvent::Data(_, _, 7, p) if p == payload)
                })
            })
    }
}

pub fn state_hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn deterministic<F: FnMut() -> Vec<u8>>(mut run: F) -> bool {
    state_hash(&run()) == state_hash(&run())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn loopback_uses_real_transport_contract() {
        assert!(HeadlessHarness::new().round_trip(b"hello").await)
    }
    #[test]
    fn deterministic_check() {
        assert!(deterministic(|| vec![1, 2, 3]))
    }
}
