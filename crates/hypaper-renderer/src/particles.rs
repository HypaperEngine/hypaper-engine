//! CPU-driven particle system with GPU instanced rendering.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::{
    error::RendererError,
    texture::{load_texture_from_bytes, GpuTexture},
};

use hypaper_types::layer::{EmitterMode, ParticleLayer};

fn next_f32(seed: &mut u64) -> f32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*seed >> 33) as f32) / (u32::MAX as f32)
}

fn rand_range(seed: &mut u64, min: f32, max: f32) -> f32 {
    min + next_f32(seed) * (max - min)
}

/// A single particle in the simulation.
pub struct Particle {
    position: [f32; 2],
    velocity: [f32; 2],
    size: f32,
    opacity: f32,
    lifetime: f32,
    age: f32,
}

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
struct ParticleInstance {
    position: [f32; 2],
    size: f32,
    opacity: f32,
}

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
struct ParticleUniforms {
    resolution: [f32; 2],
    _pad: [f32; 2],
}

const PARTICLE_SHADER: &str = r#"
struct Uniforms {
    resolution: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var t_particle: texture_2d<f32>;
@group(0) @binding(2) var s_particle: sampler;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @location(0) inst_pos: vec2<f32>,
    @location(1) inst_size: f32,
    @location(2) inst_opacity: f32,
) -> VertexOutput {
    var offsets = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>(-0.5,  0.5),
        vec2<f32>(-0.5,  0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>( 0.5,  0.5),
    );
    var tex_uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );

    let offset = offsets[vi];

    // Screen pixel → NDC; pixel Y=0 is top → NDC Y=+1
    let ndc_x = (inst_pos.x / uniforms.resolution.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (inst_pos.y / uniforms.resolution.y) * 2.0;

    let sx = inst_size / uniforms.resolution.x * 2.0;
    let sy = inst_size / uniforms.resolution.y * 2.0;

    var out: VertexOutput;
    out.clip_pos = vec4<f32>(ndc_x + offset.x * sx, ndc_y + offset.y * sy, 0.0, 1.0);
    out.uv = tex_uvs[vi];
    out.opacity = inst_opacity;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_particle, s_particle, in.uv);
    color.a *= in.opacity;
    return color;
}
"#;

/// A CPU-driven particle system that renders particles as instanced textured quads.
pub struct ParticleSystem {
    /// Live particles in the simulation.
    pub particles: Vec<Particle>,
    /// Texture applied to each particle quad.
    pub texture: Option<GpuTexture>,
    /// Configuration for this particle system.
    pub config: ParticleLayer,
    /// LCG state for random value generation.
    pub rng_seed: u64,
    emit_accumulator: f32,
    screen_w: u32,
    screen_h: u32,
    pipeline: Option<wgpu::RenderPipeline>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    uniform_buffer: Option<wgpu::Buffer>,
}

impl ParticleSystem {
    /// Creates a new particle system from `config`. GPU resources are not
    /// allocated until [`build_pipeline`](Self::build_pipeline) is called.
    pub fn new(config: ParticleLayer) -> Self {
        Self {
            particles: Vec::new(),
            texture: None,
            config,
            rng_seed: 0xdeadbeef_cafebabe_u64,
            emit_accumulator: 0.0,
            screen_w: 0,
            screen_h: 0,
            pipeline: None,
            bind_group_layout: None,
            uniform_buffer: None,
        }
    }

