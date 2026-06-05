//! wgpu device and queue initialisation.

use std::ptr::NonNull;

use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};

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

/// Creates a [`RenderContext`] and attaches a wgpu surface to a live Wayland window.
///
/// The adapter is requested with `compatible_surface` so wgpu selects a backend
/// that can present to the given Wayland surface.
///
/// # Safety
///
/// The caller must guarantee that:
/// - `raw.display_ptr` is a valid, non-null `*mut wl_display` that outlives the
///   returned [`wgpu::Surface`].
/// - `raw.surface_ptr` is a valid, non-null `*mut wl_surface` that outlives the
///   returned [`wgpu::Surface`].
///
/// In practice, keeping the originating [`hypaper_wayland::surface::WaylandSurface`]
/// alive for at least as long as the returned surface satisfies these requirements.
///
/// # Errors
///
/// - [`RendererError::Surface`] if the display or surface pointer is null, or if
///   wgpu rejects the raw Wayland handles.
/// - [`RendererError::NoAdapter`] if no GPU adapter compatible with the surface is found.
/// - [`RendererError::DeviceCreation`] if the logical device request fails.
pub async unsafe fn create_context_for_surface(
    raw: &hypaper_wayland::raw_handle::RawWindowHandle,
    width: u32,
    height: u32,
) -> Result<(RenderContext, wgpu::Surface<'static>), RendererError> {
    let _ = (width, height); // dimensions reserved for future surface configuration

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
        ..Default::default()
    });

    // SAFETY: The caller guarantees that both pointers are valid and non-null
    // for the lifetime of the returned Surface.
    let display_nn = NonNull::new(raw.display_ptr)
        .ok_or_else(|| RendererError::Surface("null wl_display pointer".into()))?;
    let surface_nn = NonNull::new(raw.surface_ptr)
        .ok_or_else(|| RendererError::Surface("null wl_surface pointer".into()))?;

    let raw_display = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display_nn));
    let raw_window = RawWindowHandle::Wayland(WaylandWindowHandle::new(surface_nn));

    // SAFETY: raw_display and raw_window are backed by live wl_display and
    // wl_surface objects, as guaranteed by the caller via the function's safety
    // contract.  The 'static lifetime is intentional: safety is asserted by the
    // caller, not enforced by the type system.
    let surface: wgpu::Surface<'static> = unsafe {
        instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: raw_display,
                raw_window_handle: raw_window,
            })
            .map_err(|e| RendererError::Surface(e.to_string()))?
    };

    tracing::info!(
        backends = ?wgpu::Backends::VULKAN | wgpu::Backends::GL,
        "requesting wgpu adapter for Wayland surface",
    );

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or(RendererError::NoAdapter)?;

    let info = adapter.get_info();
    tracing::info!(
        name = %info.name,
        backend = ?info.backend,
        "selected GPU adapter for surface",
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

    Ok((
        RenderContext {
            adapter,
            device,
            queue,
        },
        surface,
    ))
}
