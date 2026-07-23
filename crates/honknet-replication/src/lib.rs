use honknet_core::Entity;
use honknet_math::Vec2;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplicationMode {
    All,
    Owner,
    Observers,
    ServerOnly,
    InitialOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    pub component_id: u16,
    pub revision: u64,
    pub dirty_mask: Vec<u64>,
    pub bytes: Vec<u8>,
    pub mode: ReplicationMode,
}

impl ComponentState {
    pub fn mark(&mut self, field: usize) {
        let word = field / 64;
        if self.dirty_mask.len() <= word {
            self.dirty_mask.resize(word + 1, 0)
        }
        self.dirty_mask[word] |= 1 << (field % 64)
    }
    pub fn clean(&mut self) {
        self.dirty_mask.fill(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub entity: Entity,
    pub revision: u64,
    pub position: Vec2,
    pub owner: Option<u64>,
    pub importance: f32,
    pub frequency: u16,
    pub components: Vec<ComponentState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u64,
    pub entities: Vec<EntityState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub tick: u64,
    pub baseline: u64,
    pub spawns: Vec<EntityState>,
    pub updates: Vec<EntityState>,
    pub despawns: Vec<Entity>,
}

pub struct InterestContext {
    pub client: u64,
    pub controlled: Option<Entity>,
    pub position: Vec2,
    pub observers: HashSet<Entity>,
    pub forced: HashSet<Entity>,
    pub teams: HashSet<u64>,
}

pub trait InterestProvider: Send + Sync {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool;
}

pub struct SpatialProvider {
    pub radius: f32,
}

impl InterestProvider for SpatialProvider {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool {
        (e.position - c.position).length_squared() <= self.radius * self.radius
    }
}

pub struct OwnershipProvider;
impl InterestProvider for OwnershipProvider {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool {
        e.owner == Some(c.client)
    }
}

pub struct ForcedProvider;
impl InterestProvider for ForcedProvider {
    fn interested(&self, c: &InterestContext, e: &EntityState) -> bool {
        c.forced.contains(&e.entity) || c.observers.contains(&e.entity)
    }
}

#[derive(Default)]
pub struct Replicator {
    pub states: HashMap<Entity, EntityState>,
    pub history: HashMap<u64, Snapshot>,
    pub providers: Vec<Box<dyn InterestProvider>>,
    pub max_history: usize,
}

impl Replicator {
    pub fn visible<'a>(&'a self, c: &InterestContext) -> Vec<&'a EntityState> {
        self.states
            .values()
            .filter(|e| self.providers.iter().any(|p| p.interested(c, e)))
            .collect()
    }
    pub fn build_snapshot(
        &mut self,
        tick: u64,
        c: &InterestContext,
        byte_budget: usize,
    ) -> Snapshot {
        let mut v = self.visible(c);
        v.sort_by(|a, b| priority(c, b).total_cmp(&priority(c, a)));
        let mut used = 0;
        let mut out = vec![];
        for e in v {
            let cost = e
                .components
                .iter()
                .map(|x| x.bytes.len() + 24)
                .sum::<usize>()
                + 48;
            if used + cost > byte_budget {
                continue;
            }
            used += cost;
            out.push(e.clone())
        }
        let s = Snapshot {
            tick,
            entities: out,
        };
        self.history.insert(tick, s.clone());
        while self.history.len() > self.max_history.max(32) {
            if let Some(k) = self.history.keys().min().copied() {
                self.history.remove(&k);
            }
        }
        s
    }
    pub fn delta(&self, baseline: u64, current: &Snapshot) -> Option<Delta> {
        let old = self.history.get(&baseline)?;
        let a: HashMap<_, _> = old.entities.iter().map(|e| (e.entity, e)).collect();
        let b: HashMap<_, _> = current.entities.iter().map(|e| (e.entity, e)).collect();
        Some(Delta {
            tick: current.tick,
            baseline,
            spawns: b
                .iter()
                .filter(|(k, _)| !a.contains_key(k))
                .map(|(_, e)| (*e).clone())
                .collect(),
            updates: b
                .iter()
                .filter(|(k, e)| a.get(k).is_some_and(|o| o.revision != e.revision))
                .map(|(_, e)| (*e).clone())
                .collect(),
            despawns: a.keys().filter(|k| !b.contains_key(k)).copied().collect(),
        })
    }
    pub fn parallel_snapshots(
        &self,
        clients: &[(InterestContext, usize)],
        tick: u64,
    ) -> Vec<Snapshot> {
        clients
            .par_iter()
            .map(|(c, b)| {
                let mut v = self.visible(c);
                v.sort_by(|a, b| priority(c, b).total_cmp(&priority(c, a)));
                let mut used = 0;
                Snapshot {
                    tick,
                    entities: v
                        .into_iter()
                        .filter_map(|e| {
                            let n = e
                                .components
                                .iter()
                                .map(|x| x.bytes.len() + 24)
                                .sum::<usize>()
                                + 48;
                            if used + n > *b {
                                None
                            } else {
                                used += n;
                                Some(e.clone())
                            }
                        })
                        .collect(),
                }
            })
            .collect()
    }
}

fn priority(c: &InterestContext, e: &EntityState) -> f32 {
    let d = (e.position - c.position).length().max(1.);
    e.importance * 1000. / d
        + if e.owner == Some(c.client) {
            10000.
        } else {
            0.
        }
        + if c.forced.contains(&e.entity) {
            20000.
        } else {
            0.
        }
}