    /// Compiles the render pipeline. Must be called once after construction and
    /// before the first [`render`](Self::render) call.
    pub fn build_pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle-shader"),
            source: wgpu::ShaderSource::Wgsl(PARTICLE_SHADER.into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle-pipeline-layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ParticleInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle-uniforms"),
            contents: bytemuck::bytes_of(&ParticleUniforms {
                resolution: [0.0, 0.0],
                _pad: [0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        self.pipeline = Some(pipeline);
        self.bind_group_layout = Some(bgl);
        self.uniform_buffer = Some(uniform_buffer);
    }

    /// Decodes `bytes` as an image and uploads it as the particle texture.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::Texture`] if the image cannot be decoded.
    pub fn set_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> Result<(), RendererError> {
        let tex = load_texture_from_bytes(device, queue, bytes, "particle-tex")?;
        self.texture = Some(tex);
        Ok(())
    }

    /// Advances the simulation by `delta` seconds: ages and culls dead particles,
    /// then emits new ones according to `emit_rate`.
    pub fn update(&mut self, delta: f32, screen_w: u32, screen_h: u32) {
        self.screen_w = screen_w;
        self.screen_h = screen_h;

        let gravity = self.config.gravity;
        let mut i = 0;
        while i < self.particles.len() {
            let p = &mut self.particles[i];
            p.age += delta;
            if p.age >= p.lifetime {
                self.particles.swap_remove(i);
            } else {
                p.position[0] += p.velocity[0] * delta;
                p.position[1] += p.velocity[1] * delta;
                p.velocity[1] += gravity * delta;
                i += 1;
            }
        }

        self.emit_accumulator += self.config.emit_rate * delta;
        let to_emit = self.emit_accumulator as u32;
        self.emit_accumulator -= to_emit as f32;

        let max_count = self.config.count as usize;
        for _ in 0..to_emit {
            if self.particles.len() >= max_count {
                break;
            }
            let p = self.spawn_particle(screen_w, screen_h);
            self.particles.push(p);
        }
    }

    fn spawn_particle(&mut self, screen_w: u32, screen_h: u32) -> Particle {
        let sw = screen_w as f32;
        let sh = screen_h as f32;

        // Extract config values before taking the mutable seed borrow.
        let emitter = self.config.emitter.clone();
        let max_lifetime = self.config.lifetime;
        let abs_vx = self.config.velocity_x.abs();
        let abs_vy = self.config.velocity_y.abs();
        let max_size = self.config.size;
        let max_opacity = self.config.opacity;

        let seed = &mut self.rng_seed;

        let position = match emitter {
            EmitterMode::Top => [rand_range(seed, 0.0, sw), 0.0],
            EmitterMode::Bottom => [rand_range(seed, 0.0, sw), sh],
            EmitterMode::Edges => {
                let edge = (next_f32(seed) * 4.0) as u32 % 4;
                match edge {
                    0 => [rand_range(seed, 0.0, sw), 0.0],
                    1 => [rand_range(seed, 0.0, sw), sh],
                    2 => [0.0, rand_range(seed, 0.0, sh)],
                    _ => [sw, rand_range(seed, 0.0, sh)],
                }
            }
            EmitterMode::Point(x, y) => [x * sw, y * sh],
            EmitterMode::Fullscreen => [rand_range(seed, 0.0, sw), rand_range(seed, 0.0, sh)],
        };

        let lifetime = rand_range(seed, max_lifetime * 0.5, max_lifetime);
        let vx = rand_range(seed, -abs_vx, abs_vx);
        let vy = rand_range(seed, -abs_vy, abs_vy);
        let size = rand_range(seed, max_size * 0.5, max_size);
        let opacity = rand_range(seed, max_opacity * 0.5, max_opacity);

        Particle {
            position,
            velocity: [vx, vy],
            size,
            opacity,
            lifetime,
            age: 0.0,
        }
    }

    /// Records an instanced draw pass for all live particles into `encoder`.
    ///
    /// No-ops if the pipeline has not been built, no texture is set, dimensions
    /// are zero, or there are no live particles.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        let pipeline = match &self.pipeline {
            Some(p) => p,
            None => return,
        };
        let tex = match &self.texture {
            Some(t) => t,
            None => return,
        };
        let bgl = match &self.bind_group_layout {
            Some(b) => b,
            None => return,
        };
        let uniform_buffer = match &self.uniform_buffer {
            Some(u) => u,
            None => return,
        };

        if self.particles.is_empty() || self.screen_w == 0 || self.screen_h == 0 {
            return;
        }

        queue.write_buffer(
            uniform_buffer,
            0,
            bytemuck::bytes_of(&ParticleUniforms {
                resolution: [self.screen_w as f32, self.screen_h as f32],
                _pad: [0.0, 0.0],
            }),
        );

        let instances: Vec<ParticleInstance> = self
            .particles
            .iter()
            .map(|p| ParticleInstance {
                position: p.position,
                size: p.size,
                opacity: p.opacity,
            })
            .collect();

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle-instances"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle-bg"),
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&tex.sampler),
                },
            ],
        });

        let instance_count = instances.len() as u32;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("particle-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, instance_buffer.slice(..));
            pass.draw(0..6, 0..instance_count);
        }
    }
}
