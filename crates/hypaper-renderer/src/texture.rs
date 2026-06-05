//! GPU texture creation and image upload utilities.

use wgpu::util::DeviceExt;

use crate::error::RendererError;

/// A 2-D texture resident on the GPU with an associated view and sampler.
pub struct GpuTexture {
    /// The underlying wgpu texture object.
    pub texture: wgpu::Texture,
    /// Default full-mip view used for sampling.
    pub view: wgpu::TextureView,
    /// Linear sampler with clamp-to-edge addressing.
    pub sampler: wgpu::Sampler,
    /// Texture width in texels.
    pub width: u32,
    /// Texture height in texels.
    pub height: u32,
}

/// Decodes an image from `bytes` and uploads it to the GPU as an sRGB texture.
///
/// Supports any format recognised by the [`image`] crate (PNG, JPEG, WebP, …).
/// The image is converted to RGBA8 before upload.
///
/// # Errors
///
/// Returns [`RendererError::Texture`] if the bytes cannot be decoded.
pub fn load_texture_from_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bytes: &[u8],
    label: &str,
) -> Result<GpuTexture, RendererError> {
    let img = image::load_from_memory(bytes).map_err(|e| RendererError::Texture(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &rgba,
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(&format!("{label}-sampler")),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    Ok(GpuTexture {
        texture,
        view,
        sampler,
        width,
        height,
    })
}
