use honknet_client_runtime::{ClientConnectionState, ClientRuntime};
use honknet_math::Vec2;
use honknet_net_core::{
    decode_message, encode_message_envelope, GameAction, GameActionRequestPayload,
    GameActionResultPayload, LobbyReadyPayload, LobbyStatePayload, NetworkMessage,
    NetworkPacketEnvelope, ServerWelcomePayload, BUILD_VERSION, CONTENT_MANIFEST_ID,
    CONTENT_VERSION,
};
use honknet_replication::{Delta, Snapshot};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmClientRuntime {
    runtime: ClientRuntime,
    session_peer_id: u64,
    reconnect_token: Option<String>,
    action_results: Vec<GameActionResultPayload>,
    lobby_state: Option<LobbyStatePayload>,
}

impl Default for WasmClientRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmClientRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self {
            runtime: ClientRuntime::new(),
            session_peer_id: 0,
            reconnect_token: None,
            action_results: Vec::new(),
            lobby_state: None,
        }
    }

    pub fn initialize_client(&mut self) -> Result<(), JsValue> {
        self.runtime.set_state(ClientConnectionState::Disconnected);
        Ok(())
    }

    pub fn connect_client(&mut self, _url: &str) -> Result<(), JsValue> {
        self.runtime
            .set_state(ClientConnectionState::TransportConnecting);
        Ok(())
    }

    pub fn disconnect_client(&mut self) {
        self.runtime.set_state(ClientConnectionState::Disconnected);
        self.runtime.entity_mapping = Default::default();
        self.runtime.world = Default::default();
    }

    pub fn push_network_message(&mut self, data: &[u8]) -> Result<(), JsValue> {
        if let Ok((env, payload)) = NetworkPacketEnvelope::decode(data) {
            let compressed = (env.flags & 4) != 0;
            match env.message_id {
                Snapshot::ID => {
                    if let Ok(snapshot) =
                        decode_message::<Snapshot>(payload, compressed, 1024 * 1024)
                    {
                        self.runtime.apply_snapshot(&snapshot);
                        self.runtime.set_state(ClientConnectionState::Active);
                    }
                }
                Delta::ID => {
                    if let Ok(delta) = decode_message::<Delta>(payload, compressed, 1024 * 1024) {
                        self.runtime.apply_delta(&delta);
                        self.runtime.set_state(ClientConnectionState::Active);
                    }
                }
                ServerWelcomePayload::ID => {
                    if let Ok(welcome) =
                        decode_message::<ServerWelcomePayload>(payload, compressed, 1024)
                    {
                        self.session_peer_id = welcome.peer_id;
                        self.reconnect_token = welcome.reconnect_token;
                        self.runtime.set_state(ClientConnectionState::Active);
                    }
                }
                GameActionResultPayload::ID => {
                    if let Ok(result) =
                        decode_message::<GameActionResultPayload>(payload, compressed, 1024)
                    {
                        self.action_results.push(result);
                    }
                }
                LobbyStatePayload::ID => {
                    if let Ok(state) =
                        decode_message::<LobbyStatePayload>(payload, compressed, 4096)
                    {
                        self.lobby_state = Some(state);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn push_input(&mut self, sequence: u32, x: f32, y: f32) {
        self.runtime.enqueue_input(sequence as u64, Vec2::new(x, y));
    }

    pub fn tick_client(&mut self, delta_seconds: f32) -> Result<(), JsValue> {
        self.runtime
            .tick(delta_seconds)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn apply_snapshot(&mut self, data: &[u8]) -> Result<(), JsValue> {
        if let Ok(snapshot) = decode_message::<Snapshot>(data, false, 1024 * 1024) {
            self.runtime.apply_snapshot(&snapshot);
        }
        Ok(())
    }

    pub fn apply_delta(&mut self, data: &[u8]) -> Result<(), JsValue> {
        if let Ok(delta) = decode_message::<Delta>(data, false, 1024 * 1024) {
            self.runtime.apply_delta(&delta);
        }
        Ok(())
    }

    pub fn extract_render_frame(&mut self) -> JsValue {
        let frame = self.runtime.extract_render_frame();
        serde_wasm_bindgen::to_value(&frame).unwrap_or(JsValue::NULL)
    }

    pub fn get_hud_state(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.runtime.owner_hud_state()).unwrap_or(JsValue::NULL)
    }

    pub fn ack_render_frame(&mut self, tick: u64) {
        self.runtime.last_acked_baseline = tick;
    }

    pub fn create_input_payload(&self, sequence: u32, x: f32, y: f32) -> Vec<u8> {
        let input = honknet_net_core::ClientInputPayload {
            sequence,
            movement: Vec2::new(x, y),
        };
        encode_message_envelope(&input, self.runtime.client_tick, false).unwrap_or_default()
    }

    pub fn create_hello_payload(&self) -> Vec<u8> {
        let hello = honknet_net_core::ClientHelloPayload {
            protocol_version: honknet_net_core::PROTOCOL_VERSION,
            engine_version: BUILD_VERSION.to_string(),
            content_version: CONTENT_VERSION.to_string(),
            content_manifest_hash: CONTENT_MANIFEST_ID.to_string(),
            supported_compression: vec![],
            auth_token: Some("auth-ok".to_string()),
            reconnect_token: self.reconnect_token.clone(),
        };
        encode_message_envelope(&hello, self.runtime.client_tick, false).unwrap_or_default()
    }

    pub fn create_ack_payload(&self, acked_tick: u64) -> Vec<u8> {
        let ack = honknet_net_core::StateAckPayload { acked_tick };
        encode_message_envelope(&ack, self.runtime.client_tick, false).unwrap_or_default()
    }

    pub fn create_lobby_ready_payload(&self, ready: bool, preferred_job: &str) -> Vec<u8> {
        encode_message_envelope(
            &LobbyReadyPayload {
                ready,
                preferred_jobs: vec![preferred_job.to_string()],
            },
            self.runtime.client_tick,
            false,
        )
        .unwrap_or_default()
    }

    pub fn get_lobby_state(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.lobby_state).unwrap_or(JsValue::NULL)
    }

    pub fn create_action_payload(&self, sequence: u32, action: &str, entity_uid: u64) -> Vec<u8> {
        let target = honknet_core::Entity::new((entity_uid >> 32) as u32, entity_uid as u32);
        let action = match action {
            "interact" => GameAction::Interact { target },
            "attack" => GameAction::Attack { target },
            "pickup" => GameAction::Pickup { target },
            "bandage" => GameAction::Bandage { target },
            "bruise" => GameAction::Treat {
                target,
                treatment: honknet_net_core::MedicalTreatment::BruisePack,
            },
            "burn" => GameAction::Treat {
                target,
                treatment: honknet_net_core::MedicalTreatment::BurnGel,
            },
            "cpr" => GameAction::Cpr { target },
            "surgeryChest" => GameAction::Surgery {
                target,
                zone: honknet_net_core::BodyZoneId::Chest,
            },
            "grab" => GameAction::Grab { target },
            "releaseGrab" => GameAction::ReleaseGrab,
            "pull" => GameAction::Pull { target },
            "stopPulling" => GameAction::StopPulling,
            "carry" => GameAction::Carry { target },
            "dropCarried" => GameAction::DropCarried,
            "buckle" => GameAction::Buckle { fixture: target },
            "unbuckle" => GameAction::Unbuckle,
            "equipJumpsuit" => GameAction::Equip {
                slot: honknet_net_core::EquipmentSlotId::Jumpsuit,
            },
            "unequipJumpsuit" => GameAction::Unequip {
                slot: honknet_net_core::EquipmentSlotId::Jumpsuit,
            },
            "store" => GameAction::Store { container: target },
            "drop" => GameAction::Drop,
            _ => return Vec::new(),
        };
        encode_message_envelope(
            &GameActionRequestPayload { sequence, action },
            self.runtime.client_tick,
            false,
        )
        .unwrap_or_default()
    }

    pub fn drain_action_results(&mut self) -> JsValue {
        let results = std::mem::take(&mut self.action_results);
        serde_wasm_bindgen::to_value(&results).unwrap_or(JsValue::NULL)
    }

    pub fn get_client_state(&self) -> u32 {
        self.runtime.state as u32
    }

    pub fn get_diagnostics(&self) -> String {
        format!(
            "State: {:?}, Entities: {}, Tick: {}, LastAck: {}",
            self.runtime.state,
            self.runtime.world.entities().count(),
            self.runtime.client_tick,
            self.runtime.last_acked_baseline
        )
    }
}
