use honknet_math::Vec2;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::{collections::HashMap, io::Cursor, sync::Arc};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("backend: {0}")]
    Backend(String),
    #[error("decode: {0}")]
    Decode(String),
}

#[derive(Debug, Clone)]
pub struct SoundParams {
    pub position: Option<Vec2>,
    pub velocity: Vec2,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub bus: String,
    pub priority: i32,
    pub max_distance: f32,
    pub reverb: f32,
}

impl Default for SoundParams {
    fn default() -> Self {
        Self {
            position: None,
            velocity: Vec2::ZERO,
            volume: 1.,
            pitch: 1.,
            looping: false,
            bus: "master".into(),
            priority: 0,
            max_distance: 32.,
            reverb: 0.,
        }
    }
}

pub trait AudioBackend: Send {
    fn play(&mut self, data: Arc<Vec<u8>>, params: SoundParams) -> Result<u64, AudioError>;
    fn stop(&mut self, id: u64);
    fn set_listener(&mut self, position: Vec2, velocity: Vec2);
    fn update(&mut self);
}

struct Voice {
    sink: Sink,
    params: SoundParams,
}

pub struct NativeAudio {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    voices: HashMap<u64, Voice>,
    listener: Vec2,
    listener_velocity: Vec2,
    next: u64,
    buses: HashMap<String, f32>,
    limit: usize,
}

unsafe impl Send for NativeAudio {}

impl NativeAudio {
    pub fn new(limit: usize) -> Result<Self, AudioError> {
        let (s, h) = OutputStream::try_default().map_err(|e| AudioError::Backend(e.to_string()))?;
        Ok(Self {
            _stream: s,
            handle: h,
            voices: HashMap::new(),
            listener: Vec2::ZERO,
            listener_velocity: Vec2::ZERO,
            next: 1,
            buses: HashMap::from([("master".into(), 1.)]),
            limit: limit.max(1),
        })
    }
    pub fn set_bus(&mut self, n: &str, v: f32) {
        self.buses.insert(n.into(), v.max(0.));
    }
}

impl AudioBackend for NativeAudio {
    fn play(&mut self, data: Arc<Vec<u8>>, params: SoundParams) -> Result<u64, AudioError> {
        if self.voices.len() >= self.limit {
            if let Some(id) = self
                .voices
                .iter()
                .min_by_key(|(_, v)| v.params.priority)
                .map(|(i, _)| *i)
            {
                self.stop(id)
            }
        }
        let decoder = Decoder::new(Cursor::new((*data).clone()))
            .map_err(|e| AudioError::Decode(e.to_string()))?;
        let sink = Sink::try_new(&self.handle).map_err(|e| AudioError::Backend(e.to_string()))?;
        if params.looping {
            sink.append(decoder.repeat_infinite().speed(params.pitch.max(0.1)))
        } else {
            sink.append(decoder.speed(params.pitch.max(0.1)))
        }
        let id = self.next;
        self.next += 1;
        self.voices.insert(id, Voice { sink, params });
        self.update();
        Ok(id)
    }
    fn stop(&mut self, id: u64) {
        if let Some(v) = self.voices.remove(&id) {
            v.sink.stop()
        }
    }
    fn set_listener(&mut self, p: Vec2, v: Vec2) {
        self.listener = p;
        self.listener_velocity = v
    }
    fn update(&mut self) {
        self.voices.retain(|_, v| {
            let distance = v
                .params
                .position
                .map_or(0., |p| (p - self.listener).length());
            let attenuation = (1. - distance / v.params.max_distance.max(0.1)).clamp(0., 1.);
            let bus = *self.buses.get(&v.params.bus).unwrap_or(&1.);
            v.sink.set_volume(v.params.volume * attenuation * bus);
            !v.sink.empty() || v.params.looping
        });
    }
}

#[derive(Default)]
pub struct NullAudio {
    pub played: u64,
    pub listener: Vec2,
}

impl AudioBackend for NullAudio {
    fn play(&mut self, _: Arc<Vec<u8>>, _: SoundParams) -> Result<u64, AudioError> {
        self.played += 1;
        Ok(self.played)
    }
    fn stop(&mut self, _: u64) {}
    fn set_listener(&mut self, p: Vec2, _: Vec2) {
        self.listener = p
    }
    fn update(&mut self) {}
}
