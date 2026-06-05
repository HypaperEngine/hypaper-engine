//! Per-layer WGSL shader renderer with automatic time/resolution/mouse uniforms.

use crate::{
    error::RendererError,
    uniforms::{create_uniform_buffer, AutoUniforms},
};

/// WGSL preamble injected before the user's shader source.
///
/// Declares `AutoUniforms` and binds the buffer to `@group(0) @binding(0)`.
/// User shaders access uniforms as:
/// - `uniforms.time`         — elapsed seconds
/// - `uniforms.resolution_x` / `uniforms.resolution_y` — viewport size in pixels
/// - `uniforms.mouse_x`      / `uniforms.mouse_y`       — cursor position in pixels
///
/// The struct layout (8 × f32 = 32 bytes) matches [`AutoUniforms`] exactly.
const PREAMBLE: &str = r#"
// Injected by hypaper-renderer — do not redeclare.
struct AutoUniforms {
    time: f32,
    resolution_x: f32,
    resolution_y: f32,
    mouse_x: f32,
    mouse_y: f32,
    _p0: f32,
    _p1: f32,
    _p2: f32,
}
@group(0) @binding(0) var<uniform> uniforms: AutoUniforms;
"#;

/// Renders a single fullscreen WGSL shader layer with automatic per-frame uniforms.
///
/// The renderer compiles user-provided WGSL source prepended with a generated
/// preamble that declares the `AutoUniforms` struct and binding.  Users must
/// provide `vs_main` (vertex) and `fs_main` (fragment) entry points and must
/// **not** redeclare the injected symbols.
///
/// Alpha blending is enabled so shader layers can be composited over an
/// underlying image layer.
pub struct ShaderLayerRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl ShaderLayerRenderer {
    /// Compiles a WGSL shader layer and creates the GPU pipeline.
    ///
    /// The `AutoUniforms` binding declaration is prepended automatically.
    /// The user's WGSL must export `vs_main` and `fs_main`.
    ///
    /// WGSL validation errors are reported asynchronously by wgpu and do not
    /// cause this function to return `Err`; they will appear in the wgpu debug
    /// log when the pipeline is first used.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError`] if a synchronous validation path is added in
    /// the future; currently always succeeds.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        wgsl_source: &str,
    ) -> Result<Self, RendererError> {
        let full_source = format!("{PREAMBLE}\n{wgsl_source}");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader-layer"),
            source: wgpu::ShaderSource::Wgsl(full_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shader-layer-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_buffer = create_uniform_buffer(device);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shader-layer-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shader-layer-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shader-layer-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
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

        Ok(Self {
            pipeline,
            uniform_buffer,
            bind_group,
        })
    }

    /// Writes updated `time`, `resolution`, and `mouse` values into the uniform
    /// buffer.  Call once per frame before [`render`](Self::render).
    pub fn update_uniforms(
        &mut self,
        queue: &wgpu::Queue,
        time: f32,
        resolution: [f32; 2],
        mouse: [f32; 2],
    ) {
        let uniforms = AutoUniforms {
            time,
            resolution,
            mouse,
            _pad: [0.0; 3],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Records a fullscreen render pass for this shader layer into `encoder`,
    /// drawing over `output_view`.
    ///
    /// Uses `LoadOp::Load` to composite over existing content; the caller is
    /// responsible for ensuring `output_view` has been cleared before the first
    /// layer renders.
    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shader-layer-pass"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}
