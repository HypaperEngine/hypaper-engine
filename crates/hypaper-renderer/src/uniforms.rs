//! Automatic per-frame uniform buffer shared with all WGSL shader layers.

/// Per-frame uniforms injected automatically into every shader layer.
///
/// Layout (32 bytes, `#[repr(C)]`):
///
/// | offset | field        | type     |
/// |--------|--------------|----------|
/// | 0      | `time`       | `f32`    |
/// | 4      | `resolution` | `[f32;2]`|
/// | 12     | `mouse`      | `[f32;2]`|
/// | 20     | `_pad`       | `[f32;3]`|
///
/// The three padding words grow the struct to 32 bytes, which is the smallest
/// multiple of 16 that fits the payload and satisfies the WGSL extended-alignment
/// requirement for `uniform` address-space structs.
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct AutoUniforms {
    /// Elapsed seconds since the wallpaper started rendering.
    pub time: f32,
    /// Viewport size in pixels: `[width, height]`.
    pub resolution: [f32; 2],
    /// Cursor position in pixels: `[x, y]`.
    pub mouse: [f32; 2],
    /// Padding to 32 bytes for wgpu uniform-buffer alignment.
    pub _pad: [f32; 3],
}

/// Creates a GPU buffer sized for [`AutoUniforms`], ready for per-frame writes.
///
/// Usage flags: `UNIFORM | COPY_DST` — the buffer can be bound as a uniform
/// and updated every frame via [`wgpu::Queue::write_buffer`].
pub fn create_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("auto-uniforms"),
        size: std::mem::size_of::<AutoUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
