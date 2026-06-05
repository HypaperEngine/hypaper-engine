//! wgpu device and queue initialisation.

use crate::error::RendererError;

/// An initialised wgpu rendering context (headless — no surface attached).
///
/// Owns the adapter, logical device, and command queue. A Wayland surface will
/// be attached once `hypaper-wayland` integration is complete.
pub struct RenderContext {
    /// The physical GPU adapter selected by wgpu.
    pub adapter: wgpu::Adapter,
    /// The logical wgpu device used for all resource creation.
    pub device: wgpu::Device,
    /// The command queue used to submit GPU work.
    pub queue: wgpu::Queue,
}

/// Creates a headless [`RenderContext`], preferring Vulkan with a GLES fallback.
///
/// No Wayland surface is required at this stage; the context can be used for
/// off-screen rendering and texture uploads until a surface is available.
///
/// # Errors
///
/// Returns [`RendererError::NoAdapter`] if no compatible GPU is found, or
/// [`RendererError::DeviceCreation`] if the logical device request fails.
pub async fn create_context() -> Result<RenderContext, RendererError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
        ..Default::default()
    });

    tracing::info!(
        backends = ?wgpu::Backends::VULKAN | wgpu::Backends::GL,
        "requesting wgpu adapter",
    );

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or(RendererError::NoAdapter)?;

    let info = adapter.get_info();
    tracing::info!(
        name = %info.name,
        backend = ?info.backend,
        "selected GPU adapter",
    );

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("hypaper-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        )
        .await
        .map_err(|e| RendererError::DeviceCreation(e.to_string()))?;

    Ok(RenderContext {
        adapter,
        device,
        queue,
    })
}
