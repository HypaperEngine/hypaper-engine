//! Fullscreen render pipeline for displaying a textured quad.

/// WGSL shader source for the fullscreen textured-quad pipeline.
///
/// The vertex shader generates a screen-filling quad from 6 vertices using
/// only the built-in `vertex_index`; no vertex buffer is required.
const FULLSCREEN_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    return VertexOutput(vec4<f32>(pos[vi], 0.0, 1.0), uv[vi]);
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}
"#;

/// A compiled wgpu render pipeline for drawing a fullscreen textured quad.
///
/// Wraps the [`wgpu::RenderPipeline`] together with the bind group layout so
/// that callers can create texture bind groups compatible with this pipeline.
pub struct RenderPipeline {
    /// The underlying compiled wgpu pipeline.
    pub inner: wgpu::RenderPipeline,
    /// Bind group layout for slot 0: `texture_2d` at binding 0, `sampler` at binding 1.
    pub bind_group_layout: wgpu::BindGroupLayout,
}

/// Compiles a fullscreen textured-quad pipeline targeting `format`.
///
/// The pipeline requires no vertex buffer; positions and UVs are generated
/// from `vertex_index` inside the vertex shader. Bind group 0 must contain a
/// `texture_2d<f32>` at binding 0 and a `sampler` at binding 1.
pub fn create_fullscreen_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fullscreen-shader"),
        source: wgpu::ShaderSource::Wgsl(FULLSCREEN_SHADER.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fullscreen-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fullscreen-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let inner = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fullscreen-pipeline"),
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
                blend: Some(wgpu::BlendState::REPLACE),
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

    RenderPipeline {
        inner,
        bind_group_layout,
    }
}
