use honknet_ecs::{CommandBuffer, World};
use rayon::prelude::*;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Phase {
    Startup,
    PreInput,
    Input,
    PreSimulation,
    Simulation,
    Physics,
    PostPhysics,
    Gameplay,
    PostGameplay,
    ReplicationPrepare,
    Replication,
    Persistence,
    Frame,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(u64);

impl TimerId {
    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
struct TimerEntry<T> {
    id: TimerId,
    sequence: u64,
    repeat_interval: Option<u64>,
    payload: T,
}

#[derive(Debug, Clone)]
pub struct TickTimerQueue<T> {
    next_id: u64,
    next_sequence: u64,
    entries: BTreeMap<u64, Vec<TimerEntry<T>>>,
}

impl<T> Default for TickTimerQueue<T> {
    fn default() -> Self {
        Self {
            next_id: 1,
            next_sequence: 0,
            entries: BTreeMap::new(),
        }
    }
}

impl<T: Clone> TickTimerQueue<T> {
    pub fn schedule_at(&mut self, due_tick: u64, payload: T) -> TimerId {
        self.schedule(due_tick, None, payload)
    }

    pub fn schedule_repeating(
        &mut self,
        first_tick: u64,
        interval_ticks: u64,
        payload: T,
    ) -> TimerId {
        self.schedule(first_tick, Some(interval_ticks.max(1)), payload)
    }

    fn schedule(&mut self, due_tick: u64, repeat_interval: Option<u64>, payload: T) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.entries.entry(due_tick).or_default().push(TimerEntry {
            id,
            sequence,
            repeat_interval,
            payload,
        });
        id
    }

    pub fn cancel(&mut self, id: TimerId) -> bool {
        let mut removed = false;
        self.entries.retain(|_, entries| {
            let before = entries.len();
            entries.retain(|entry| entry.id != id);
            removed |= entries.len() != before;
            !entries.is_empty()
        });
        removed
    }

    pub fn advance(&mut self, current_tick: u64) -> Vec<(TimerId, T)> {
        let future = self.entries.split_off(&current_tick.saturating_add(1));
        let due = std::mem::replace(&mut self.entries, future);
        let mut entries: Vec<(u64, TimerEntry<T>)> = due
            .into_iter()
            .flat_map(|(tick, entries)| entries.into_iter().map(move |entry| (tick, entry)))
            .collect();
        entries.sort_by_key(|(tick, entry)| (*tick, entry.sequence));

        let mut callbacks = Vec::with_capacity(entries.len());
        for (tick, entry) in entries {
            callbacks.push((entry.id, entry.payload.clone()));
            if let Some(interval) = entry.repeat_interval {
                self.entries
                    .entry(tick.saturating_add(interval))
                    .or_default()
                    .push(entry);
            }
        }
        callbacks
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Access {
    pub reads: HashSet<&'static str>,
    pub writes: HashSet<&'static str>,
}

impl Access {
    pub fn conflicts(&self, other: &Self) -> bool {
        self.writes
            .iter()
            .any(|name| other.writes.contains(name) || other.reads.contains(name))
            || self.reads.iter().any(|name| other.writes.contains(name))
    }
}

pub trait System: Send {
    fn name(&self) -> &'static str;
    fn phase(&self) -> Phase;
    fn access(&self) -> Access;

    fn before(&self) -> &'static [&'static str] {
        &[]
    }

    fn after(&self) -> &'static [&'static str] {
        &[]
    }

    fn frequency(&self) -> u32 {
        1
    }

    fn run(&mut self, world: &mut World, commands: &mut CommandBuffer, delta_seconds: f32);
}

#[derive(Debug, Error)]
pub enum ScheduleError {
    #[error("dependency cycle in phase {0:?}")]
    Cycle(Phase),

    #[error("system failed: {0}")]
    System(String),
}

#[derive(Debug, Clone, Default)]
pub struct SystemTiming {
    pub runs: u64,
    pub total: Duration,
    pub maximum: Duration,
}

struct Entry {
    system: Box<dyn System>,
    timing: SystemTiming,
}

#[derive(Default)]
pub struct Scheduler {
    entries: Vec<Entry>,
    frame: u64,
}

impl Scheduler {
    pub fn add<S>(&mut self, system: S)
    where
        S: System + 'static,
    {
        self.add_boxed(Box::new(system));
    }

    pub fn add_boxed(&mut self, system: Box<dyn System>) {
        self.entries.push(Entry {
            system,
            timing: SystemTiming::default(),
        });
    }

    pub fn timings(&self) -> HashMap<&'static str, SystemTiming> {
        self.entries
            .iter()
            .map(|entry| (entry.system.name(), entry.timing.clone()))
            .collect()
    }

    pub fn run(&mut self, world: &mut World, delta_seconds: f32) -> Result<(), ScheduleError> {
        self.frame += 1;

        let phases = [
            Phase::Startup,
            Phase::PreInput,
            Phase::Input,
            Phase::PreSimulation,
            Phase::Simulation,
            Phase::Physics,
            Phase::PostPhysics,
            Phase::Gameplay,
            Phase::PostGameplay,
            Phase::ReplicationPrepare,
            Phase::Replication,
            Phase::Persistence,
            Phase::Frame,
        ];

        for phase in phases {
            let ids: Vec<usize> = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.system.phase() == phase
                        && self.frame % u64::from(entry.system.frequency()) == 0
                })
                .map(|(index, _)| index)
                .collect();

            for batch in self.batches(&ids, phase)? {
                for index in batch {
                    let started_at = Instant::now();
                    let mut commands = CommandBuffer::default();

                    self.entries[index]
                        .system
                        .run(world, &mut commands, delta_seconds);

                    commands
                        .apply(world)
                        .map_err(|error| ScheduleError::System(error.to_string()))?;

                    let elapsed = started_at.elapsed();
                    let timing = &mut self.entries[index].timing;
                    timing.runs += 1;
                    timing.total += elapsed;
                    timing.maximum = timing.maximum.max(elapsed);
                }
            }
        }

        world.advance_tick();
        Ok(())
    }

    fn batches(&self, ids: &[usize], phase: Phase) -> Result<Vec<Vec<usize>>, ScheduleError> {
        let mut remaining: HashSet<usize> = ids.iter().copied().collect();
        let mut completed = HashSet::new();
        let names: HashMap<&str, usize> = ids
            .iter()
            .map(|index| (self.entries[*index].system.name(), *index))
            .collect();
        let mut batches = Vec::new();

        while !remaining.is_empty() {
            let mut ready: Vec<usize> = remaining
                .iter()
                .copied()
                .filter(|index| {
                    self.entries[*index].system.after().iter().all(|name| {
                        names
                            .get(name)
                            .is_none_or(|dependency| completed.contains(dependency))
                    })
                })
                .collect();
            ready.sort_unstable();

            if ready.is_empty() {
                return Err(ScheduleError::Cycle(phase));
            }

            let mut batch: Vec<usize> = Vec::new();
            for index in ready {
                let candidate_access = self.entries[index].system.access();
                let has_conflict = batch.iter().any(|existing_index| {
                    candidate_access.conflicts(&self.entries[*existing_index].system.access())
                });

                if !has_conflict {
                    batch.push(index);
                }
            }

            for index in &batch {
                remaining.remove(index);
                completed.insert(*index);
            }

            batches.push(batch);
        }

        Ok(batches)
    }
}

