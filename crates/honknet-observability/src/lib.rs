use parking_lot::RwLock;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Registry, TextEncoder,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,
    pub tick_seconds: HistogramVec,
    pub systems: HistogramVec,
    pub network_bytes: IntCounterVec,
    pub pvs_visible: IntGauge,
    pub entities: IntGauge,
    pub physics_contacts: IntGauge,
    pub prediction_mismatch: IntCounterVec,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        let tick_seconds = HistogramVec::new(
            HistogramOpts::new("honknet_tick_seconds", "Authoritative tick duration"),
            &["phase"],
        )
        .expect("valid tick histogram");
        let systems = HistogramVec::new(
            HistogramOpts::new("honknet_system_seconds", "System duration"),
            &["system"],
        )
        .expect("valid system histogram");
        let network_bytes = IntCounterVec::new(
            prometheus::Opts::new("honknet_network_bytes_total", "Network bytes"),
            &["direction", "channel"],
        )
        .expect("valid network counter");
        let pvs_visible =
            IntGauge::new("honknet_pvs_visible", "Visible entities").expect("valid PVS gauge");
        let entities =
            IntGauge::new("honknet_entities", "Entity count").expect("valid entity gauge");
        let physics_contacts = IntGauge::new("honknet_physics_contacts", "Physics contacts")
            .expect("valid contact gauge");
        let prediction_mismatch = IntCounterVec::new(
            prometheus::Opts::new("honknet_prediction_mismatch_total", "Prediction mismatches"),
            &["component"],
        )
        .expect("valid prediction counter");
        registry
            .register(Box::new(tick_seconds.clone()))
            .expect("register tick histogram");
        registry
            .register(Box::new(systems.clone()))
            .expect("register system histogram");
        registry
            .register(Box::new(network_bytes.clone()))
            .expect("register network counter");
        registry
            .register(Box::new(pvs_visible.clone()))
            .expect("register PVS gauge");
        registry
            .register(Box::new(entities.clone()))
            .expect("register entity gauge");
        registry
            .register(Box::new(physics_contacts.clone()))
            .expect("register contact gauge");
        registry
            .register(Box::new(prediction_mismatch.clone()))
            .expect("register prediction counter");
        Self {
            registry,
            tick_seconds,
            systems,
            network_bytes,
            pvs_visible,
            entities,
            physics_contacts,
            prediction_mismatch,
        }
    }
    pub fn encode(&self) -> String {
        let mut bytes = Vec::new();
        TextEncoder::new()
            .encode(&self.registry.gather(), &mut bytes)
            .unwrap_or_default();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Health {
    pub live: bool,
    pub ready: bool,
    pub tick: u64,
    pub uptime_seconds: f64,
    pub checks: HashMap<String, bool>,
}

#[derive(Clone)]
pub struct HealthState {
    start: Instant,
    inner: Arc<RwLock<Health>>,
}

impl Default for HealthState {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            inner: Arc::new(RwLock::new(Health {
                live: true,
                ready: false,
                tick: 0,
                uptime_seconds: 0.0,
                checks: HashMap::new(),
            })),
        }
    }
}

impl HealthState {
    pub fn set_check(&self, name: &str, ok: bool) {
        let mut health = self.inner.write();
        health.checks.insert(name.to_owned(), ok);
        health.ready = health.checks.values().all(|value| *value);
        health.uptime_seconds = self.start.elapsed().as_secs_f64();
    }
    pub fn tick(&self, tick: u64) {
        let mut health = self.inner.write();
        health.tick = tick;
        health.uptime_seconds = self.start.elapsed().as_secs_f64();
    }
    pub fn snapshot(&self) -> Health {
        self.inner.read().clone()
    }
}

pub struct ProfileGuard<'a> {
    name: &'a str,
    start: Instant,
    sink: &'a RwLock<HashMap<String, (u64, Duration)>>,
}

impl Drop for ProfileGuard<'_> {
    fn drop(&mut self) {
        let mut sink = self.sink.write();
        let entry = sink.entry(self.name.to_owned()).or_default();
        entry.0 += 1;
        entry.1 += self.start.elapsed();
    }
}

#[derive(Default)]
pub struct Profiler {
    data: RwLock<HashMap<String, (u64, Duration)>>,
}

impl Profiler {
    pub fn span<'a>(&'a self, name: &'a str) -> ProfileGuard<'a> {
        ProfileGuard {
            name,
            start: Instant::now(),
            sink: &self.data,
        }
    }
    pub fn snapshot(&self) -> HashMap<String, (u64, Duration)> {
        self.data.read().clone()
    }
}

pub async fn serve_http(
    address: &str,
    metrics: Metrics,
    health: HealthState,
) -> Result<(), std::io::Error> {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind(address).await?;
    loop {
        let (mut stream, _) = listener.accept().await?;
        let metrics = metrics.clone();
        let health = health.clone();
        tokio::spawn(async move {
            let mut request = [0u8; 2048];
            let n = stream.read(&mut request).await.unwrap_or(0);
            let line = String::from_utf8_lossy(&request[..n]);
            let (_path, content_type, body, status) = if line.starts_with("GET /metrics ") {
                ("/metrics", "text/plain", metrics.encode(), "200 OK")
            } else if line.starts_with("GET /health ") || line.starts_with("GET /ready ") {
                let h = health.snapshot();
                let status = if h.live && (h.ready || line.starts_with("GET /health ")) {
                    "200 OK"
                } else {
                    "503 Service Unavailable"
                };
                (
                    "/health",
                    "application/json",
                    serde_json::to_string(&h).unwrap_or_default(),
                    status,
                )
            } else {
                ("/", "text/plain", "not found".into(), "404 Not Found")
            };
            let response = format!("HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len());
            let _ = stream.write_all(response.as_bytes()).await;
        });
    }
}
