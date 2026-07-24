pub mod components;
pub mod round;
pub mod systems;

use anyhow::{Context, Result};
use honknet_core::Entity;
use honknet_ecs::{CommandBuffer, DynamicId, StorageKind, World};
use honknet_events::{SignalContext, SignalTarget};
use honknet_math::Vec2;
use honknet_net_core::{
    BodyZoneId, EquipmentSlotId, GameAction, GameActionRequestPayload, GameActionResultPayload,
    GameActionStatus, MedicalTreatment, BUILD_VERSION,
};
use honknet_runtime::{EngineRuntime, EngineRuntimeConfig, EngineRuntimeState, PositionComponent};
use honknet_scheduler::{TickTimerQueue, TimerId};
use honknet_script::{
    GameApi, GameScriptRuntime, SandboxedScriptRuntime, ScriptBundle, ScriptCommand,
    ScriptEntitySnapshot, ScriptEvent, ScriptRelationSnapshot, ScriptTickContext,
    ScriptWorldSnapshot,
};
use std::{
    collections::{HashMap, VecDeque},
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};
use tracing::{debug, error, info, warn};

pub const GAME_NAME: &str = "Honknet";
pub const CONTENT_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/content");
pub const DEFAULT_MAP: &str = "maps/tgstation_alpha.yml";
const GAME_SCRIPT_BUNDLE: &str = include_str!("../scripts/dist/game.js");

pub struct GameApplication {
    runtime: EngineRuntime,
    round: round::RoundController,
    script_runtime: Box<dyn GameScriptRuntime>,
    action_queue: VecDeque<(u64, GameActionRequestPayload)>,
    action_results: Vec<(u64, GameActionResultPayload)>,
    last_action_sequence: HashMap<u64, u32>,
    action_cooldowns: HashMap<u64, u64>,
    last_processed_inputs: HashMap<u64, u32>,
    station_atmosphere: Option<Entity>,
    timers: TickTimerQueue<GameTimerCallback>,
}

#[derive(Debug, Clone)]
enum GameTimerCallback {
    CompleteDoAfter { actor: Entity },
}

impl GameApplication {
    pub fn new(config: EngineRuntimeConfig) -> Result<Self> {
        Ok(Self {
            runtime: EngineRuntime::new(config)?,
            round: round::RoundController::default(),
            script_runtime: Box::new(SandboxedScriptRuntime::new()?),
            action_queue: VecDeque::new(),
            action_results: Vec::new(),
            last_action_sequence: HashMap::new(),
            action_cooldowns: HashMap::new(),
            last_processed_inputs: HashMap::new(),
            station_atmosphere: None,
            timers: TickTimerQueue::default(),
        })
    }

    pub fn initialize(mut self) -> Result<Self> {
        self.initialize_engine();
        self.initialize_game_runtime();
        self.load_content_manifest()?;
        self.load_map()?;
        self.initialize_script_runtime()?;
        self.start_round();
        Ok(self)
    }

    pub fn runtime(&self) -> &EngineRuntime {
        &self.runtime
    }

    pub fn round(&self) -> &round::RoundController {
        &self.round
    }

    pub fn register_lobby_peer(&mut self, peer: u64) {
        self.round.join(peer);
    }

    pub fn set_lobby_ready(&mut self, peer: u64, ready: bool, preferences: Vec<String>) -> bool {
        self.round.set_ready(peer, ready, preferences)
    }

    pub fn request_round_end(&mut self, reason: impl Into<String>) -> bool {
        let reason = reason.into();
        if self.round.request_end(reason.clone()) {
            self.runtime
                .record_replay_marker(format!("round_end_requested:{reason}"));
            true
        } else {
            false
        }
    }

    pub fn spawn_player(&mut self, peer: u64, position: Vec2) -> Result<Entity> {
        self.round.join(peer);
        let entity = self.runtime.spawn_player(peer, position)?;
        self.runtime
            .world
            .insert(entity, components::MobStateComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::HealthComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::TargetZoneComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::CombatIntentComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::InteractionComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::HandsComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::InventoryComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::BuckledComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::ReagentHolderComponent::default())?;
        self.runtime
            .world
            .insert(entity, components::MetabolismComponent::default())?;
        if let Some(atmosphere) = self.station_atmosphere {
            self.runtime.world.insert(
                entity,
                components::BreathingEnvironmentComponent { atmosphere },
            )?;
        }
        systems::create_human_body(&mut self.runtime.world, entity)?;
        Ok(entity)
    }

    pub fn dispatch_script_event(&mut self, event: ScriptEvent) -> Result<()> {
        let snapshot = self.script_world_snapshot();
        self.script_runtime.dispatch_event(event, snapshot)?;
        self.apply_script_commands()
    }

    /// Emits through native subscribers first, then through the game script.
    pub fn emit_signal(&mut self, mut signal: SignalContext) -> Result<SignalContext> {
        match &signal.target {
            SignalTarget::Entity { entity } | SignalTarget::Component { entity, .. } => {
                anyhow::ensure!(
                    self.runtime.world.is_alive(*entity),
                    "signal target is a stale entity: {entity:?}"
                );
            }
            SignalTarget::Global => {}
        }

        self.runtime.signal_bus.emit(&mut signal);
        if signal.propagation_stopped {
            return Ok(signal);
        }

        let id = signal.id.clone();
        let target = signal.target.clone();
        let cancellable = signal.cancellable;
        let was_cancelled = signal.cancelled;
        let was_stopped = signal.propagation_stopped;
        let mut result = self.script_runtime.dispatch_signal(signal)?;
        anyhow::ensure!(
            result.id == id && result.target == target && result.cancellable == cancellable,
            "game script changed immutable signal routing metadata"
        );
        result.cancelled = cancellable && (was_cancelled || result.cancelled);
        result.propagation_stopped |= was_stopped;
        self.apply_script_commands()?;
        Ok(result)
    }

    pub fn check_access(&mut self, user: Entity, target: Entity) -> Result<bool> {
        let allowed = systems::check_user_access(&self.runtime.world, user, target);
        let signal = self.emit_signal(SignalContext::new(
            honknet_events::SignalId::new("game.accessAttempt")?,
            SignalTarget::Entity { entity: target },
            serde_json::json!({ "user": user, "target": target, "allowed": allowed }),
            true,
        ))?;
        Ok(!signal.cancelled && signal.payload["allowed"].as_bool().unwrap_or(allowed))
    }

    pub fn interact(
        &mut self,
        user: Entity,
        target: Entity,
        user_pos: Vec2,
        target_pos: Vec2,
    ) -> Result<bool> {
        let signal = self.emit_signal(SignalContext::new(
            honknet_events::SignalId::new("game.interactionAttempt")?,
            SignalTarget::Entity { entity: target },
            serde_json::json!({ "user": user, "target": target }),
            true,
        ))?;
        if signal.cancelled {
            return Ok(false);
        }
        let access_allowed = if self
            .runtime
            .world
            .contains::<components::DoorComponent>(target)
        {
            Some(self.check_access(user, target)?)
        } else {
            None
        };
        Ok(systems::interaction_system_with_access(
            &mut self.runtime.world,
            user,
            target,
            user_pos,
            target_pos,
            access_allowed,
        ))
    }

    pub fn attack(&mut self, attacker: Entity, target: Entity) -> Result<bool> {
        let intent = self
            .runtime
            .world
            .get::<components::CombatIntentComponent>(attacker)
            .map(|component| component.intent)
            .unwrap_or(components::CombatIntent::Help);
        if intent != components::CombatIntent::Harm {
            return Ok(systems::attack_entity(
                &mut self.runtime.world,
                attacker,
                target,
            ));
        }
        let (brute, burn) = systems::attack_damage(&self.runtime.world, attacker);
        let signal = self.emit_signal(SignalContext::new(
            honknet_events::SignalId::new("game.damageAttempt")?,
            SignalTarget::Entity { entity: target },
            serde_json::json!({
                "attacker": attacker,
                "target": target,
                "brute": brute,
                "burn": burn,
                "damage": brute + burn
            }),
            true,
        ))?;
        if signal.cancelled {
            return Ok(false);
        }
        let brute = signal.payload["brute"].as_f64().unwrap_or(brute as f64) as f32;
        let burn = signal.payload["burn"].as_f64().unwrap_or(burn as f64) as f32;
        Ok(systems::attack_entity_with_damage(
            &mut self.runtime.world,
            attacker,
            target,
            Some((brute, burn)),
        ))
    }

    pub fn heal(&mut self, target: Entity, amount: f32, source: &str) -> Result<bool> {
        let signal = self.emit_signal(SignalContext::new(
            honknet_events::SignalId::new("game.healAttempt")?,
            SignalTarget::Entity { entity: target },
            serde_json::json!({ "target": target, "amount": amount, "source": source }),
            true,
        ))?;
        if signal.cancelled {
            return Ok(false);
        }
        let amount = signal.payload["amount"].as_f64().unwrap_or(amount as f64) as f32;
        let Some(health) = self
            .runtime
            .world
            .get_mut::<components::HealthComponent>(target)
        else {
            return Ok(false);
        };
        health.current = (health.current + amount.max(0.0)).min(health.max);
        Ok(true)
    }

    pub fn enqueue_action(&mut self, peer: u64, request: GameActionRequestPayload) {
        let status = if self
            .last_action_sequence
            .get(&peer)
            .is_some_and(|last| request.sequence <= *last)
        {
            Some(GameActionStatus::Duplicate)
        } else if self.action_queue.len() >= 256 {
            Some(GameActionStatus::QueueFull)
        } else {
            None
        };
        if let Some(status) = status {
            self.push_action_result(peer, request.sequence, status);
            return;
        }
        self.last_action_sequence.insert(peer, request.sequence);
        self.action_queue.push_back((peer, request));
    }

    pub fn acknowledge_input(&mut self, peer: u64, sequence: u32) {
        let acknowledged = self.last_processed_inputs.entry(peer).or_default();
        *acknowledged = (*acknowledged).max(sequence);
    }

    pub fn drain_action_results(&mut self) -> Vec<(u64, GameActionResultPayload)> {
        std::mem::take(&mut self.action_results)
    }