/// Executes independent, immutable jobs in parallel.
pub fn parallel_map<T, R, F>(items: Vec<T>, function: F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Send + Sync,
{
    items.into_par_iter().map(function).collect()
}

pub trait PreparedSystem: Send + Sync {
    fn name(&self) -> &'static str;
    fn phase(&self) -> Phase;
    fn prepare(
        &self,
        world: &World,
        delta_seconds: f32,
    ) -> Box<dyn FnOnce() -> CommandBuffer + Send>;
}

pub fn run_prepared_parallel(
    systems: &[Box<dyn PreparedSystem>],
    world: &mut World,
    phase: Phase,
    delta_seconds: f32,
) -> Result<(), ScheduleError> {
    let jobs: Vec<_> = systems
        .iter()
        .filter(|system| system.phase() == phase)
        .map(|system| system.prepare(world, delta_seconds))
        .collect();

    let buffers: Vec<_> = jobs.into_par_iter().map(|job| job()).collect();
    for buffer in buffers {
        buffer
            .apply(world)
            .map_err(|error| ScheduleError::System(error.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timers_are_deterministic_cancellable_and_repeatable() {
        let mut timers = TickTimerQueue::default();
        let cancelled = timers.schedule_at(2, "cancelled");
        timers.schedule_at(2, "second");
        timers.schedule_at(1, "first");
        let repeating = timers.schedule_repeating(1, 2, "repeat");
        assert!(timers.cancel(cancelled));

        assert_eq!(
            timers.advance(1),
            vec![(TimerId(3), "first"), (repeating, "repeat")]
        );
        assert_eq!(timers.advance(2), vec![(TimerId(2), "second")]);
        assert_eq!(timers.advance(3), vec![(repeating, "repeat")]);
        assert!(timers.cancel(repeating));
        assert!(timers.is_empty());
    }
}
