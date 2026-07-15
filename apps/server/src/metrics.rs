use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

#[derive(Debug)]
pub struct EngineMetrics {
    started_at: Instant,
    connections_current: AtomicU64,
    connections_total: AtomicU64,
    connections_rejected: AtomicU64,
    messages_received: AtomicU64,
    malformed_messages: AtomicU64,
    rate_limited_messages: AtomicU64,
    bytes_received: AtomicU64,
    full_snapshots_sent: AtomicU64,
    delta_snapshots_sent: AtomicU64,
    ticks_total: AtomicU64,
    tick_overruns_total: AtomicU64,
    last_tick_micros: AtomicU64,
    max_tick_micros: AtomicU64,
    script_failures_total: AtomicU64,
    autosaves_total: AtomicU64,
    persistence_failures_total: AtomicU64,
}

impl Default for EngineMetrics {
    fn default() -> Self {
        Self {
            started_at: Instant::now(),
            connections_current: AtomicU64::new(0),
            connections_total: AtomicU64::new(0),
            connections_rejected: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            malformed_messages: AtomicU64::new(0),
            rate_limited_messages: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            full_snapshots_sent: AtomicU64::new(0),
            delta_snapshots_sent: AtomicU64::new(0),
            ticks_total: AtomicU64::new(0),
            tick_overruns_total: AtomicU64::new(0),
            last_tick_micros: AtomicU64::new(0),
            max_tick_micros: AtomicU64::new(0),
            script_failures_total: AtomicU64::new(0),
            autosaves_total: AtomicU64::new(0),
            persistence_failures_total: AtomicU64::new(0),
        }
    }
}

impl EngineMetrics {
    pub fn connection_opened(&self) {
        self.connections_current.fetch_add(1, Ordering::Relaxed);
        self.connections_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn connection_closed(&self) {
        let _ =
            self.connections_current
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    Some(value.saturating_sub(1))
                });
    }

    pub fn connection_rejected(&self) {
        self.connections_rejected.fetch_add(1, Ordering::Relaxed);
    }

    pub fn message_received(&self, bytes: usize) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received
            .fetch_add(bytes.try_into().unwrap_or(u64::MAX), Ordering::Relaxed);
    }

    pub fn malformed_message(&self) {
        self.malformed_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn rate_limited_message(&self) {
        self.rate_limited_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot_sent(&self, full: bool) {
        if full {
            self.full_snapshots_sent.fetch_add(1, Ordering::Relaxed);
        } else {
            self.delta_snapshots_sent.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn tick_completed(&self, elapsed_micros: u64, budget_micros: u64) {
        self.ticks_total.fetch_add(1, Ordering::Relaxed);
        self.last_tick_micros
            .store(elapsed_micros, Ordering::Relaxed);
        self.max_tick_micros
            .fetch_max(elapsed_micros, Ordering::Relaxed);
        if elapsed_micros > budget_micros {
            self.tick_overruns_total.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn script_failure(&self) {
        self.script_failures_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn autosave_completed(&self) {
        self.autosaves_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn persistence_failure(&self) {
        self.persistence_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub fn render_prometheus(&self, entity_count: usize, online_players: usize) -> String {
        let value = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        format!(
            concat!(
                "# TYPE honknet_uptime_seconds gauge\n",
                "honknet_uptime_seconds {}\n",
                "# TYPE honknet_connections_current gauge\n",
                "honknet_connections_current {}\n",
                "# TYPE honknet_connections_total counter\n",
                "honknet_connections_total {}\n",
                "# TYPE honknet_connections_rejected_total counter\n",
                "honknet_connections_rejected_total {}\n",
                "# TYPE honknet_messages_received_total counter\n",
                "honknet_messages_received_total {}\n",
                "# TYPE honknet_malformed_messages_total counter\n",
                "honknet_malformed_messages_total {}\n",
                "# TYPE honknet_rate_limited_messages_total counter\n",
                "honknet_rate_limited_messages_total {}\n",
                "# TYPE honknet_network_bytes_received_total counter\n",
                "honknet_network_bytes_received_total {}\n",
                "# TYPE honknet_full_snapshots_sent_total counter\n",
                "honknet_full_snapshots_sent_total {}\n",
                "# TYPE honknet_delta_snapshots_sent_total counter\n",
                "honknet_delta_snapshots_sent_total {}\n",
                "# TYPE honknet_ticks_total counter\n",
                "honknet_ticks_total {}\n",
                "# TYPE honknet_tick_overruns_total counter\n",
                "honknet_tick_overruns_total {}\n",
                "# TYPE honknet_last_tick_seconds gauge\n",
                "honknet_last_tick_seconds {:.6}\n",
                "# TYPE honknet_max_tick_seconds gauge\n",
                "honknet_max_tick_seconds {:.6}\n",
                "# TYPE honknet_script_failures_total counter\n",
                "honknet_script_failures_total {}\n",
                "# TYPE honknet_autosaves_total counter\n",
                "honknet_autosaves_total {}\n",
                "# TYPE honknet_persistence_failures_total counter\n",
                "honknet_persistence_failures_total {}\n",
                "# TYPE honknet_entities gauge\n",
                "honknet_entities {}\n",
                "# TYPE honknet_players_online gauge\n",
                "honknet_players_online {}\n"
            ),
            self.uptime_seconds(),
            value(&self.connections_current),
            value(&self.connections_total),
            value(&self.connections_rejected),
            value(&self.messages_received),
            value(&self.malformed_messages),
            value(&self.rate_limited_messages),
            value(&self.bytes_received),
            value(&self.full_snapshots_sent),
            value(&self.delta_snapshots_sent),
            value(&self.ticks_total),
            value(&self.tick_overruns_total),
            value(&self.last_tick_micros) as f64 / 1_000_000.0,
            value(&self.max_tick_micros) as f64 / 1_000_000.0,
            value(&self.script_failures_total),
            value(&self.autosaves_total),
            value(&self.persistence_failures_total),
            entity_count,
            online_players,
        )
    }
}