    pub fn tick(&mut self, delta_seconds: f32) -> Result<()> {
        if self.runtime.state != EngineRuntimeState::Running {
            return Ok(());
        }
        self.process_action_queue()?;
        self.process_do_afters();
        systems::door_system(&mut self.runtime.world, delta_seconds);
        systems::power_grid_system(&mut self.runtime.world, delta_seconds);
        systems::atmosphere_system(&mut self.runtime.world, delta_seconds);
        systems::chemistry_system(&mut self.runtime.world, delta_seconds);
        systems::physiology_system(&mut self.runtime.world, delta_seconds);
        systems::health_system(&mut self.runtime.world);
        if self.round.tick() {
            self.apply_round_assignments()?;
        }
        let context = ScriptTickContext {
            tick: self.runtime.world.tick(),
            dt: delta_seconds,
            world: self.script_world_snapshot(),
        };
        self.script_runtime.update(context)?;
        self.apply_script_commands()?;
        self.runtime.tick(delta_seconds)?;
        systems::physical_interaction_system(&mut self.runtime.world, &mut self.runtime.physics);
        self.sync_game_replication();
        for entity in self.runtime.world.query::<PositionComponent>() {
            if let Some(position) = self
                .runtime
                .physics
                .bodies
                .get(&entity)
                .map(|body| body.position)
            {
                if let Some(component) = self.runtime.world.get_mut::<PositionComponent>(entity) {
                    component.0 = position;
                }
            }
        }
        Ok(())
    }

    fn apply_round_assignments(&mut self) -> Result<()> {
        let assignments = self
            .round
            .players
            .iter()
            .filter_map(|(peer, player)| {
                let job_id = player.assigned_job.as_ref()?;
                let definition = self.round.jobs.get(job_id)?;
                Some((*peer, definition.clone()))
            })
            .collect::<Vec<_>>();
        for (peer, job) in assignments {
            let Some(&player) = self.runtime.players.get(&peer) else {
                continue;
            };
            self.runtime.world.insert(
                player,
                round::JobComponent {
                    job_id: job.id.clone(),
                    display_name: job.display_name.clone(),
                },
            )?;
            let card = self.runtime.world.spawn();
            self.runtime
                .world
                .insert(card, components::ItemComponent::default())?;
            self.runtime.world.insert(
                card,
                components::WearableComponent {
                    allowed_slots: vec![components::EquipmentSlot::IdCard],
                },
            )?;
            self.runtime.world.insert(
                card,
                components::IdCardComponent {
                    owner_name: format!("Crewmember-{peer}"),
                    job_title: job.display_name,
                    access_tags: job.access_tags,
                },
            )?;
            if let Some(inventory) = self
                .runtime
                .world
                .get_mut::<components::InventoryComponent>(player)
            {
                inventory
                    .slots
                    .insert(components::EquipmentSlot::IdCard, Some(card));
            }
            if let Some(item) = self
                .runtime
                .world
                .get_mut::<components::ItemComponent>(card)
            {
                item.in_container = Some(player);
            }
        }
        Ok(())
    }

    fn sync_game_replication(&mut self) {
        let tick = self.runtime.world.tick();
        for (&peer, &entity) in &self.runtime.players {
            let mut game_components = Vec::new();
            if let Some(status) = self
                .runtime
                .world
                .get::<components::MobStateComponent>(entity)
            {
                game_components.push(honknet_replication::ComponentState::encode(
                    honknet_replication::NET_ID_MOB_STATUS,
                    tick,
                    honknet_replication::ReplicationMode::Replicated,
                    &honknet_replication::NetMobStatusComponent {
                        state: format!("{:?}", status.state),
                    },
                ));
            }
            if let Some(hands) = self.runtime.world.get::<components::HandsComponent>(entity) {
                game_components.push(honknet_replication::ComponentState::encode(
                    honknet_replication::NET_ID_HANDS,
                    tick,
                    honknet_replication::ReplicationMode::OwnerOnly,
                    &honknet_replication::NetHandsComponent {
                        active_hand: hands.active_hand_index,
                        held_item: hands.item_in_hand,
                        maximum_hands: hands.max_hands,
                    },
                ));
            }
            if let Some(inventory) = self
                .runtime
                .world
                .get::<components::InventoryComponent>(entity)
            {
                let mut slots = inventory
                    .slots
                    .iter()
                    .map(|(slot, item)| (format!("{slot:?}"), *item))
                    .collect::<Vec<_>>();
                slots.sort_by(|a, b| a.0.cmp(&b.0));
                game_components.push(honknet_replication::ComponentState::encode(
                    honknet_replication::NET_ID_EQUIPMENT,
                    tick,
                    honknet_replication::ReplicationMode::OwnerOnly,
                    &honknet_replication::NetEquipmentComponent { slots },
                ));
            }
            if let (Some(blood), Some(physiology)) = (
                self.runtime
                    .world
                    .get::<components::BloodstreamComponent>(entity),
                self.runtime
                    .world
                    .get::<components::PhysiologyComponent>(entity),
            ) {
                game_components.push(honknet_replication::ComponentState::encode(
                    honknet_replication::NET_ID_MEDICAL_STATUS,
                    tick,
                    honknet_replication::ReplicationMode::OwnerOnly,
                    &honknet_replication::NetMedicalStatusComponent {
                        blood_fraction: if blood.max_volume > 0.0 {
                            blood.volume / blood.max_volume
                        } else {
                            0.0
                        },
                        oxygen_saturation: blood.oxygen_saturation,
                        pain: physiology.pain,
                        shock: physiology.shock,
                        conscious: physiology.conscious,
                    },
                ));
            }
            let grab = self.runtime.world.get::<components::GrabComponent>(entity);
            let action = self
                .runtime
                .world
                .get::<components::DoAfterComponent>(entity);
            game_components.push(honknet_replication::ComponentState::encode(
                honknet_replication::NET_ID_INTERACTION_STATUS,
                tick,
                honknet_replication::ReplicationMode::OwnerOnly,
                &honknet_replication::NetInteractionStatusComponent {
                    grabbed: grab.map(|component| component.target),
                    grab_strength: grab.map(|component| format!("{:?}", component.strength)),
                    pulling: self
                        .runtime
                        .world
                        .get::<components::PullingComponent>(entity)
                        .map(|component| component.target),
                    carrying: self
                        .runtime
                        .world
                        .get::<components::CarryingComponent>(entity)
                        .map(|component| component.target),
                    buckled_to: self
                        .runtime
                        .world
                        .get::<components::BuckledComponent>(entity)
                        .and_then(|component| component.fixture),
                    action_kind: action.map(|component| format!("{:?}", component.kind)),
                    action_started_tick: action.map(|component| component.started_tick),
                    action_completes_tick: action.map(|component| component.completes_tick),
                },
            ));
            game_components.push(honknet_replication::ComponentState::encode(
                honknet_replication::NET_ID_PREDICTION_ACK,
                tick,
                honknet_replication::ReplicationMode::OwnerOnly,
                &honknet_replication::NetPredictionAckComponent {
                    last_processed_input: self
                        .last_processed_inputs
                        .get(&peer)
                        .copied()
                        .unwrap_or_default(),
                },
            ));

            if let Some(state) = self.runtime.replication.states.get_mut(&entity) {
                let changed = game_components.iter().any(|component| {
                    state
                        .components
                        .iter()
                        .find(|old| old.component_id == component.component_id)
                        .is_none_or(|old| old.bytes != component.bytes)
                });
                state
                    .components
                    .retain(|component| component.component_id < 100);
                state.components.extend(game_components);
                state.owner = Some(peer);
                if changed {
                    state.revision = tick;
                }
            }
        }
    }

    fn process_action_queue(&mut self) -> Result<()> {
        while let Some((peer, request)) = self.action_queue.pop_front() {
            if let Some(status) = self.process_action(peer, request.sequence, &request.action)? {
                self.push_action_result(peer, request.sequence, status);
            }
        }
        Ok(())
    }

