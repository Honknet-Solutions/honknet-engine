use honknet_ecs::Component;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundPhase {
    Lobby,
    Starting,
    InProgress,
    Ending,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    pub id: String,
    pub display_name: String,
    pub capacity: u16,
    pub access_tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LobbyPlayer {
    pub ready: bool,
    pub preferences: Vec<String>,
    pub assigned_job: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobComponent {
    pub job_id: String,
    pub display_name: String,
}

impl Component for JobComponent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundController {
    pub phase: RoundPhase,
    pub round_id: u64,
    pub elapsed_ticks: u64,
    pub countdown_ticks: u64,
    pub minimum_ready_players: usize,
    pub players: BTreeMap<u64, LobbyPlayer>,
    pub jobs: BTreeMap<String, JobDefinition>,
    pub end_reason: Option<String>,
}

impl Default for RoundController {
    fn default() -> Self {
        let jobs = [
            JobDefinition {
                id: "captain".into(),
                display_name: "Captain".into(),
                capacity: 1,
                access_tags: vec!["command".into(), "all_station".into()],
            },
            JobDefinition {
                id: "medical_doctor".into(),
                display_name: "Medical Doctor".into(),
                capacity: 4,
                access_tags: vec!["medical".into()],
            },
            JobDefinition {
                id: "engineer".into(),
                display_name: "Engineer".into(),
                capacity: 4,
                access_tags: vec!["engineering".into(), "maintenance".into()],
            },
            JobDefinition {
                id: "security_officer".into(),
                display_name: "Security Officer".into(),
                capacity: 4,
                access_tags: vec!["security".into()],
            },
            JobDefinition {
                id: "passenger".into(),
                display_name: "Passenger".into(),
                capacity: u16::MAX,
                access_tags: Vec::new(),
            },
        ]
        .into_iter()
        .map(|job| (job.id.clone(), job))
        .collect();
        Self {
            phase: RoundPhase::Lobby,
            round_id: 1,
            elapsed_ticks: 0,
            countdown_ticks: 150,
            minimum_ready_players: 1,
            players: BTreeMap::new(),
            jobs,
            end_reason: None,
        }
    }
}

impl RoundController {
    pub fn join(&mut self, peer: u64) {
        self.players.entry(peer).or_default();
    }

    pub fn leave(&mut self, peer: u64) {
        self.players.remove(&peer);
    }

    pub fn set_ready(&mut self, peer: u64, ready: bool, preferences: Vec<String>) -> bool {
        if self.phase != RoundPhase::Lobby && self.phase != RoundPhase::Starting {
            return false;
        }
        let Some(player) = self.players.get_mut(&peer) else {
            return false;
        };
        player.ready = ready;
        player.preferences = preferences
            .into_iter()
            .filter(|job| self.jobs.contains_key(job))
            .collect();
        if !ready && self.phase == RoundPhase::Starting {
            self.phase = RoundPhase::Lobby;
            self.elapsed_ticks = 0;
        }
        true
    }

    pub fn tick(&mut self) -> bool {
        match self.phase {
            RoundPhase::Lobby => {
                if self.ready_count() >= self.minimum_ready_players {
                    self.phase = RoundPhase::Starting;
                    self.elapsed_ticks = 0;
                }
            }
            RoundPhase::Starting => {
                if self.ready_count() < self.minimum_ready_players {
                    self.phase = RoundPhase::Lobby;
                    self.elapsed_ticks = 0;
                } else {
                    self.elapsed_ticks += 1;
                    if self.elapsed_ticks >= self.countdown_ticks {
                        self.assign_jobs();
                        self.phase = RoundPhase::InProgress;
                        self.elapsed_ticks = 0;
                        return true;
                    }
                }
            }
            RoundPhase::InProgress | RoundPhase::Ending => self.elapsed_ticks += 1,
            RoundPhase::Ended => {}
        }
        false
    }

    pub fn request_end(&mut self, reason: impl Into<String>) -> bool {
        if self.phase != RoundPhase::InProgress {
            return false;
        }
        self.phase = RoundPhase::Ending;
        self.elapsed_ticks = 0;
        self.end_reason = Some(reason.into());
        true
    }

    pub fn finish_end(&mut self) -> bool {
        if self.phase != RoundPhase::Ending {
            return false;
        }
        self.phase = RoundPhase::Ended;
        true
    }

    pub fn reset(&mut self) {
        self.round_id += 1;
        self.phase = RoundPhase::Lobby;
        self.elapsed_ticks = 0;
        self.end_reason = None;
        for player in self.players.values_mut() {
            player.ready = false;
            player.assigned_job = None;
        }
    }

    pub fn ready_count(&self) -> usize {
        self.players.values().filter(|player| player.ready).count()
    }

    fn assign_jobs(&mut self) {
        let mut used = HashMap::<String, u16>::new();
        let ready_peers = self
            .players
            .iter()
            .filter_map(|(peer, player)| player.ready.then_some(*peer))
            .collect::<BTreeSet<_>>();
        for peer in ready_peers {
            let preferences = self.players[&peer].preferences.clone();
            let assigned = preferences
                .into_iter()
                .find(|job| {
                    self.jobs.get(job).is_some_and(|definition| {
                        used.get(job).copied().unwrap_or_default() < definition.capacity
                    })
                })
                .unwrap_or_else(|| "passenger".into());
            *used.entry(assigned.clone()).or_default() += 1;
            self.players.get_mut(&peer).unwrap().assigned_job = Some(assigned);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_cancels_when_readiness_is_lost() {
        let mut round = RoundController {
            countdown_ticks: 2,
            ..RoundController::default()
        };
        round.join(1);
        round.set_ready(1, true, vec!["engineer".into()]);
        round.tick();
        assert_eq!(round.phase, RoundPhase::Starting);
        round.set_ready(1, false, Vec::new());
        assert_eq!(round.phase, RoundPhase::Lobby);
    }

    #[test]
    fn job_capacity_and_peer_order_are_deterministic() {
        let mut round = RoundController {
            countdown_ticks: 1,
            ..RoundController::default()
        };
        round.join(20);
        round.join(10);
        round.set_ready(20, true, vec!["captain".into()]);
        round.set_ready(10, true, vec!["captain".into()]);
        round.tick();
        assert_eq!(round.phase, RoundPhase::Starting);
        assert!(round.tick());
        assert_eq!(round.phase, RoundPhase::InProgress);
        assert_eq!(round.players[&10].assigned_job.as_deref(), Some("captain"));
        assert_eq!(
            round.players[&20].assigned_job.as_deref(),
            Some("passenger")
        );
    }
}
