use bytemuck::{
    Pod,
    Zeroable
};
use honknet_math::{
    Aabb,
    Vec2
};
use std::collections::{
    BTreeMap,
    HashMap
};
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SpriteInstance {
    pub position:[f32;
    2],
    pub size:[f32;
    2],
    pub color:[f32;
    4],
    pub uv:[f32;
    4],
    pub rotation: f32,
    pub z: f32,
    pub _pad:[f32;
    2]
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub center: Vec2,
    pub size: Vec2,
    pub layer_mask: u64
}

#[derive(Debug, Clone)]
pub struct Light {
    pub position: Vec2,
    pub radius: f32,
    pub color:[f32;
    3],
    pub intensity: f32
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub life: f32,
    pub color:[f32;
    4],
    pub size: f32
}

#[derive(Debug, Clone)]
pub enum RenderPassKind {
    Visibility,
    Tiles,
    Opaque,
    Lighting,
    Transparent,
    Particles,
    Overlays,
    WorldUi,
    ScreenUi,
    PostProcess,
    Debug
}

#[derive(Default)]
pub struct RenderGraph {
    passes: Vec<(String, RenderPassKind, Vec<String>)>
}

impl RenderGraph {
    pub fn add(&mut self, name: &str, kind: RenderPassKind, after: Vec<String>) {
        self.passes.push((name.into(), kind, after))
    }
    pub fn ordered(&self) -> Vec<&str> {
        let mut done = std::collections::HashSet::new();
        let mut out = vec![];
        while out.len()<self.passes.len() {
            let mut progress = false;
            for (n, _, deps) in &self.passes {
                if !done.contains(n) && deps.iter().all(|d| done.contains(d)) {
                    done.insert(n.clone());
                    out.push(n.as_str());
                    progress = true
                }
            }
            if !progress {
                break
            }
        }
        out
    }
}

pub struct SpriteBatch {
    layers: BTreeMap<i32, Vec<SpriteInstance>>
}

impl Default for SpriteBatch {
    fn default() -> Self {
        Self {
            layers: BTreeMap::new()
        }
    }
}

impl SpriteBatch {
    pub fn push(&mut self, layer: i32, s: SpriteInstance) {
        self.layers.entry(layer).or_default().push(s)
    }
    pub fn visible(&self, c: &Camera) -> Vec<SpriteInstance> {
        let view = Aabb::from_center_half(c.center, c.size * 0.5);
        self.layers.values().flatten().filter(|s| Aabb::from_center_half(Vec2::new(s.position[0], s.position[1]),
        Vec2::new(s.size[0], s.size[1]) * 0.5).intersects(view)).copied().collect()
    }
    pub fn clear(&mut self) {
        self.layers.clear()
    }
}

pub struct Atlas {
    pub size:[u32;
    2], next:[u32;
    2], row: u32, pub regions: HashMap<String,[f32;
    4]>
}

impl Atlas {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            size:[w, h], next:[0, 0], row: 0, regions: HashMap::new()
        }
    }
    pub fn insert(&mut self, key: &str, w: u32, h: u32) -> Option<[f32;
    4]> {
        if self.next[0] + w> self.size[0] {
            self.next[0] = 0;
            self.next[1] += self.row;
            self.row = 0
        }
        if self.next[1] + h > self.size[1] {
            return None
        }
        let r =[self.next[0] as f32 / self.size[0] as f32, self.next[1] as f32 / self.size[1] as f32, w as f32 / self.size[0] as f32,
        h as f32 / self.size[1] as f32];
        self.next[0] += w;
        self.row = self.row.max(h);
        self.regions.insert(key.into(), r);
        Some(r)
    }
}

pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    _target: wgpu::Texture,
    view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    instance: wgpu::Buffer,
    capacity: usize,
    pub size:[u32;
    2]
}

impl WgpuRenderer {
    pub async fn new(width: u32, height: u32) -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance, compatible_surface: None, force_fallback_adapter: false
        }).await.ok_or("failed to find an appropriate adapter".to_string())?;
        let(device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Honknet device"), required_features: wgpu::Features::empty(), required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance
        }, None).await.map_err(|e| e.to_string())?;
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Honknet target"), size: wgpu::Extent3d {
                width, height, depth_or_array_layers: 1
            }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[]
        });
        let view = target.create_view(&Default::default());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"), source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into())
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite layout"), bind_group_layouts: &[], push_constant_ranges: &[]
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite pipeline"), layout: Some(&layout), vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SpriteInstance>() as u64, step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4,
                    4 => Float32, 5 => Float32]
                }]
            }, fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState {
                    format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL
                })]
            }), primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(),
            multiview: None, cache: None
        });
        let capacity = 1024;
        let instance = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite instances"), size:(capacity * std::mem::size_of::<SpriteInstance>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false
        });
        Ok(Self {
            device, queue, _target: target, view, pipeline, instance, capacity, size:[width, height]
        })
    }
    pub fn render(&mut self, sprites: &[SpriteInstance], clear:[f64;
    4]) {
        if sprites.len() > self.capacity {
            self.capacity = sprites.len().next_power_of_two();
            self.instance = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite instances"), size:(self.capacity * std::mem::size_of::<SpriteInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false
            });
        }
        self.queue.write_buffer(&self.instance, 0, bytemuck::cast_slice(sprites));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprites"), color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view, resolve_target: None, ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0], g: clear[1], b: clear[2], a: clear[3]
                        }), store: wgpu::StoreOp::Store
                    }
                })], depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None
            });
            p.set_pipeline(&self.pipeline);
            p.set_vertex_buffer(0, self.instance.slice(..));
            p.draw(0..6, 0..sprites.len() as u32);
        }
        self.queue.submit(Some(encoder.finish()));
    }
}

pub fn update_particles(p: &mut Vec<Particle>, dt: f32) {
    for x in p.iter_mut() {
        x.position += x.velocity * dt;
        x.life -= dt
    }
    p.retain(|x| x.life > 0.)
}
