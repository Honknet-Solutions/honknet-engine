use honknet_core::Entity;
use honknet_math::Vec2;
use rand::{rngs::SmallRng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputCommand {
    pub sequence: u32,
    pub client_tick: u64,
    pub estimated_server_tick: u64,
    pub movement: Vec2,
    pub aim: Vec2,
    pub pressed: u64,
    pub released: u64,
    pub analog: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedState {
    pub tick: u64,
    pub entity: Entity,
    pub position: Vec2,
    pub velocity: Vec2,
    pub alive: bool,
    pub user: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputFrame {
    pub tick: u64,
    pub sequence: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PredictionHistory {
    pub input: InputCommand,
    pub predicted_transform: PredictedState,
    pub predicted_velocity: Vec2,
    pub predicted_physics_state: Vec<u8>,
    pub rng_state: u64,
    pub predicted_spawns: Vec<Entity>,
    pub predicted_despawns: Vec<Entity>,
}

pub struct Reconciliation {
    pub max_divergence: f32,
}

impl Reconciliation {
    pub fn find_divergence(&self, auth: &PredictedState, hist: &PredictionHistory) -> bool {
        (auth.position - hist.predicted_transform.position).length() > self.max_divergence
    }

    pub fn replay_unconfirmed<F>(
        &self,
        start_state: &PredictedState,
        inputs: &[InputCommand],
        mut apply: F,
    ) -> PredictedState
    where
        F: FnMut(&PredictedState, &InputCommand) -> PredictedState,
    {
        let mut state = start_state.clone();
        for input in inputs {
            state = apply(&state, input);
        }
        state
    }

    pub fn smooth_correction(
        &self,
        current: &mut PredictedState,
        target: &PredictedState,
        factor: f32,
    ) {
        current.position = current.position + (target.position - current.position) * factor;
        current.velocity = current.velocity + (target.velocity - current.velocity) * factor;
    }
}

#[allow(dead_code)]
pub struct SnapshotBuffer {
    pub snapshots: VecDeque<PredictedState>,
    pub capacity: usize,
}

pub struct RemoteInterpolation {
    pub buffer: SnapshotBuffer,
    pub interpolation_delay: f64,
    pub snap_threshold: f32,
}

impl RemoteInterpolation {
    pub fn new(capacity: usize, delay: f64, snap: f32) -> Self {
        Self {
            buffer: SnapshotBuffer {
                snapshots: VecDeque::new(),
                capacity,
            },
            interpolation_delay: delay,
            snap_threshold: snap,
        }
    }

    pub fn teleport_detection(&self, a: &PredictedState, b: &PredictedState) -> bool {
        (a.position - b.position).length() > self.snap_threshold
    }

    pub fn interpolate_position(&self, a: &PredictedState, b: &PredictedState, t: f32) -> Vec2 {
        if self.teleport_detection(a, b) {
            return b.position;
        }
        a.position + (b.position - a.position) * t
    }

    pub fn interpolate_rotation(&self, a: f32, b: f32, t: f32) -> f32 {
        let mut diff = b - a;
        while diff < -std::f32::consts::PI {
            diff += std::f32::consts::PI * 2.0;
        }
        while diff > std::f32::consts::PI {
            diff -= std::f32::consts::PI * 2.0;
        }
        a + diff * t
    }

    pub fn interpolate_animation(&self, a: u32, b: u32, t: f32) -> u32 {
        if t >= 1.0 {
            b
        } else {
            a
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PredictionDiagnostics {
    pub rollbacks: u64,
    pub replays: u64,
    pub max_error: f32,
    pub last_error: f32,
    pub predicted_spawns: u64,
    pub predicted_despawns: u64,
}

pub struct PredictionBuffer {
    states: HashMap<Entity, VecDeque<PredictedState>>,
    inputs: VecDeque<InputFrame>,
    capacity: usize,
    pub diagnostics: PredictionDiagnostics,
}

impl PredictionBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            states: HashMap::new(),
            inputs: VecDeque::new(),
            capacity: capacity.max(2),
            diagnostics: Default::default(),
        }
    }
    pub fn record_state(&mut self, s: PredictedState) {
        let q = self.states.entry(s.entity).or_default();
        q.push_back(s);
        while q.len() > self.capacity {
            q.pop_front();
        }
    }
    pub fn record_input(&mut self, i: InputFrame) {
        self.inputs.push_back(i);
        while self.inputs.len() > self.capacity {
            self.inputs.pop_front();
        }
    }
    pub fn reconcile<F>(
        &mut self,
        authoritative: PredictedState,
        tolerance: f32,
        mut apply: F,
    ) -> PredictedState
    where
        F: FnMut(&PredictedState, &InputFrame) -> PredictedState,
    {
        let predicted = self
            .states
            .get(&authoritative.entity)
            .and_then(|q| q.iter().find(|x| x.tick == authoritative.tick))
            .cloned();
        let error = predicted.as_ref().map_or(f32::INFINITY, |p| {
            (p.position - authoritative.position).length()
        });
        self.diagnostics.last_error = error;
        self.diagnostics.max_error = self.diagnostics.max_error.max(error);
        if error <= tolerance {
            return predicted.unwrap();
        }
        self.diagnostics.rollbacks += 1;
        let start_tick = authoritative.tick;
        let mut state = authoritative;
        for i in self.inputs.iter().filter(|i| i.tick > start_tick) {
            state = apply(&state, i);
            self.diagnostics.replays += 1;
        }
        self.record_state(state.clone());
        state
    }
    pub fn interpolate(&self, e: Entity, tick: f64) -> Option<PredictedState> {
        let q = self.states.get(&e)?;
        let a = q.iter().rev().find(|x| x.tick as f64 <= tick)?;
        let b = q.iter().find(|x| x.tick as f64 >= tick).unwrap_or(a);
        if a.tick == b.tick {
            return Some(a.clone());
        }
        let t = ((tick - a.tick as f64) / (b.tick - a.tick) as f64) as f32;
        let mut r = a.clone();
        r.position = a.position + (b.position - a.position) * t;
        r.velocity = a.velocity + (b.velocity - a.velocity) * t;
        Some(r)
    }
    pub fn extrapolate(
        &self,
        e: Entity,
        tick: u64,
        max_ticks: u64,
        dt: f32,
    ) -> Option<PredictedState> {
        let mut s = self.states.get(&e)?.back()?.clone();
        let n = tick.saturating_sub(s.tick).min(max_ticks);
        s.position += s.velocity * (dt * n as f32);
        s.tick += n;
        Some(s)
    }
}

pub struct DeterministicRandom(SmallRng);
impl DeterministicRandom {
    pub fn for_tick(seed: u64, tick: u64, stream: u64) -> Self {
        Self(SmallRng::seed_from_u64(
            seed ^ tick.rotate_left(17) ^ stream.rotate_right(9),
        ))
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
}