    fn process_action(
        &mut self,
        peer: u64,
        sequence: u32,
        action: &GameAction,
    ) -> Result<Option<GameActionStatus>> {
        let Some(&actor) = self.runtime.players.get(&peer) else {
            return Ok(Some(GameActionStatus::Denied));
        };
        let current_tick = self.runtime.world.tick();
        if self
            .action_cooldowns
            .get(&peer)
            .is_some_and(|until| current_tick < *until)
        {
            return Ok(Some(GameActionStatus::Cooldown));
        }

        let target = match action {
            GameAction::Interact { target }
            | GameAction::Attack { target }
            | GameAction::Pickup { target }
            | GameAction::Bandage { target }
            | GameAction::Treat { target, .. }
            | GameAction::Cpr { target }
            | GameAction::Surgery { target, .. }
            | GameAction::Grab { target }
            | GameAction::Pull { target }
            | GameAction::Carry { target } => Some(*target),
            GameAction::Buckle { fixture } => Some(*fixture),
            GameAction::Store { container } => Some(*container),
            GameAction::ReleaseGrab
            | GameAction::StopPulling
            | GameAction::DropCarried
            | GameAction::Unbuckle
            | GameAction::Equip { .. }
            | GameAction::Unequip { .. }
            | GameAction::Drop => None,
        };
        if let Some(target) = target {
            if !self.runtime.world.is_alive(target) {
                return Ok(Some(GameActionStatus::InvalidTarget));
            }
            let (Some(actor_body), Some(target_body)) = (
                self.runtime.physics.bodies.get(&actor),
                self.runtime.physics.bodies.get(&target),
            ) else {
                return Ok(Some(GameActionStatus::InvalidTarget));
            };
            let reach = self
                .runtime
                .world
                .get::<components::InteractionComponent>(actor)
                .map(|component| component.reach_distance)
                .unwrap_or(2.5);
            if (actor_body.position - target_body.position).length() > reach {
                return Ok(Some(GameActionStatus::OutOfRange));
            }
        }

        if matches!(
            action,
            GameAction::Bandage { .. }
                | GameAction::Treat { .. }
                | GameAction::Cpr { .. }
                | GameAction::Surgery { .. }
                | GameAction::Carry { .. }
        ) {
            let target = target.expect("medical actions have a target");
            if self
                .runtime
                .world
                .contains::<components::DoAfterComponent>(actor)
            {
                return Ok(Some(GameActionStatus::Denied));
            }
            let kind = match action {
                GameAction::Bandage { .. } => components::DoAfterKind::Bandage,
                GameAction::Treat {
                    treatment: MedicalTreatment::Bandage,
                    ..
                } => components::DoAfterKind::Bandage,
                GameAction::Treat {
                    treatment: MedicalTreatment::BruisePack,
                    ..
                } => components::DoAfterKind::TreatBruise,
                GameAction::Treat {
                    treatment: MedicalTreatment::BurnGel,
                    ..
                } => components::DoAfterKind::TreatBurn,
                GameAction::Cpr { .. } => components::DoAfterKind::Cpr,
                GameAction::Surgery { .. } => components::DoAfterKind::Surgery,
                GameAction::Carry { .. } => components::DoAfterKind::Carry,
                _ => unreachable!(),
            };
            let required_treatment = match kind {
                components::DoAfterKind::Bandage => Some(components::WoundTreatment::Bandage),
                components::DoAfterKind::TreatBruise => {
                    Some(components::WoundTreatment::BruisePack)
                }
                components::DoAfterKind::TreatBurn => Some(components::WoundTreatment::BurnGel),
                components::DoAfterKind::Cpr => None,
                components::DoAfterKind::Carry => None,
                components::DoAfterKind::Surgery => None,
            };
            let supply = if let Some(required) = required_treatment {
                let supply = self
                    .runtime
                    .world
                    .get::<components::HandsComponent>(actor)
                    .and_then(|hands| hands.item_in_hand);
                let valid = supply
                    .and_then(|entity| {
                        self.runtime
                            .world
                            .get::<components::MedicalSupplyComponent>(entity)
                    })
                    .is_some_and(|item| item.treatment == required && item.charges > 0);
                if !valid {
                    return Ok(Some(GameActionStatus::Denied));
                }
                supply
            } else if let GameAction::Surgery { zone, .. } = action {
                let tool = self
                    .runtime
                    .world
                    .get::<components::HandsComponent>(actor)
                    .and_then(|hands| hands.item_in_hand);
                let valid = tool.is_some_and(|tool| {
                    systems::prepare_surgery(
                        &mut self.runtime.world,
                        target,
                        target_zone(*zone),
                        tool,
                    )
                });
                if !valid {
                    return Ok(Some(GameActionStatus::Denied));
                }
                tool
            } else {
                None
            };
            let duration_ticks = match kind {
                components::DoAfterKind::Bandage => self.runtime.config.tick_rate * 3,
                components::DoAfterKind::TreatBruise | components::DoAfterKind::TreatBurn => {
                    self.runtime.config.tick_rate * 2
                }
                components::DoAfterKind::Cpr => self.runtime.config.tick_rate * 2,
                components::DoAfterKind::Carry => self.runtime.config.tick_rate * 2,
                components::DoAfterKind::Surgery => self.runtime.config.tick_rate * 3,
            } as u64;
            let start_position = self.runtime.physics.bodies[&actor].position;
            let target_start_position = self.runtime.physics.bodies[&target].position;
            let timer_id = self.timers.schedule_at(
                current_tick + duration_ticks.max(1),
                GameTimerCallback::CompleteDoAfter { actor },
            );
            self.runtime.world.insert(
                actor,
                components::DoAfterComponent {
                    timer_id: timer_id.as_u64(),
                    peer,
                    sequence,
                    kind,
                    target,
                    supply,
                    started_tick: current_tick,
                    completes_tick: current_tick + duration_ticks.max(1),
                    start_position,
                    target_start_position,
                    max_movement: 0.5,
                },
            )?;
            self.action_cooldowns
                .insert(peer, current_tick + duration_ticks.max(1));
            return Ok(None);
        }

        let success = match action {
            GameAction::Interact { target } | GameAction::Pickup { target } => {
                let actor_position = self.runtime.physics.bodies[&actor].position;
                let target_position = self.runtime.physics.bodies[target].position;
                self.interact(actor, *target, actor_position, target_position)?
            }
            GameAction::Attack { target } => self.attack(actor, *target)?,
            GameAction::Grab { target } => systems::grab(&mut self.runtime.world, actor, *target),
            GameAction::ReleaseGrab => systems::release_grab(&mut self.runtime.world, actor),
            GameAction::Pull { target } => {
                systems::start_pulling(&mut self.runtime.world, actor, *target)
            }
            GameAction::StopPulling => systems::stop_pulling(&mut self.runtime.world, actor),
            GameAction::DropCarried => systems::drop_carried(&mut self.runtime.world, actor),
            GameAction::Buckle { fixture } => {
                systems::buckle(&mut self.runtime.world, actor, *fixture)
            }
            GameAction::Unbuckle => systems::unbuckle(&mut self.runtime.world, actor),
            GameAction::Equip { slot } => {
                systems::equip_item(&mut self.runtime.world, actor, equipment_slot(*slot))
            }
            GameAction::Unequip { slot } => {
                systems::unequip_item(&mut self.runtime.world, actor, equipment_slot(*slot))
                    .is_some()
            }
            GameAction::Store { container } => {
                let item = self
                    .runtime
                    .world
                    .get_mut::<components::HandsComponent>(actor)
                    .and_then(|hands| hands.item_in_hand.take());
                if let Some(item) = item {
                    if let Some(component) = self
                        .runtime
                        .world
                        .get_mut::<components::ItemComponent>(item)
                    {
                        component.in_container = None;
                    }
                    let stored =
                        systems::container_system(&mut self.runtime.world, *container, item);
                    if !stored {
                        self.runtime
                            .world
                            .get_mut::<components::HandsComponent>(actor)
                            .unwrap()
                            .item_in_hand = Some(item);
                        self.runtime
                            .world
                            .get_mut::<components::ItemComponent>(item)
                            .unwrap()
                            .in_container = Some(actor);
                    }
                    stored
                } else {
                    false
                }
            }
            GameAction::Drop => systems::drop_item(&mut self.runtime.world, actor).is_some(),
            GameAction::Bandage { .. }
            | GameAction::Treat { .. }
            | GameAction::Cpr { .. }
            | GameAction::Surgery { .. }
            | GameAction::Carry { .. } => {
                unreachable!()
            }
        };
        self.action_cooldowns.insert(peer, current_tick + 1);
        Ok(Some(if success {
            GameActionStatus::Success
        } else {
            GameActionStatus::Cancelled
        }))
    }

    fn process_do_afters(&mut self) {
        let current_tick = self.runtime.world.tick();
        let actors = self.runtime.world.query::<components::DoAfterComponent>();
        for actor in actors {
            let Some(action) = self
                .runtime
                .world
                .get::<components::DoAfterComponent>(actor)
                .cloned()
            else {
                continue;
            };
            let moved = self.runtime.physics.bodies.get(&actor).is_none_or(|body| {
                (body.position - action.start_position).length() > action.max_movement
            });
            let target_moved = self
                .runtime
                .physics
                .bodies
                .get(&action.target)
                .is_none_or(|body| {
                    (body.position - action.target_start_position).length() > action.max_movement
                });
            let actor_incapacitated = self
                .runtime
                .world
                .get::<components::MobStateComponent>(actor)
                .is_some_and(|state| state.state != components::MobState::Alive);
            let supply_invalid = action.supply.is_some_and(|supply| {
                let not_held = self
                    .runtime
                    .world
                    .get::<components::HandsComponent>(actor)
                    .is_none_or(|hands| hands.item_in_hand != Some(supply));
                let depleted_medical = self
                    .runtime
                    .world
                    .get::<components::MedicalSupplyComponent>(supply)
                    .is_some_and(|item| item.charges == 0);
                let recognized = self
                    .runtime
                    .world
                    .contains::<components::MedicalSupplyComponent>(supply)
                    || self
                        .runtime
                        .world
                        .contains::<components::ToolComponent>(supply);
                not_held || depleted_medical || !recognized
            });
            let invalid = moved
                || target_moved
                || actor_incapacitated
                || supply_invalid
                || !self.runtime.world.is_alive(action.target);
            if invalid {
                self.timers.cancel(TimerId::from_u64(action.timer_id));
                self.runtime
                    .world
                    .remove_component::<components::DoAfterComponent>(actor);
                self.push_action_result(action.peer, action.sequence, GameActionStatus::Cancelled);
                continue;
            }
        }

        for (timer_id, callback) in self.timers.advance(current_tick) {
            match callback {
                GameTimerCallback::CompleteDoAfter { actor } => {
                    let Some(action) = self
                        .runtime
                        .world
                        .get::<components::DoAfterComponent>(actor)
                        .cloned()
                    else {
                        continue;
                    };
                    if action.timer_id != timer_id.as_u64() {
                        continue;
                    }
                    let success = match action.kind {
                        components::DoAfterKind::Bandage => systems::bandage_most_severe_wound(
                            &mut self.runtime.world,
                            action.target,
                        ),
                        components::DoAfterKind::TreatBruise => systems::treat_most_severe_wound(
                            &mut self.runtime.world,
                            action.target,
                            components::WoundTreatment::BruisePack,
                        ),
                        components::DoAfterKind::TreatBurn => systems::treat_most_severe_wound(
                            &mut self.runtime.world,
                            action.target,
                            components::WoundTreatment::BurnGel,
                        ),
                        components::DoAfterKind::Cpr => {
                            systems::cpr_pulse(&mut self.runtime.world, action.target)
                        }
                        components::DoAfterKind::Carry => {
                            systems::start_carrying(&mut self.runtime.world, actor, action.target)
                        }
                        components::DoAfterKind::Surgery => action.supply.is_some_and(|tool| {
                            systems::complete_surgery_step(
                                &mut self.runtime.world,
                                action.target,
                                tool,
                            )
                        }),
                    };
                    if success {
                        if let Some(supply) = action.supply {
                            if let Some(item) = self
                                .runtime
                                .world
                                .get_mut::<components::MedicalSupplyComponent>(supply)
                            {
                                item.charges = item.charges.saturating_sub(1);
                            }
                        }
                    }
                    self.runtime
                        .world
                        .remove_component::<components::DoAfterComponent>(actor);
                    self.push_action_result(
                        action.peer,
                        action.sequence,
                        if success {
                            GameActionStatus::Success
                        } else {
                            GameActionStatus::Denied
                        },
                    );
                }
            }
        }
    }

    fn push_action_result(&mut self, peer: u64, sequence: u32, status: GameActionStatus) {
        self.action_results.push((
            peer,
            GameActionResultPayload {
                sequence,
                status,
                server_tick: self.runtime.world.tick(),
            },
        ));
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.script_runtime.shutdown()?;
        self.apply_script_commands()?;
        self.runtime.finalize_shutdown()?;
        Ok(())
    }

