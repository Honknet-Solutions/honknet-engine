use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tokio::{
    net::TcpListener,
    sync::{OwnedSemaphorePermit, Semaphore},
};
use tracing::{error, info, warn};

use crate::{app_state::AppState, client_session, metrics::EngineMetrics};

pub async fn run(listen_addr: &str, state: AppState) -> Result<()> {
    let listener = TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("failed to bind {listen_addr}"))?;
    let max_connections = env_usize("HONKNET_MAX_CONNECTIONS", 256).max(1);
    let max_connections_per_ip = env_usize("HONKNET_MAX_CONNECTIONS_PER_IP", 8).max(1);
    let permits = Arc::new(Semaphore::new(max_connections));
    let per_ip = Arc::new(Mutex::new(HashMap::<IpAddr, usize>::new()));

    info!(
        listen_addr,
        max_connections, max_connections_per_ip, "WebSocket listener started"
    );

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let _ = stream.set_nodelay(true);
        let permit = match permits.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                state.metrics.connection_rejected();
                warn!(%peer_addr, "Connection rejected: server capacity reached");
                drop(stream);
                continue;
            }
        };
        let ip_guard = match IpConnectionGuard::acquire(
            per_ip.clone(),
            peer_addr.ip(),
            max_connections_per_ip,
        ) {
            Some(guard) => guard,
            None => {
                state.metrics.connection_rejected();
                warn!(%peer_addr, "Connection rejected: per-IP limit reached");
                drop(stream);
                continue;
            }
        };
        let state = state.clone();

        tokio::spawn(async move {
            let _permit: OwnedSemaphorePermit = permit;
            let _ip_guard = ip_guard;
            state.metrics.connection_opened();
            let _metrics_guard = ConnectionMetricsGuard(state.metrics.clone());
            if let Err(error) = client_session::run(stream, peer_addr, state).await {
                error!(%peer_addr, %error, "Client session failed");
            }
        });
    }
}

struct ConnectionMetricsGuard(Arc<EngineMetrics>);

impl Drop for ConnectionMetricsGuard {
    fn drop(&mut self) {
        self.0.connection_closed();
    }
}

struct IpConnectionGuard {
    registry: Arc<Mutex<HashMap<IpAddr, usize>>>,
    ip: IpAddr,
}

impl IpConnectionGuard {
    fn acquire(
        registry: Arc<Mutex<HashMap<IpAddr, usize>>>,
        ip: IpAddr,
        maximum: usize,
    ) -> Option<Self> {
        {
            let mut counts = registry
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let count = counts.entry(ip).or_default();
            if *count >= maximum {
                return None;
            }
            *count += 1;
        }
        Some(Self { registry, ip })
    }
}

impl Drop for IpConnectionGuard {
    fn drop(&mut self) {
        let mut counts = self
            .registry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(count) = counts.get_mut(&self.ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                counts.remove(&self.ip);
            }
        }
    }
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}
