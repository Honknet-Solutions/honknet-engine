use boa_engine::{
    context::{ContextBuilder, HostHooks},
    realm::Realm,
    Context, JsNativeError, JsResult, JsString, JsValue, Source,
};
use honknet_core::Entity;
use honknet_events::SignalContext;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};
use thiserror::Error;

const MAX_BUNDLE_BYTES: usize = 1024 * 1024;
const MAX_COMMAND_BYTES: usize = 256 * 1024;
const MAX_ARRAY_BUFFER_BITS: u64 = 8 * 1024 * 1024 * 8;

#[derive(Debug)]
struct SandboxHooks;

impl HostHooks for SandboxHooks {
    fn ensure_can_compile_strings(
        &self,
        _realm: Realm,
        _parameters: &[JsString],
        _body: &JsString,
        _direct: bool,
        _context: &mut Context,
    ) -> JsResult<()> {
        Err(JsNativeError::typ()
            .with_message("dynamic code compilation is disabled")
            .into())
    }

    fn utc_now(&self) -> i64 {
        0
    }

    fn local_timezone_offset_seconds(&self, _unix_time_seconds: i64) -> i32 {
        0
    }

    fn max_buffer_size(&self, _context: &mut Context) -> u64 {
        MAX_ARRAY_BUFFER_BITS
    }
}

static SANDBOX_HOOKS: SandboxHooks = SandboxHooks;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("script runtime is unavailable")]
    Unavailable,
    #[error("script runtime: {0}")]
    Runtime(String),
    #[error("script protocol: {0}")]
    Protocol(String),
}