    fn initialize_engine(&mut self) {
        self.runtime.initialize();
    }

    fn initialize_game_runtime(&mut self) {
        register_game_components(&mut self.runtime.world);
        info!("{GAME_NAME} gameplay runtime initialized");
    }

    fn load_content_manifest(&mut self) -> Result<()> {
        self.runtime.state = honknet_runtime::EngineRuntimeState::LoadingContent;
        let prototypes = Path::new(CONTENT_ROOT).join("prototypes");
        let mut files = Vec::new();
        collect_yaml_files(&prototypes, &mut files)?;
        files.sort();

        let mut sources = Vec::with_capacity(files.len());
        for path in files {
            let source = fs::read_to_string(&path)
                .with_context(|| format!("failed reading prototype {}", path.display()))?;
            sources.push(source);
        }
        self.runtime
            .prototypes
            .load_yaml_batch(&sources)
            .context("failed loading fixed game prototype manifest")?;

        info!(
            "Loaded fixed {GAME_NAME} content from {}",
            prototypes.display()
        );
        Ok(())
    }

    fn load_map(&mut self) -> Result<()> {
        let map_path = Path::new(CONTENT_ROOT).join(DEFAULT_MAP);
        fs::read_to_string(&map_path)
            .with_context(|| format!("failed reading map {}", map_path.display()))?;

        let grid_entity = self.runtime.world.spawn();
        let mut station_grid = honknet_map::Grid {
            entity: grid_entity,
            transform: honknet_math::Transform2::IDENTITY,
            z_level: 0,
            parent: None,
            linear_velocity: Vec2::ZERO,
            chunks: std::collections::HashMap::new(),
            revision: 1,
        };
        station_grid
            .chunks
            .insert((0, 0), honknet_map::Chunk::new((0, 0), 1));
        self.runtime.map.grids.insert(grid_entity, station_grid);
        let atmosphere = self.runtime.world.spawn();
        self.runtime
            .world
            .insert(atmosphere, components::TileAtmosphereComponent::default())?;
        self.station_atmosphere = Some(atmosphere);
        self.runtime
            .map
            .metadata
            .insert("source".into(), DEFAULT_MAP.into());
        Ok(())
    }

    fn initialize_script_runtime(&mut self) -> Result<()> {
        self.script_runtime
            .load_bundle(&ScriptBundle::new("honknet-game", GAME_SCRIPT_BUNDLE))?;
        self.script_runtime.initialize(GameApi {
            build_version: BUILD_VERSION.to_string(),
            capabilities: vec![
                "log".into(),
                "events".into(),
                "signals".into(),
                "worldCommands".into(),
            ],
        })?;
        self.apply_script_commands()
    }

    fn apply_script_commands(&mut self) -> Result<()> {
        let mut world_commands = CommandBuffer::default();
        for command in self.script_runtime.drain_commands() {
            match command {
                ScriptCommand::Log { level, message } => match level.as_str() {
                    "debug" => debug!(target: "honknet_game_script", "{message}"),
                    "warn" => warn!(target: "honknet_game_script", "{message}"),
                    "error" => error!(target: "honknet_game_script", "{message}"),
                    _ => info!(target: "honknet_game_script", "{message}"),
                },
                ScriptCommand::Despawn { entity } => {
                    if self.runtime.world.is_alive(entity) {
                        world_commands.despawn(entity);
                    }
                }
                ScriptCommand::SetComponent {
                    entity,
                    component,
                    value,
                } => {
                    let component = DynamicId::new(component)?;
                    world_commands.try_run(move |world| {
                        world.set_dynamic_component(entity, &component, value)
                    });
                }
                ScriptCommand::RemoveComponent { entity, component } => {
                    let component = DynamicId::new(component)?;
                    world_commands.try_run(move |world| {
                        if !world.is_alive(entity) {
                            return Err(honknet_ecs::EcsError::Stale(entity));
                        }
                        world.remove_dynamic_component(entity, &component)?;
                        Ok(())
                    });
                }
                ScriptCommand::AddRelation {
                    kind,
                    source,
                    target,
                } => {
                    let kind = DynamicId::new(kind)?;
                    world_commands.try_run(move |world| {
                        world.add_relation(&kind, source, target)?;
                        Ok(())
                    });
                }
                ScriptCommand::RemoveRelation {
                    kind,
                    source,
                    target,
                } => {
                    let kind = DynamicId::new(kind)?;
                    world_commands.try_run(move |world| {
                        if !world.is_alive(source) {
                            return Err(honknet_ecs::EcsError::Stale(source));
                        }
                        world.remove_relation(&kind, source, target)?;
                        Ok(())
                    });
                }
            }
        }
        world_commands.apply(&mut self.runtime.world)?;
        Ok(())
    }

    fn script_world_snapshot(&self) -> ScriptWorldSnapshot {
        let mut entities: Vec<_> = self
            .runtime
            .world
            .entities()
            .filter_map(|entity| {
                let components = self.runtime.world.dynamic_entity_snapshot(entity);
                (!components.is_empty()).then_some(ScriptEntitySnapshot { entity, components })
            })
            .collect();
        entities.sort_by_key(|snapshot| snapshot.entity);
        let relations = self
            .runtime
            .world
            .relation_snapshot()
            .into_iter()
            .map(|relation| ScriptRelationSnapshot {
                kind: relation.kind.as_str().to_string(),
                source: relation.source,
                target: relation.target,
            })
            .collect();
        ScriptWorldSnapshot {
            entities,
            relations,
        }
    }

    fn start_round(&mut self) {
        self.runtime.ready();
        self.runtime.start();
    }
}

fn equipment_slot(slot: EquipmentSlotId) -> components::EquipmentSlot {
    match slot {
        EquipmentSlotId::Head => components::EquipmentSlot::Head,
        EquipmentSlotId::Mask => components::EquipmentSlot::Mask,
        EquipmentSlotId::Jumpsuit => components::EquipmentSlot::Jumpsuit,
        EquipmentSlotId::OuterClothing => components::EquipmentSlot::OuterClothing,
        EquipmentSlotId::Gloves => components::EquipmentSlot::Gloves,
        EquipmentSlotId::Shoes => components::EquipmentSlot::Shoes,
        EquipmentSlotId::Belt => components::EquipmentSlot::Belt,
        EquipmentSlotId::PocketLeft => components::EquipmentSlot::PocketLeft,
        EquipmentSlotId::PocketRight => components::EquipmentSlot::PocketRight,
        EquipmentSlotId::IdCard => components::EquipmentSlot::IdCard,
    }
}

fn target_zone(zone: BodyZoneId) -> components::TargetZone {
    match zone {
        BodyZoneId::Head => components::TargetZone::Head,
        BodyZoneId::Chest => components::TargetZone::Chest,
        BodyZoneId::Groin => components::TargetZone::Groin,
        BodyZoneId::LeftArm => components::TargetZone::LeftArm,
        BodyZoneId::RightArm => components::TargetZone::RightArm,
        BodyZoneId::LeftLeg => components::TargetZone::LeftLeg,
        BodyZoneId::RightLeg => components::TargetZone::RightLeg,
    }
}

impl Deref for GameApplication {
    type Target = EngineRuntime;

    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

impl DerefMut for GameApplication {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime
    }
}

fn collect_yaml_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed reading content directory {}", directory.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_yaml_files(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "yml" || extension == "yaml")
        {
            files.push(path);
        }
    }
    Ok(())
}

