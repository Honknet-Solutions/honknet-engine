use honknet_math::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputSource {
    Key(String),
    MouseButton(u8),
    GamepadButton(u16),
    GamepadAxis(u16),
    TouchGesture(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub source: InputSource,
    pub scale: f32,
    pub dead_zone: f32,
    pub chord: Vec<InputSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionMap {
    pub actions: HashMap<String, Vec<Binding>>,
}

#[derive(Debug, Clone, Default)]
pub struct RawInput {
    pub digital: HashSet<InputSource>,
    pub analog: HashMap<InputSource, f32>,
    pub pointer: Vec2,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputFrame {
    pub sequence: u32,
    pub tick: u64,
    pub actions: HashMap<String, f32>,
    pub pointer: Vec2,
}

#[derive(Debug, Clone)]
pub struct InputContext {
    pub name: String,
    pub enabled: bool,
    pub captures_ui: bool,
    pub priority: i32,
}

#[derive(Default)]
pub struct InputSystem {
    pub map: ActionMap,
    contexts: Vec<InputContext>,
    sequence: u32,
}

impl InputSystem {
    pub fn push_context(&mut self, context: InputContext) {
        self.contexts.push(context);
        self.contexts.sort_by_key(|c| -c.priority);
    }
    pub fn set_enabled(&mut self, name: &str, enabled: bool) {
        if let Some(c) = self.contexts.iter_mut().find(|c| c.name == name) {
            c.enabled = enabled;
        }
    }
    pub fn sample(&mut self, tick: u64, raw: &RawInput) -> InputFrame {
        self.sequence = self.sequence.wrapping_add(1);
        let mut actions = HashMap::new();
        for (name, bindings) in &self.map.actions {
            let mut best = 0.0f32;
            for b in bindings {
                if !b.chord.iter().all(|x| raw.digital.contains(x)) {
                    continue;
                }
                let value = if raw.digital.contains(&b.source) {
                    1.0
                } else {
                    *raw.analog.get(&b.source).unwrap_or(&0.0)
                };
                let value = if value.abs() < b.dead_zone {
                    0.0
                } else {
                    value * b.scale
                };
                if value.abs() > best.abs() {
                    best = value;
                }
            }
            actions.insert(name.clone(), best);
        }
        InputFrame {
            sequence: self.sequence,
            tick,
            actions,
            pointer: raw.pointer,
        }
    }
    pub fn pack(frame: &InputFrame) -> Vec<u8> {
        bincode_like(frame)
    }
}

fn bincode_like(frame: &InputFrame) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(frame.sequence.to_le_bytes());
    out.extend(frame.tick.to_le_bytes());
    out.extend(frame.pointer.x.to_le_bytes());
    out.extend(frame.pointer.y.to_le_bytes());
    out.extend((frame.actions.len() as u16).to_le_bytes());
    let mut keys: Vec<_> = frame.actions.iter().collect();
    keys.sort_by_key(|x| x.0);
    for (k, v) in keys {
        out.extend((k.len() as u16).to_le_bytes());
        out.extend(k.as_bytes());
        out.extend(v.to_le_bytes());
    }
    out
}
