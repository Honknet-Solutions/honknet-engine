use honknet_ecs::{CommandBuffer, World};
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
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
        self.entries.push(Entry {
            system: Box::new(system),
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