pub fn register_game_components(world: &mut World) {
    world.register::<components::DoorComponent>(StorageKind::Packed);
    world.register::<components::HandsComponent>(StorageKind::Packed);
    world.register::<components::ItemComponent>(StorageKind::Packed);
    world.register::<components::InventoryComponent>(StorageKind::Packed);
    world.register::<components::WearableComponent>(StorageKind::Packed);
    world.register::<components::InteractionComponent>(StorageKind::Packed);
    world.register::<components::DoAfterComponent>(StorageKind::Sparse);
    world.register::<components::GrabComponent>(StorageKind::Sparse);
    world.register::<components::PullingComponent>(StorageKind::Sparse);
    world.register::<components::CarryingComponent>(StorageKind::Sparse);
    world.register::<components::CarriedComponent>(StorageKind::Sparse);
    world.register::<components::BuckledComponent>(StorageKind::Sparse);
    world.register::<components::BuckleFixtureComponent>(StorageKind::Sparse);
    world.register::<components::ExamineComponent>(StorageKind::Packed);
    world.register::<components::ContainerComponent>(StorageKind::Packed);
    world.register::<components::MobStateComponent>(StorageKind::Packed);
    world.register::<components::HealthComponent>(StorageKind::Packed);
    world.register::<components::BodyComponent>(StorageKind::Packed);
    world.register::<components::BodyPartComponent>(StorageKind::Packed);
    world.register::<components::OrganComponent>(StorageKind::Packed);
    world.register::<components::WoundComponent>(StorageKind::Packed);
    world.register::<components::BloodstreamComponent>(StorageKind::Packed);
    world.register::<components::RespirationComponent>(StorageKind::Packed);
    world.register::<components::PhysiologyComponent>(StorageKind::Packed);
    world.register::<components::MedicalSupplyComponent>(StorageKind::Packed);
    world.register::<components::SurgeryComponent>(StorageKind::Sparse);
    world.register::<components::TargetZoneComponent>(StorageKind::Packed);
    world.register::<components::IdCardComponent>(StorageKind::Packed);
    world.register::<components::AccessReaderComponent>(StorageKind::Packed);
    world.register::<components::DoorBoltComponent>(StorageKind::Packed);
    world.register::<components::DoorPressureComponent>(StorageKind::Packed);
    world.register::<components::ToolComponent>(StorageKind::Packed);
    world.register::<components::CableComponent>(StorageKind::Packed);
    world.register::<components::SmesComponent>(StorageKind::Packed);
    world.register::<components::ApcComponent>(StorageKind::Packed);
    world.register::<components::PoweredComponent>(StorageKind::Packed);
    world.register::<components::PowerNetworkMemberComponent>(StorageKind::Packed);
    world.register::<components::TileAtmosphereComponent>(StorageKind::Packed);
    world.register::<components::AtmosConnectionComponent>(StorageKind::Packed);
    world.register::<components::AtmosZoneComponent>(StorageKind::Packed);
    world.register::<components::BreathingEnvironmentComponent>(StorageKind::Packed);
    world.register::<components::PipeComponent>(StorageKind::Packed);
    world.register::<components::ReagentHolderComponent>(StorageKind::Packed);
    world.register::<components::MetabolismComponent>(StorageKind::Packed);
    world.register::<components::ChemDispenserComponent>(StorageKind::Packed);
    world.register::<components::CombatIntentComponent>(StorageKind::Packed);
    world.register::<round::JobComponent>(StorageKind::Packed);
    world.register::<components::MeleeWeaponComponent>(StorageKind::Packed);
    for component in ["game.body", "game.bodyPart", "game.wound", "game.status"] {
        world.register_dynamic_component(
            DynamicId::new(component).expect("valid built-in dynamic component ID"),
        );
    }
    for relation in [
        "game.parent",
        "game.containedIn",
        "game.equippedTo",
        "game.attachedTo",
    ] {
        world.register_relation(DynamicId::new(relation).expect("valid built-in relation ID"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use components::*;
    use honknet_events::{SignalId, SignalTarget, SignalTargetFilter};
    use honknet_math::Vec2;
    use honknet_physics::{Body, Fixture, Shape};
    use honknet_runtime::EngineRuntimeState;
    use systems::*;

    #[test]
    fn fixed_game_application_loads_bundled_content() {
        let application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();

        assert_eq!(application.runtime().state, EngineRuntimeState::Running);
        assert_eq!(
            application.runtime().map.metadata.get("source"),
            Some(&DEFAULT_MAP.to_string())
        );
        assert!(application.runtime().prototypes.get("MobHuman").is_some());
    }

    #[test]
    fn ready_player_receives_deterministic_job_and_access_card() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let player = application.spawn_player(41, Vec2::ZERO).unwrap();
        application.round.countdown_ticks = 1;
        assert!(application.set_lobby_ready(41, true, vec!["engineer".into()]));
        application.tick(1.0 / 30.0).unwrap();
        application.tick(1.0 / 30.0).unwrap();

        assert_eq!(
            application
                .world
                .get::<round::JobComponent>(player)
                .unwrap()
                .job_id,
            "engineer"
        );
        let card = application
            .world
            .get::<InventoryComponent>(player)
            .unwrap()
            .slots[&EquipmentSlot::IdCard]
            .unwrap();
        assert!(application
            .world
            .get::<IdCardComponent>(card)
            .unwrap()
            .access_tags
            .contains(&"engineering".to_string()));
    }

    #[test]
    fn graceful_shutdown_writes_replay_and_persistence_checkpoint() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("honknet-shutdown-{unique}"));
        std::fs::create_dir_all(&root).unwrap();
        let replay = root.join("round.hnrp");
        let persistence = root.join("persistence");
        let mut application = GameApplication::new(EngineRuntimeConfig {
            persistence_path: Some(persistence.clone()),
            replay_path: Some(replay.clone()),
            ..EngineRuntimeConfig::default()
        })
        .unwrap()
        .initialize()
        .unwrap();
        application.spawn_player(42, Vec2::ZERO).unwrap();
        application.tick(1.0 / 30.0).unwrap();
        application.shutdown().unwrap();

        assert!(replay.is_file());
        assert!(std::fs::metadata(&replay).unwrap().len() > 16);
        assert!(persistence.join("game").join("checkpoint.bin").is_file());
    }

    #[test]
    fn script_world_changes_are_applied_through_commands() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let entity = application.world.spawn();

        application
            .dispatch_script_event(ScriptEvent {
                name: "despawnRequested".into(),
                target: Some(entity),
                payload: serde_json::Value::Null,
            })
            .unwrap();

        assert!(!application.world.is_alive(entity));
    }

    #[test]
    fn script_can_set_registered_dynamic_component() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let entity = application.world.spawn();
        let status = DynamicId::new("game.status").unwrap();

        application
            .dispatch_script_event(ScriptEvent {
                name: "setStatusRequested".into(),
                target: Some(entity),
                payload: serde_json::json!({ "conscious": false, "pain": 42 }),
            })
            .unwrap();

        assert_eq!(
            application
                .world
                .dynamic_component(entity, &status)
                .unwrap()
                .value,
            serde_json::json!({ "conscious": false, "pain": 42 })
        );
    }

    #[test]
    fn native_and_script_signal_handlers_run_in_defined_order() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let id = SignalId::new("game.damageAttempt").unwrap();
        application
            .signal_bus
            .subscribe(id.clone(), SignalTargetFilter::Global, 50, |signal| {
                signal.payload["damage"] = serde_json::json!(25);
            });

        let result = application
            .emit_signal(SignalContext::new(
                id,
                SignalTarget::Global,
                serde_json::json!({ "damage": -1, "blocked": true }),
                true,
            ))
            .unwrap();

        assert_eq!(result.payload["damage"], 25);
        assert!(result.cancelled);
        assert!(result.propagation_stopped);
    }

    #[test]
    fn native_propagation_stop_skips_script_handlers() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let id = SignalId::new("game.damageAttempt").unwrap();
        application
            .signal_bus
            .subscribe(id.clone(), SignalTargetFilter::Any, 0, |signal| {
                signal.stop_propagation()
            });

        let result = application
            .emit_signal(SignalContext::new(
                id,
                SignalTarget::Global,
                serde_json::json!({ "damage": -5 }),
                true,
            ))
            .unwrap();

        assert_eq!(result.payload["damage"], -5);
        assert!(result.propagation_stopped);
    }

    #[test]
    fn gameplay_actions_are_controlled_by_cancellable_signals() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let attacker = application.world.spawn();
        application
            .world
            .insert(
                attacker,
                CombatIntentComponent {
                    intent: CombatIntent::Harm,
                },
            )
            .unwrap();
        let target = application.world.spawn();
        application
            .world
            .insert(
                target,
                HealthComponent {
                    current: 50.0,
                    max: 100.0,
                },
            )
            .unwrap();
        application.signal_bus.subscribe(
            SignalId::new("game.damageAttempt").unwrap(),
            SignalTargetFilter::Entity(target),
            100,
            |signal| {
                signal.payload["brute"] = serde_json::json!(12.0);
            },
        );
        application.signal_bus.subscribe(
            SignalId::new("game.healAttempt").unwrap(),
            SignalTargetFilter::Entity(target),
            100,
            |signal| {
                signal.payload["amount"] = serde_json::json!(4.0);
            },
        );

        assert!(application.attack(attacker, target).unwrap());
        assert_eq!(
            application
                .world
                .get::<HealthComponent>(target)
                .unwrap()
                .current,
            38.0
        );
        assert!(application.heal(target, 50.0, "test").unwrap());
        assert_eq!(
            application
                .world
                .get::<HealthComponent>(target)
                .unwrap()
                .current,
            42.0
        );

        application.signal_bus.subscribe(
            SignalId::new("game.interactionAttempt").unwrap(),
            SignalTargetFilter::Entity(target),
            200,
            |signal| {
                signal.cancel();
                signal.stop_propagation();
            },
        );
        assert!(!application
            .interact(attacker, target, Vec2::ZERO, Vec2::ZERO)
            .unwrap());
    }

    #[test]
    fn access_signal_can_deny_an_otherwise_allowed_door() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let user = application.world.spawn();
        application
            .world
            .insert(user, InteractionComponent::default())
            .unwrap();
        let door = application.world.spawn();
        application
            .world
            .insert(door, DoorComponent::default())
            .unwrap();
        application.signal_bus.subscribe(
            SignalId::new("game.accessAttempt").unwrap(),
            SignalTargetFilter::Entity(door),
            100,
            |signal| {
                signal.cancel();
                signal.stop_propagation();
            },
        );

        assert!(!application
            .interact(user, door, Vec2::ZERO, Vec2::ZERO)
            .unwrap());
        assert_eq!(
            application.world.get::<DoorComponent>(door).unwrap().state,
            DoorState::Closed
        );
    }

    #[test]
    fn local_damage_creates_body_part_wound_and_blood_loss() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let patient = application.spawn_player(42, Vec2::ZERO).unwrap();
        let chest = application
            .world
            .get::<BodyComponent>(patient)
            .unwrap()
            .parts[&TargetZone::Chest];

        let wound = systems::apply_local_damage(
            &mut application.world,
            patient,
            TargetZone::Chest,
            20.0,
            0.0,
        )
        .unwrap();
        systems::physiology_system(&mut application.world, 10.0);

        let part = application.world.get::<BodyPartComponent>(chest).unwrap();
        assert_eq!(part.brute_damage, 20.0);
        let wound_state = application.world.get::<WoundComponent>(wound).unwrap();
        assert_eq!(wound_state.kind, WoundKind::Cut);
        assert_eq!(wound_state.body_part, chest);
        assert!(
            application
                .world
                .get::<BloodstreamComponent>(patient)
                .unwrap()
                .volume
                < STANDARD_BLOOD_VOLUME
        );
    }

    #[test]
    fn blood_volume_drives_conscious_state_instead_of_legacy_hp() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let patient = application.spawn_player(43, Vec2::ZERO).unwrap();
        application
            .world
            .get_mut::<BloodstreamComponent>(patient)
            .unwrap()
            .volume = STANDARD_BLOOD_VOLUME * 0.5;

        systems::health_system(&mut application.world);

        assert_eq!(
            application
                .world
                .get::<MobStateComponent>(patient)
                .unwrap()
                .state,
            MobState::Critical
        );
        assert_eq!(
            application
                .world
                .get::<HealthComponent>(patient)
                .unwrap()
                .current,
            50.0
        );
    }

    #[test]
    fn failed_breathing_causes_hypoxia_and_unconsciousness() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let patient = application.spawn_player(44, Vec2::ZERO).unwrap();
        application
            .world
            .get_mut::<RespirationComponent>(patient)
            .unwrap()
            .breathing = false;

        systems::physiology_system(&mut application.world, 9.0);
        systems::health_system(&mut application.world);

        assert!(
            application
                .world
                .get::<BloodstreamComponent>(patient)
                .unwrap()
                .oxygen_saturation
                < 0.3
        );
        assert_eq!(
            application
                .world
                .get::<MobStateComponent>(patient)
                .unwrap()
                .state,
            MobState::Unconscious
        );
    }

    #[test]
    fn bandaging_stops_wound_blood_loss() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let patient = application.spawn_player(45, Vec2::ZERO).unwrap();
        let wound = systems::apply_local_damage(
            &mut application.world,
            patient,
            TargetZone::LeftArm,
            20.0,
            0.0,
        )
        .unwrap();
        systems::physiology_system(&mut application.world, 5.0);
        let after_bleeding = application
            .world
            .get::<BloodstreamComponent>(patient)
            .unwrap()
            .volume;

        systems::bandage_wound(&mut application.world, wound).unwrap();
        systems::physiology_system(&mut application.world, 5.0);

        assert_eq!(
            application
                .world
                .get::<BloodstreamComponent>(patient)
                .unwrap()
                .volume,
            after_bleeding
        );
    }

    #[test]
    fn body_contains_functional_organs_as_attached_entities() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let patient = application.spawn_player(46, Vec2::ZERO).unwrap();
        let body = application.world.get::<BodyComponent>(patient).unwrap();

        assert_eq!(body.organs.len(), 4);
        for organ in body.organs.values() {
            assert!(application
                .world
                .get::<OrganComponent>(*organ)
                .unwrap()
                .functional());
        }
    }

    #[test]
    fn bandage_action_completes_only_after_do_after_duration() {
        let mut application = GameApplication::new(EngineRuntimeConfig {
            tick_rate: 1,
            ..EngineRuntimeConfig::default()
        })
        .unwrap()
        .initialize()
        .unwrap();
        let medic = application.spawn_player(50, Vec2::ZERO).unwrap();
        let patient = application.spawn_player(51, Vec2::new(1.0, 0.0)).unwrap();
        let wound = systems::apply_local_damage(
            &mut application.world,
            patient,
            TargetZone::LeftArm,
            20.0,
            0.0,
        )
        .unwrap();
        let gauze = application.world.spawn();
        application
            .world
            .insert(gauze, ItemComponent::default())
            .unwrap();
        application
            .world
            .insert(
                gauze,
                MedicalSupplyComponent {
                    treatment: WoundTreatment::Bandage,
                    charges: 2,
                },
            )
            .unwrap();
        application
            .world
            .get_mut::<HandsComponent>(medic)
            .unwrap()
            .item_in_hand = Some(gauze);
        application.enqueue_action(
            50,
            GameActionRequestPayload {
                sequence: 1,
                action: GameAction::Bandage { target: patient },
            },
        );

        for _ in 0..3 {
            application.tick(1.0).unwrap();
        }
        assert!(application.drain_action_results().is_empty());
        assert!(
            !application
                .world
                .get::<WoundComponent>(wound)
                .unwrap()
                .bandaged
        );

        application.tick(1.0).unwrap();
        assert_eq!(
            application.drain_action_results()[0].1.status,
            GameActionStatus::Success
        );
        assert!(
            application
                .world
                .get::<WoundComponent>(wound)
                .unwrap()
                .bandaged
        );
        assert!(!application.world.contains::<DoAfterComponent>(medic));
        assert_eq!(
            application
                .world
                .get::<MedicalSupplyComponent>(gauze)
                .unwrap()
                .charges,
            1
        );
    }

    #[test]
    fn movement_cancels_cpr_do_after() {
        let mut application = GameApplication::new(EngineRuntimeConfig {
            tick_rate: 1,
            ..EngineRuntimeConfig::default()
        })
        .unwrap()
        .initialize()
        .unwrap();
        let medic = application.spawn_player(52, Vec2::ZERO).unwrap();
        let patient = application.spawn_player(53, Vec2::new(1.0, 0.0)).unwrap();
        application.enqueue_action(
            52,
            GameActionRequestPayload {
                sequence: 1,
                action: GameAction::Cpr { target: patient },
            },
        );
        application.tick(1.0).unwrap();
        application.physics.bodies.get_mut(&medic).unwrap().position = Vec2::new(1.0, 0.0);

        application.tick(1.0).unwrap();

        assert_eq!(
            application.drain_action_results()[0].1.status,
            GameActionStatus::Cancelled
        );
        assert!(!application.world.contains::<DoAfterComponent>(medic));
    }

    #[test]
    fn wound_treatments_are_type_specific() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let patient = application.spawn_player(54, Vec2::ZERO).unwrap();
        let bruise = systems::apply_local_damage(
            &mut application.world,
            patient,
            TargetZone::RightArm,
            5.0,
            0.0,
        )
        .unwrap();

        assert!(!systems::treat_most_severe_wound(
            &mut application.world,
            patient,
            WoundTreatment::BurnGel,
        ));
        assert!(systems::treat_most_severe_wound(
            &mut application.world,
            patient,
            WoundTreatment::BruisePack,
        ));
        let wound = application.world.get::<WoundComponent>(bruise).unwrap();
        assert_eq!(wound.treatment, Some(WoundTreatment::BruisePack));
        assert_eq!(wound.damage, 2.5);
    }

    #[test]
    fn medical_action_is_denied_without_matching_supply() {
        let mut application = GameApplication::new(EngineRuntimeConfig {
            tick_rate: 1,
            ..EngineRuntimeConfig::default()
        })
        .unwrap()
        .initialize()
        .unwrap();
        application.spawn_player(55, Vec2::ZERO).unwrap();
        let patient = application.spawn_player(56, Vec2::new(1.0, 0.0)).unwrap();
        systems::apply_local_damage(&mut application.world, patient, TargetZone::Chest, 5.0, 0.0)
            .unwrap();
        application.enqueue_action(
            55,
            GameActionRequestPayload {
                sequence: 1,
                action: GameAction::Treat {
                    target: patient,
                    treatment: MedicalTreatment::BruisePack,
                },
            },
        );

        application.tick(1.0).unwrap();

        assert_eq!(
            application.drain_action_results()[0].1.status,
            GameActionStatus::Denied
        );
    }

    #[test]
    fn grabbed_character_can_be_pulled_by_authoritative_physics() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let actor = application.spawn_player(60, Vec2::ZERO).unwrap();
        let target = application.spawn_player(61, Vec2::new(1.0, 0.0)).unwrap();
        application.enqueue_action(
            60,
            GameActionRequestPayload {
                sequence: 1,
                action: GameAction::Grab { target },
            },
        );
        application.tick(1.0 / 30.0).unwrap();
        application.drain_action_results();
        application.enqueue_action(
            60,
            GameActionRequestPayload {
                sequence: 2,
                action: GameAction::Pull { target },
            },
        );
        application.tick(1.0 / 30.0).unwrap();
        application.drain_action_results();
        application.physics.bodies.get_mut(&actor).unwrap().position = Vec2::new(3.0, 0.0);

        application.tick(1.0 / 30.0).unwrap();

        assert!(
            (application.physics.bodies[&target].position - Vec2::new(2.0, 0.0)).length() < 0.01
        );
        assert_eq!(
            application
                .world
                .get::<PullingComponent>(actor)
                .unwrap()
                .target,
            target
        );
    }

    #[test]
    fn buckled_character_follows_fixture_and_can_unbuckle() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let passenger = application.spawn_player(62, Vec2::ZERO).unwrap();
        let fixture = application.world.spawn();
        application
            .world
            .insert(fixture, BuckleFixtureComponent::default())
            .unwrap();
        application.physics.insert(Body::dynamic(
            fixture,
            Vec2::new(1.0, 0.0),
            10.0,
            Fixture {
                shape: Shape::Circle { radius: 0.5 },
                friction: 0.5,
                restitution: 0.0,
                sensor: false,
                layer: 1,
                mask: 1,
            },
        ));
        application.enqueue_action(
            62,
            GameActionRequestPayload {
                sequence: 1,
                action: GameAction::Buckle { fixture },
            },
        );
        application.tick(1.0 / 30.0).unwrap();
        application
            .physics
            .bodies
            .get_mut(&fixture)
            .unwrap()
            .position = Vec2::new(2.0, 0.0);

        application.tick(1.0 / 30.0).unwrap();

        assert_eq!(
            application
                .world
                .get::<BuckledComponent>(passenger)
                .unwrap()
                .fixture,
            Some(fixture)
        );
        assert!(
            (application.physics.bodies[&passenger].position
                - application.physics.bodies[&fixture].position)
                .length()
                < 0.01
        );
        application.enqueue_action(
            62,
            GameActionRequestPayload {
                sequence: 2,
                action: GameAction::Unbuckle,
            },
        );
        application.tick(1.0 / 30.0).unwrap();
        assert_eq!(
            application
                .world
                .get::<BuckledComponent>(passenger)
                .unwrap()
                .fixture,
            None
        );
    }

    #[test]
    fn equipment_rejects_incompatible_items_and_preserves_hand() {
        let mut world = World::default();
        register_game_components(&mut world);
        let user = world.spawn();
        world.insert(user, HandsComponent::default()).unwrap();
        world.insert(user, InventoryComponent::default()).unwrap();
        let item = world.spawn();
        world.insert(item, ItemComponent::default()).unwrap();
        world
            .insert(
                item,
                WearableComponent {
                    allowed_slots: vec![EquipmentSlot::Head],
                },
            )
            .unwrap();
        assert!(pick_up_item(&mut world, user, item));

        assert!(!equip_item(&mut world, user, EquipmentSlot::Jumpsuit));
        assert_eq!(
            world.get::<HandsComponent>(user).unwrap().item_in_hand,
            Some(item)
        );
        assert_eq!(
            world.get::<InventoryComponent>(user).unwrap().slots[&EquipmentSlot::Jumpsuit],
            None
        );
    }

    #[test]
    fn container_capacity_uses_item_size_not_item_count() {
        let mut world = World::default();
        register_game_components(&mut world);
        let container = world.spawn();
        world
            .insert(
                container,
                ContainerComponent {
                    capacity: 3,
                    contents: Vec::new(),
                },
            )
            .unwrap();
        let first = world.spawn();
        let second = world.spawn();
        world
            .insert(
                first,
                ItemComponent {
                    size: 2,
                    ..ItemComponent::default()
                },
            )
            .unwrap();
        world
            .insert(
                second,
                ItemComponent {
                    size: 2,
                    ..ItemComponent::default()
                },
            )
            .unwrap();

        assert!(container_system(&mut world, container, first));
        assert!(!container_system(&mut world, container, second));
        assert_eq!(
            world.get::<ContainerComponent>(container).unwrap().contents,
            vec![first]
        );
    }

    #[test]
    fn aggressive_grab_allows_timed_carry_of_incapacitated_character() {
        let mut application = GameApplication::new(EngineRuntimeConfig {
            tick_rate: 1,
            ..EngineRuntimeConfig::default()
        })
        .unwrap()
        .initialize()
        .unwrap();
        let carrier = application.spawn_player(70, Vec2::ZERO).unwrap();
        let patient = application.spawn_player(71, Vec2::new(1.0, 0.0)).unwrap();
        application
            .world
            .get_mut::<BloodstreamComponent>(patient)
            .unwrap()
            .volume = STANDARD_BLOOD_VOLUME * 0.5;

        for sequence in 1..=2 {
            application.enqueue_action(
                70,
                GameActionRequestPayload {
                    sequence,
                    action: GameAction::Grab { target: patient },
                },
            );
            application.tick(1.0).unwrap();
            application.drain_action_results();
        }
        assert_eq!(
            application
                .world
                .get::<GrabComponent>(carrier)
                .unwrap()
                .strength,
            GrabStrength::Aggressive
        );

        application.enqueue_action(
            70,
            GameActionRequestPayload {
                sequence: 3,
                action: GameAction::Carry { target: patient },
            },
        );
        for _ in 0..3 {
            application.tick(1.0).unwrap();
        }
        assert_eq!(
            application
                .world
                .get::<CarryingComponent>(carrier)
                .unwrap()
                .target,
            patient
        );
        assert_eq!(
            application
                .world
                .get::<CarriedComponent>(patient)
                .unwrap()
                .carrier,
            carrier
        );

        application
            .physics
            .bodies
            .get_mut(&carrier)
            .unwrap()
            .position = Vec2::new(2.0, 0.0);
        application.tick(1.0).unwrap();
        assert_eq!(
            application.physics.bodies[&patient].position,
            application.physics.bodies[&carrier].position
        );
    }

    #[test]
    fn test_interaction_and_door_toggle() {
        let mut world = World::default();
        register_game_components(&mut world);

        let user = world.spawn();
        world.insert(user, InteractionComponent::default()).unwrap();

        let door = world.spawn();
        world.insert(door, DoorComponent::default()).unwrap();

        // 1. User interacts with door within reach
        let success = interaction_system(&mut world, user, door, Vec2::ZERO, Vec2::new(1.0, 0.0));
        assert!(success);
        assert_eq!(
            world.get::<DoorComponent>(door).unwrap().state,
            DoorState::Open
        );

        // 2. Door auto-closes after delay
        door_system(&mut world, 3.5);
        assert_eq!(
            world.get::<DoorComponent>(door).unwrap().state,
            DoorState::Closed
        );
    }

    #[test]
    fn test_hands_pickup_and_drop() {
        let mut world = World::default();
        register_game_components(&mut world);

        let user = world.spawn();
        world.insert(user, HandsComponent::default()).unwrap();
        world.insert(user, InteractionComponent::default()).unwrap();

        let toolbox = world.spawn();
        world.insert(toolbox, ItemComponent::default()).unwrap();

        // Pickup item
        let picked = pick_up_item(&mut world, user, toolbox);
        assert!(picked);
        assert_eq!(
            world.get::<HandsComponent>(user).unwrap().item_in_hand,
            Some(toolbox)
        );
        assert_eq!(
            world.get::<ItemComponent>(toolbox).unwrap().in_container,
            Some(user)
        );

        // Drop item
        let dropped = drop_item(&mut world, user);
        assert_eq!(dropped, Some(toolbox));
        assert_eq!(
            world.get::<HandsComponent>(user).unwrap().item_in_hand,
            None
        );
        assert_eq!(
            world.get::<ItemComponent>(toolbox).unwrap().in_container,
            None
        );
    }

    #[test]
    fn test_health_and_mob_state() {
        let mut world = World::default();
        register_game_components(&mut world);

        let player = world.spawn();
        world
            .insert(
                player,
                HealthComponent {
                    current: 100.0,
                    max: 100.0,
                },
            )
            .unwrap();
        world.insert(player, MobStateComponent::default()).unwrap();

        // Damage player to 15 HP (Critical)
        world.get_mut::<HealthComponent>(player).unwrap().current = 15.0;
        health_system(&mut world);
        assert_eq!(
            world.get::<MobStateComponent>(player).unwrap().state,
            MobState::Critical
        );

        // Damage player to 0 HP (Dead)
        world.get_mut::<HealthComponent>(player).unwrap().current = 0.0;
        health_system(&mut world);
        assert_eq!(
            world.get::<MobStateComponent>(player).unwrap().state,
            MobState::Dead
        );
    }

    #[test]
    fn test_inventory_equip_and_unequip() {
        let mut world = World::default();
        register_game_components(&mut world);

        let user = world.spawn();
        world.insert(user, HandsComponent::default()).unwrap();
        world.insert(user, InventoryComponent::default()).unwrap();

        let jumpsuit = world.spawn();
        world.insert(jumpsuit, ItemComponent::default()).unwrap();
        world
            .insert(
                jumpsuit,
                WearableComponent {
                    allowed_slots: vec![EquipmentSlot::Jumpsuit],
                },
            )
            .unwrap();

        // 1. Pick up jumpsuit in hand
        pick_up_item(&mut world, user, jumpsuit);
        assert_eq!(
            world.get::<HandsComponent>(user).unwrap().item_in_hand,
            Some(jumpsuit)
        );

        // 2. Equip jumpsuit from hand to Jumpsuit slot
        let equipped = equip_item(&mut world, user, EquipmentSlot::Jumpsuit);
        assert!(equipped);
        assert_eq!(
            world.get::<HandsComponent>(user).unwrap().item_in_hand,
            None
        );
        assert_eq!(
            world
                .get::<InventoryComponent>(user)
                .unwrap()
                .slots
                .get(&EquipmentSlot::Jumpsuit),
            Some(&Some(jumpsuit))
        );

        // 3. Unequip jumpsuit from Jumpsuit slot back to hand
        let unequipped = unequip_item(&mut world, user, EquipmentSlot::Jumpsuit);
        assert_eq!(unequipped, Some(jumpsuit));
        assert_eq!(
            world.get::<HandsComponent>(user).unwrap().item_in_hand,
            Some(jumpsuit)
        );
        assert_eq!(
            world
                .get::<InventoryComponent>(user)
                .unwrap()
                .slots
                .get(&EquipmentSlot::Jumpsuit),
            Some(&None)
        );
    }

    #[test]
    fn test_id_card_access_control() {
        let mut world = World::default();
        register_game_components(&mut world);

        let user = world.spawn();
        world.insert(user, HandsComponent::default()).unwrap();
        world.insert(user, InventoryComponent::default()).unwrap();
        world.insert(user, InteractionComponent::default()).unwrap();

        let restricted_door = world.spawn();
        world
            .insert(restricted_door, DoorComponent::default())
            .unwrap();
        world
            .insert(
                restricted_door,
                AccessReaderComponent {
                    required_tags: vec!["Captain".to_string()],
                },
            )
            .unwrap();

        // 1. Without Captain ID card: Access Denied
        let denied = interaction_system(&mut world, user, restricted_door, Vec2::ZERO, Vec2::ZERO);
        assert!(!denied);
        assert_eq!(
            world.get::<DoorComponent>(restricted_door).unwrap().state,
            DoorState::Closed
        );

        // 2. Equip Captain ID Card into IdCard slot: Access Granted
        let captain_id = world.spawn();
        world.insert(captain_id, ItemComponent::default()).unwrap();
        world
            .insert(
                captain_id,
                WearableComponent {
                    allowed_slots: vec![EquipmentSlot::IdCard],
                },
            )
            .unwrap();
        world
            .insert(
                captain_id,
                IdCardComponent {
                    owner_name: "Captain".to_string(),
                    job_title: "Captain".to_string(),
                    access_tags: vec!["Captain".to_string()],
                },
            )
            .unwrap();

        pick_up_item(&mut world, user, captain_id);
        equip_item(&mut world, user, EquipmentSlot::IdCard);

        let granted = interaction_system(&mut world, user, restricted_door, Vec2::ZERO, Vec2::ZERO);
        assert!(granted);
        assert_eq!(
            world.get::<DoorComponent>(restricted_door).unwrap().state,
            DoorState::Open
        );
    }

    #[test]
    fn test_bolted_door_denial() {
        let mut world = World::default();
        register_game_components(&mut world);

        let user = world.spawn();
        world.insert(user, InteractionComponent::default()).unwrap();

        let door = world.spawn();
        world.insert(door, DoorComponent::default()).unwrap();
        world
            .insert(door, DoorBoltComponent { is_bolted: true })
            .unwrap();

        // Bolted door cannot be opened via normal interaction
        let toggled = interaction_system(&mut world, user, door, Vec2::ZERO, Vec2::ZERO);
        assert!(!toggled);
        assert_eq!(
            world.get::<DoorComponent>(door).unwrap().state,
            DoorState::Closed
        );
    }

    #[test]
    fn door_refuses_to_open_without_power_or_across_unsafe_pressure() {
        let mut world = World::default();
        register_game_components(&mut world);
        let user = world.spawn();
        world.insert(user, InteractionComponent::default()).unwrap();
        let first = world.spawn();
        let second = world.spawn();
        world
            .insert(first, TileAtmosphereComponent::default())
            .unwrap();
        world
            .insert(
                second,
                TileAtmosphereComponent {
                    air: GasMix {
                        oxygen: 0.0,
                        nitrogen: 0.0,
                        carbon_dioxide: 0.0,
                        plasma: 0.0,
                        temperature: 2.7,
                    },
                    ..TileAtmosphereComponent::default()
                },
            )
            .unwrap();
        let door = world.spawn();
        world.insert(door, DoorComponent::default()).unwrap();
        world
            .insert(
                door,
                PoweredComponent {
                    is_powered: false,
                    ..PoweredComponent::default()
                },
            )
            .unwrap();
        world
            .insert(
                door,
                DoorPressureComponent {
                    first_atmosphere: first,
                    second_atmosphere: second,
                    maximum_safe_delta_kpa: 1.0,
                },
            )
            .unwrap();

        assert!(!interaction_system(
            &mut world,
            user,
            door,
            Vec2::ZERO,
            Vec2::ZERO
        ));
        world.get_mut::<PoweredComponent>(door).unwrap().is_powered = true;
        assert!(!interaction_system(
            &mut world,
            user,
            door,
            Vec2::ZERO,
            Vec2::ZERO
        ));
        world
            .get_mut::<TileAtmosphereComponent>(second)
            .unwrap()
            .air = GasMix::default();
        assert!(interaction_system(
            &mut world,
            user,
            door,
            Vec2::ZERO,
            Vec2::ZERO
        ));
    }

    #[test]
    fn test_power_grid_apc_depletion() {
        let mut world = World::default();
        register_game_components(&mut world);

        let apc = world.spawn();
        world
            .insert(
                apc,
                ApcComponent {
                    cell_charge: 10.0,
                    ..Default::default()
                },
            )
            .unwrap();

        let device = world.spawn();
        world.insert(device, PoweredComponent::default()).unwrap();

        // 1. Tick power grid -> depletes APC cell and turns off power
        power_grid_system(&mut world, 1.0);
        assert_eq!(world.get::<ApcComponent>(apc).unwrap().cell_charge, 0.0);
        assert!(!world.get::<ApcComponent>(apc).unwrap().equipment_powered);
        assert!(!world.get::<PoweredComponent>(device).unwrap().is_powered);
    }

    #[test]
    fn smes_output_is_conserved_across_multiple_apcs() {
        let mut world = World::default();
        register_game_components(&mut world);
        let first_apc = world.spawn();
        let second_apc = world.spawn();
        for apc in [first_apc, second_apc] {
            world
                .insert(
                    apc,
                    ApcComponent {
                        cell_charge: 0.0,
                        cell_capacity: 100.0,
                        ..ApcComponent::default()
                    },
                )
                .unwrap();
            world
                .insert(
                    apc,
                    PowerNetworkMemberComponent {
                        network: 4,
                        channel: PowerChannel::Equipment,
                    },
                )
                .unwrap();
        }
        let smes = world.spawn();
        world
            .insert(
                smes,
                SmesComponent {
                    charge: 100.0,
                    max_capacity: 100.0,
                    input_rate: 0.0,
                    output_rate: 100.0,
                },
            )
            .unwrap();
        world
            .insert(
                smes,
                PowerNetworkMemberComponent {
                    network: 4,
                    channel: PowerChannel::Equipment,
                },
            )
            .unwrap();

        power_grid_system(&mut world, 1.0);

        let stored = [first_apc, second_apc]
            .into_iter()
            .map(|entity| world.get::<ApcComponent>(entity).unwrap().cell_charge)
            .sum::<f32>();
        assert!((stored - 100.0).abs() < 0.001);
        assert_eq!(world.get::<SmesComponent>(smes).unwrap().charge, 0.0);
    }

    #[test]
    fn test_linda_plasma_fire() {
        let mut world = World::default();
        register_game_components(&mut world);

        let tile = world.spawn();
        world
            .insert(
                tile,
                TileAtmosphereComponent {
                    air: GasMix {
                        oxygen: 50.0,
                        plasma: 50.0,
                        temperature: 400.0, // Above ignition point 373.15K
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();

        // Tick atmos -> triggers plasma combustion
        atmosphere_system(&mut world, 1.0);
        let air = &world.get::<TileAtmosphereComponent>(tile).unwrap().air;
        assert!(air.plasma < 50.0);
        assert!(air.temperature > 400.0);
    }

    #[test]
    fn air_equalizes_only_across_an_open_pressure_boundary() {
        let mut world = World::default();
        register_game_components(&mut world);
        let first = world.spawn();
        let second = world.spawn();
        let door = world.spawn();
        let connection = world.spawn();
        world
            .insert(
                first,
                TileAtmosphereComponent {
                    air: GasMix {
                        oxygen: 100.0,
                        nitrogen: 0.0,
                        carbon_dioxide: 0.0,
                        plasma: 0.0,
                        temperature: 293.15,
                    },
                    volume: 100.0,
                    is_space: false,
                },
            )
            .unwrap();
        world
            .insert(
                second,
                TileAtmosphereComponent {
                    air: GasMix {
                        oxygen: 0.0,
                        nitrogen: 0.0,
                        carbon_dioxide: 0.0,
                        plasma: 0.0,
                        temperature: 293.15,
                    },
                    volume: 100.0,
                    is_space: false,
                },
            )
            .unwrap();
        world.insert(door, DoorComponent::default()).unwrap();
        world
            .insert(
                connection,
                AtmosConnectionComponent {
                    first,
                    second,
                    conductance: 0.5,
                    barrier: Some(door),
                },
            )
            .unwrap();

        atmosphere_system(&mut world, 1.0);
        assert_eq!(
            world
                .get::<TileAtmosphereComponent>(second)
                .unwrap()
                .air
                .oxygen,
            0.0
        );
        world.get_mut::<DoorComponent>(door).unwrap().state = DoorState::Open;
        atmosphere_system(&mut world, 1.0);
        let first_oxygen = world
            .get::<TileAtmosphereComponent>(first)
            .unwrap()
            .air
            .oxygen;
        let second_oxygen = world
            .get::<TileAtmosphereComponent>(second)
            .unwrap()
            .air
            .oxygen;
        assert!(first_oxygen < 100.0);
        assert!(second_oxygen > 0.0);
        assert!((first_oxygen + second_oxygen - 100.0).abs() < 0.001);
    }

    #[test]
    fn spawned_players_breathe_the_authoritative_station_atmosphere() {
        let mut application = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let player = application.spawn_player(80, Vec2::ZERO).unwrap();
        let atmosphere = application
            .world
            .get::<BreathingEnvironmentComponent>(player)
            .unwrap()
            .atmosphere;
        application
            .world
            .get_mut::<TileAtmosphereComponent>(atmosphere)
            .unwrap()
            .air
            .oxygen = 0.0;

        application.tick(1.0).unwrap();

        assert_eq!(
            application
                .world
                .get::<RespirationComponent>(player)
                .unwrap()
                .external_oxygen,
            0.0
        );
        assert!(
            application
                .world
                .get::<BloodstreamComponent>(player)
                .unwrap()
                .oxygen_saturation
                < 1.0
        );
    }

    #[test]
    fn test_chemistry_bicaridine_healing() {
        let mut world = World::default();
        register_game_components(&mut world);

        let patient = world.spawn();
        create_human_body(&mut world, patient).unwrap();
        apply_local_damage(&mut world, patient, TargetZone::LeftArm, 20.0, 0.0).unwrap();
        let arm = world.get::<BodyComponent>(patient).unwrap().parts[&TargetZone::LeftArm];
        let before = world.get::<BodyPartComponent>(arm).unwrap().brute_damage;
        world
            .insert(
                patient,
                ReagentHolderComponent {
                    max_volume: 50.0,
                    reagents: vec![ReagentVolume {
                        id: ReagentId::Bicaridine,
                        volume: 10.0,
                    }],
                },
            )
            .unwrap();
        world
            .insert(patient, MetabolismComponent::default())
            .unwrap();

        chemistry_system(&mut world, 1.0);
        assert!(world.get::<BodyPartComponent>(arm).unwrap().brute_damage < before);
        assert_eq!(
            world
                .get::<HealthComponent>(patient)
                .map(|health| health.current),
            None
        );
    }

    #[test]
    fn surgery_requires_ordered_tools_and_completes_as_timed_actions() {
        let mut application = GameApplication::new(EngineRuntimeConfig {
            tick_rate: 1,
            ..EngineRuntimeConfig::default()
        })
        .unwrap()
        .initialize()
        .unwrap();
        let surgeon = application.spawn_player(90, Vec2::ZERO).unwrap();
        let patient = application.spawn_player(91, Vec2::new(1.0, 0.0)).unwrap();
        apply_local_damage(
            &mut application.world,
            patient,
            TargetZone::Chest,
            40.0,
            0.0,
        )
        .unwrap();
        let chest = application
            .world
            .get::<BodyComponent>(patient)
            .unwrap()
            .parts[&TargetZone::Chest];
        let before = application
            .world
            .get::<BodyPartComponent>(chest)
            .unwrap()
            .brute_damage;

        for (index, tool_type) in [
            ToolType::Scalpel,
            ToolType::Hemostat,
            ToolType::Retractor,
            ToolType::Hemostat,
            ToolType::Cautery,
        ]
        .into_iter()
        .enumerate()
        {
            let tool = application.world.spawn();
            application
                .world
                .insert(
                    tool,
                    ItemComponent {
                        in_container: Some(surgeon),
                        ..ItemComponent::default()
                    },
                )
                .unwrap();
            application
                .world
                .insert(
                    tool,
                    ToolComponent {
                        tool_type,
                        use_delay: 1.0,
                    },
                )
                .unwrap();
            application
                .world
                .get_mut::<HandsComponent>(surgeon)
                .unwrap()
                .item_in_hand = Some(tool);
            application.enqueue_action(
                90,
                GameActionRequestPayload {
                    sequence: index as u32 + 1,
                    action: GameAction::Surgery {
                        target: patient,
                        zone: BodyZoneId::Chest,
                    },
                },
            );
            for _ in 0..4 {
                application.tick(1.0).unwrap();
            }
            assert_eq!(
                application.drain_action_results().last().unwrap().1.status,
                GameActionStatus::Success
            );
        }

        let surgery = application.world.get::<SurgeryComponent>(patient).unwrap();
        assert_eq!(surgery.step, SurgeryStep::Complete);
        assert!(!surgery.incision_open);
        assert!(
            application
                .world
                .get::<BodyPartComponent>(chest)
                .unwrap()
                .brute_damage
                < before
        );
    }

    #[test]
    fn test_combat_harm_intent_attack() {
        let mut world = World::default();
        register_game_components(&mut world);

        let attacker = world.spawn();
        world
            .insert(
                attacker,
                CombatIntentComponent {
                    intent: CombatIntent::Harm,
                },
            )
            .unwrap();
        world
            .insert(
                attacker,
                TargetZoneComponent {
                    active_zone: TargetZone::Head,
                },
            )
            .unwrap();

        let victim = world.spawn();
        world
            .insert(
                victim,
                HealthComponent {
                    current: 100.0,
                    max: 100.0,
                },
            )
            .unwrap();

        // Attack victim with Harm intent
        let hit = attack_entity(&mut world, attacker, victim);
        assert!(hit);
        assert!(world.get::<HealthComponent>(victim).unwrap().current < 100.0);
    }
}