pub type ScriptResult<T = ()> = Result<T, ScriptError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameApi {
    pub build_version: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ScriptBundle {
    pub name: String,
    pub source: String,
}

impl ScriptBundle {
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEvent {
    pub name: String,
    pub target: Option<Entity>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptWorldSnapshot {
    pub entities: Vec<ScriptEntitySnapshot>,
    pub relations: Vec<ScriptRelationSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEntitySnapshot {
    pub entity: Entity,
    pub components: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRelationSnapshot {
    pub kind: String,
    pub source: Entity,
    pub target: Entity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptTickContext {
    pub tick: u64,
    pub dt: f32,
    pub world: ScriptWorldSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ScriptCommand {
    Log {
        level: String,
        message: String,
    },
    Despawn {
        entity: Entity,
    },
    SetComponent {
        entity: Entity,
        component: String,
        value: serde_json::Value,
    },
    RemoveComponent {
        entity: Entity,
        component: String,
    },
    AddRelation {
        kind: String,
        source: Entity,
        target: Entity,
    },
    RemoveRelation {
        kind: String,
        source: Entity,
        target: Entity,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptSignalResult {
    pub commands: Vec<ScriptCommand>,
    pub signal: SignalContext,
}

pub trait GameScriptRuntime: Send {
    fn initialize(&mut self, api: GameApi) -> ScriptResult;
    fn load_bundle(&mut self, bundle: &ScriptBundle) -> ScriptResult;
    fn dispatch_event(&mut self, event: ScriptEvent, world: ScriptWorldSnapshot) -> ScriptResult;
    fn update(&mut self, context: ScriptTickContext) -> ScriptResult;
    fn dispatch_signal(&mut self, signal: SignalContext) -> ScriptResult<SignalContext>;
    fn drain_commands(&mut self) -> Vec<ScriptCommand>;
    fn shutdown(&mut self) -> ScriptResult;
}

enum Request {
    Initialize(GameApi),
    Load(ScriptBundle),
    Event(ScriptEvent, ScriptWorldSnapshot),
    Update(ScriptTickContext),
    Signal(SignalContext),
    Shutdown,
}

struct WorkerOutput {
    commands: Vec<ScriptCommand>,
    signal: Option<SignalContext>,
}

type Response = ScriptResult<WorkerOutput>;

pub struct SandboxedScriptRuntime {
    requests: SyncSender<(Request, mpsc::Sender<Response>)>,
    commands: Vec<ScriptCommand>,
    worker: Option<thread::JoinHandle<()>>,
}

impl SandboxedScriptRuntime {
    pub fn new() -> ScriptResult<Self> {
        let (requests, receiver) = mpsc::sync_channel(16);
        let worker = thread::Builder::new()
            .name("honknet-game-script".into())
            .spawn(move || worker_main(receiver))
            .map_err(|error| ScriptError::Runtime(error.to_string()))?;
        Ok(Self {
            requests,
            commands: Vec::new(),
            worker: Some(worker),
        })
    }

    fn request(&mut self, request: Request) -> ScriptResult<Option<SignalContext>> {
        let (response_tx, response_rx) = mpsc::channel();
        self.requests
            .send((request, response_tx))
            .map_err(|_| ScriptError::Unavailable)?;
        let output = response_rx.recv().map_err(|_| ScriptError::Unavailable)??;
        self.commands.extend(output.commands);
        Ok(output.signal)
    }
}

impl GameScriptRuntime for SandboxedScriptRuntime {
    fn initialize(&mut self, api: GameApi) -> ScriptResult {
        self.request(Request::Initialize(api)).map(drop)
    }

    fn load_bundle(&mut self, bundle: &ScriptBundle) -> ScriptResult {
        self.request(Request::Load(bundle.clone())).map(drop)
    }

    fn dispatch_event(&mut self, event: ScriptEvent, world: ScriptWorldSnapshot) -> ScriptResult {
        self.request(Request::Event(event, world)).map(drop)
    }

    fn update(&mut self, context: ScriptTickContext) -> ScriptResult {
        self.request(Request::Update(context)).map(drop)
    }

    fn dispatch_signal(&mut self, signal: SignalContext) -> ScriptResult<SignalContext> {
        self.request(Request::Signal(signal))?
            .ok_or_else(|| ScriptError::Protocol("signal result is missing".into()))
    }

    fn drain_commands(&mut self) -> Vec<ScriptCommand> {
        std::mem::take(&mut self.commands)
    }

    fn shutdown(&mut self) -> ScriptResult {
        if self.worker.is_none() {
            return Ok(());
        }
        self.request(Request::Shutdown)?;
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| {
                ScriptError::Runtime("script worker panicked during shutdown".into())
            })?;
        }
        Ok(())
    }
}

impl Drop for SandboxedScriptRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn worker_main(receiver: Receiver<(Request, mpsc::Sender<Response>)>) {
    let mut context = match ContextBuilder::new().host_hooks(&SANDBOX_HOOKS).build() {
        Ok(context) => context,
        Err(error) => {
            for (_, response) in receiver {
                let _ = response.send(Err(ScriptError::Runtime(error.to_string())));
            }
            return;
        }
    };
    let limits = context.runtime_limits_mut();
    limits.set_loop_iteration_limit(100_000);
    limits.set_recursion_limit(128);
    limits.set_stack_size_limit(512);

    for (request, response) in receiver {
        let should_stop = matches!(request, Request::Shutdown);
        let result = handle_request(&mut context, request);
        let _ = response.send(result);
        if should_stop {
            break;
        }
    }
}

fn handle_request(context: &mut Context, request: Request) -> Response {
    match request {
        Request::Initialize(api) => invoke_commands(context, "initialize", &api),
        Request::Load(bundle) => {
            if bundle.source.len() > MAX_BUNDLE_BYTES {
                return Err(ScriptError::Protocol(format!(
                    "bundle {} exceeds {MAX_BUNDLE_BYTES} bytes",
                    bundle.name
                )));
            }
            context
                .eval(Source::from_bytes(&bundle.source))
                .map_err(|error| ScriptError::Runtime(error.to_string()))?;
            ensure_game_contract(context)?;
            context
                .eval(Source::from_bytes(
                    "globalThis.eval = undefined; \
                     globalThis.Function = undefined; \
                     globalThis.WebAssembly = undefined; \
                     globalThis.SharedArrayBuffer = undefined; \
                     globalThis.Atomics = undefined;",
                ))
                .map_err(|error| ScriptError::Runtime(error.to_string()))?;
            Ok(WorkerOutput {
                commands: Vec::new(),
                signal: None,
            })
        }
        Request::Event(event, world) => invoke_commands(
            context,
            "dispatchEvent",
            &serde_json::json!({ "event": event, "world": world }),
        ),
        Request::Update(tick_context) => invoke_commands(context, "update", &tick_context),
        Request::Signal(signal) => {
            let result: ScriptSignalResult = invoke_json(context, "dispatchSignal", &signal)?;
            Ok(WorkerOutput {
                commands: result.commands,
                signal: Some(result.signal),
            })
        }
        Request::Shutdown => invoke_commands(context, "shutdown", &serde_json::Value::Null),
    }
}

fn ensure_game_contract(context: &mut Context) -> ScriptResult {
    let result = context
        .eval(Source::from_bytes(
            "typeof globalThis.Game === 'object' && \
             typeof Game.initialize === 'function' && \
             typeof Game.dispatchEvent === 'function' && \
             typeof Game.dispatchSignal === 'function' && \
             typeof Game.update === 'function' && \
             typeof Game.shutdown === 'function'",
        ))
        .map_err(|error| ScriptError::Runtime(error.to_string()))?;
    if result.as_boolean() == Some(true) {
        Ok(())
    } else {
        Err(ScriptError::Protocol(
            "bundle must expose globalThis.Game lifecycle methods".into(),
        ))
    }
}

fn invoke_commands<T: Serialize>(context: &mut Context, method: &str, argument: &T) -> Response {
    let commands = invoke_json(context, method, argument)?;
    Ok(WorkerOutput {
        commands,
        signal: None,
    })
}

fn invoke_json<T, R>(context: &mut Context, method: &str, argument: &T) -> ScriptResult<R>
where
    T: Serialize,
    R: for<'de> Deserialize<'de>,
{
    let argument = serde_json::to_string(argument)
        .map_err(|error| ScriptError::Protocol(error.to_string()))?;
    let source = format!("JSON.stringify(globalThis.Game.{method}({argument}) ?? [])");
    let value = context
        .eval(Source::from_bytes(&source))
        .map_err(|error| ScriptError::Runtime(error.to_string()))?;
    decode_json(value, context)
}

fn decode_json<R>(value: JsValue, context: &mut Context) -> ScriptResult<R>
where
    R: for<'de> Deserialize<'de>,
{
    let json = value
        .as_string()
        .ok_or_else(|| ScriptError::Protocol("lifecycle result is not JSON".into()))?
        .to_std_string_escaped();
    if json.len() > MAX_COMMAND_BYTES {
        return Err(ScriptError::Protocol(format!(
            "command result exceeds {MAX_COMMAND_BYTES} bytes"
        )));
    }
    let result =
        serde_json::from_str(&json).map_err(|error| ScriptError::Protocol(error.to_string()))?;
    context.clear_kept_objects();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUNDLE: &str = r#"
        "use strict";
        globalThis.Game = Object.freeze({
            initialize(api) {
                return [{ type: "log", level: "info", message: api.build_version }];
            },
            dispatchEvent(event) {
                return event.target
                    ? [{ type: "despawn", entity: event.target }]
                    : [];
            },
            dispatchSignal(signal) { return { commands: [], signal }; },
            update(context) {
                return [{ type: "log", level: "debug", message: String(context.tick) }];
            },
            shutdown() { return []; }
        });
    "#;

    #[test]
    fn executes_lifecycle_and_returns_typed_commands() {
        let mut runtime = SandboxedScriptRuntime::new().unwrap();
        runtime
            .load_bundle(&ScriptBundle::new("test", BUNDLE))
            .unwrap();
        runtime
            .initialize(GameApi {
                build_version: "test-build".into(),
                capabilities: vec![],
            })
            .unwrap();
        runtime
            .update(ScriptTickContext {
                tick: 7,
                dt: 0.1,
                world: ScriptWorldSnapshot::default(),
            })
            .unwrap();

        assert_eq!(
            runtime.drain_commands(),
            vec![
                ScriptCommand::Log {
                    level: "info".into(),
                    message: "test-build".into()
                },
                ScriptCommand::Log {
                    level: "debug".into(),
                    message: "7".into()
                }
            ]
        );
    }

    #[test]
    fn rejects_bundle_without_game_contract() {
        let mut runtime = SandboxedScriptRuntime::new().unwrap();
        let error = runtime
            .load_bundle(&ScriptBundle::new("invalid", "globalThis.value = 1;"))
            .unwrap_err();
        assert!(matches!(error, ScriptError::Protocol(_)));
    }

    #[test]
    fn interrupts_unbounded_script_loop() {
        let mut runtime = SandboxedScriptRuntime::new().unwrap();
        runtime
            .load_bundle(&ScriptBundle::new(
                "loop",
                "globalThis.Game = { initialize(){ while(true){} }, \
                 dispatchEvent(){}, dispatchSignal(signal){ return {commands: [], signal}; }, \
                 update(){}, shutdown(){} };",
            ))
            .unwrap();
        let error = runtime
            .initialize(GameApi {
                build_version: "test".into(),
                capabilities: vec![],
            })
            .unwrap_err();
        assert!(matches!(error, ScriptError::Runtime(_)));
    }

    #[test]
    fn does_not_expose_node_or_timer_apis() {
        let mut runtime = SandboxedScriptRuntime::new().unwrap();
        runtime
            .load_bundle(&ScriptBundle::new(
                "sandbox",
                r#"
                globalThis.Game = {
                    initialize() {
                        return [{
                            type: "log",
                            level: "info",
                            message: [
                                typeof process,
                                typeof require,
                                typeof setTimeout,
                                typeof WebSocket
                            ].join(",")
                        }];
                    },
                    dispatchEvent() { return []; },
                    dispatchSignal(signal) { return { commands: [], signal }; },
                    update() { return []; },
                    shutdown() { return []; }
                };
                "#,
            ))
            .unwrap();
        runtime
            .initialize(GameApi {
                build_version: "test".into(),
                capabilities: vec![],
            })
            .unwrap();

        assert_eq!(
            runtime.drain_commands(),
            vec![ScriptCommand::Log {
                level: "info".into(),
                message: "undefined,undefined,undefined,undefined".into()
            }]
        );
    }

    #[test]
    fn passes_immutable_world_snapshot_to_update() {
        let mut runtime = SandboxedScriptRuntime::new().unwrap();
        runtime
            .load_bundle(&ScriptBundle::new(
                "query",
                r#"
                globalThis.Game = {
                    initialize() { return []; },
                    dispatchEvent() { return []; },
                    dispatchSignal(signal) { return { commands: [], signal }; },
                    update(context) {
                        const pain = context.world.entities[0]
                            .components["game.status"].pain;
                        return [{ type: "log", level: "info", message: String(pain) }];
                    },
                    shutdown() { return []; }
                };
                "#,
            ))
            .unwrap();
        runtime
            .update(ScriptTickContext {
                tick: 1,
                dt: 0.1,
                world: ScriptWorldSnapshot {
                    entities: vec![ScriptEntitySnapshot {
                        entity: Entity::new(4, 2),
                        components: BTreeMap::from([(
                            "game.status".into(),
                            serde_json::json!({ "pain": 17 }),
                        )]),
                    }],
                    relations: vec![],
                },
            })
            .unwrap();

        assert_eq!(
            runtime.drain_commands(),
            vec![ScriptCommand::Log {
                level: "info".into(),
                message: "17".into()
            }]
        );
    }

    #[test]
    fn decodes_component_and_relation_commands() {
        let commands: Vec<ScriptCommand> = serde_json::from_value(serde_json::json!([
            {
                "type": "setComponent",
                "entity": { "index": 1, "generation": 0 },
                "component": "game.bodyPart",
                "value": { "zone": "head" }
            },
            {
                "type": "addRelation",
                "kind": "game.attachedTo",
                "source": { "index": 1, "generation": 0 },
                "target": { "index": 2, "generation": 0 }
            }
        ]))
        .unwrap();

        assert!(matches!(
            &commands[0],
            ScriptCommand::SetComponent { component, .. } if component == "game.bodyPart"
        ));
        assert!(matches!(
            &commands[1],
            ScriptCommand::AddRelation { kind, .. } if kind == "game.attachedTo"
        ));
    }

    #[test]
    fn script_can_mutate_and_cancel_signal() {
        let mut runtime = SandboxedScriptRuntime::new().unwrap();
        runtime
            .load_bundle(&ScriptBundle::new(
                "signal",
                r#"
                globalThis.Game = {
                    initialize() { return []; },
                    dispatchEvent() { return []; },
                    dispatchSignal(signal) {
                        signal.payload.damage -= 5;
                        signal.cancelled = signal.cancellable;
                        return {
                            commands: [{
                                type: "log",
                                level: "info",
                                message: String(signal.payload.damage)
                            }],
                            signal
                        };
                    },
                    update() { return []; },
                    shutdown() { return []; }
                };
                "#,
            ))
            .unwrap();
        let signal = SignalContext::new(
            honknet_events::SignalId::new("game.damageAttempt").unwrap(),
            honknet_events::SignalTarget::Global,
            serde_json::json!({ "damage": 12 }),
            true,
        );

        let result = runtime.dispatch_signal(signal).unwrap();

        assert_eq!(result.payload["damage"], 7);
        assert!(result.cancelled);
        assert_eq!(
            runtime.drain_commands(),
            vec![ScriptCommand::Log {
                level: "info".into(),
                message: "7".into(),
            }]
        );
    }
}
