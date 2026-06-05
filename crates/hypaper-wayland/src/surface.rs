//! Layer-shell surface creation and lifecycle management.

use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::Layer,
    zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

use crate::display::{MonitorInfo, WaylandDisplay};
use crate::error::WaylandError;

/// Configuration used when creating a [`WaylandSurface`].
#[derive(Debug, Clone)]
pub struct SurfaceConfig {
    /// Target monitor, or `None` to let the compositor choose the default output.
    pub monitor: Option<MonitorInfo>,
    /// Desired surface width in pixels (used as fallback if the compositor
    /// sends zero in the configure event).
    pub width: u32,
    /// Desired surface height in pixels (used as fallback if the compositor
    /// sends zero in the configure event).
    pub height: u32,
}

/// A `zwlr_layer_surface_v1` surface anchored to the background layer of a monitor.
///
/// The surface is created by [`create_surface`], spans the full monitor area, and
/// sits behind all normal windows (`Layer::Background`, exclusive zone `-1`).
pub struct WaylandSurface {
    /// The underlying `wl_surface` Wayland object.
    pub(crate) wl_surface: WlSurface,
    /// The `zwlr_layer_surface_v1` role object attached to `wl_surface`.
    ///
    /// Held purely for RAII: the protocol requires this proxy to outlive the
    /// `wl_surface`.  It is never read after creation.
    #[allow(dead_code)]
    pub(crate) layer_surface: ZwlrLayerSurfaceV1,
    /// Surface width in pixels as assigned by the compositor's configure event.
    pub width: u32,
    /// Surface height in pixels as assigned by the compositor's configure event.
    pub height: u32,
    /// Raw `*mut wl_display` pointer for use by [`crate::raw_handle::RawWindowHandle`].
    pub(crate) display_ptr: *mut std::ffi::c_void,
}

// SAFETY: WaylandSurface must only be used on the thread that owns the Wayland
// connection. The raw display_ptr is stable for the lifetime of the connection.
// Callers that move a WaylandSurface across threads are responsible for
// upholding the single-thread access invariant required by libwayland.
unsafe impl Send for WaylandSurface {}

/// Creates a full-screen background layer surface on the compositor.
///
/// The surface is configured with:
/// - layer: `Background` (behind all windows)
/// - anchor: all four edges (fills the entire output)
/// - exclusive zone: `-1` (does not reserve screen space)
/// - keyboard interactivity: `None`
///
/// When `config.monitor` is `Some`, the surface is attached to the matching
/// `wl_output`; if the name is not found, the first available output is used.
/// When `config.monitor` is `None`, the compositor chooses the output.
///
/// The function performs a blocking Wayland roundtrip to receive the compositor's
/// `configure` event before returning.
///
/// # Errors
///
/// - [`WaylandError::SurfaceCreation`] if the compositor does not send a configure event.
/// - [`WaylandError::Other`] if the event-queue roundtrip fails.
pub fn create_surface(
    display: &mut WaylandDisplay,
    config: SurfaceConfig,
) -> Result<WaylandSurface, WaylandError> {
    let qh = display.qh.clone();

    let wl_surface = display.compositor.create_surface(&qh, ());

    // Select the wl_output that matches config.monitor, falling back to the
    // first output or None (compositor's choice) when not found.
    let output = match &config.monitor {
        Some(monitor) => {
            let index = display
                .state
                .output_data
                .iter()
                .position(|d| d.name == monitor.name);
            index
                .and_then(|i| display.outputs.get(i))
                .or_else(|| display.outputs.first())
        }
        None => display.outputs.first(),
    };

    let layer_surface = display.layer_shell.get_layer_surface(
        &wl_surface,
        output,
        Layer::Background,
        "hypaper".to_owned(),
        &qh,
        (),
    );

    // Size 0×0 lets the compositor assign the output dimensions.
    layer_surface.set_size(0, 0);
    // Anchor to all edges so the surface fills the entire output.
    layer_surface.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
    // Exclusive zone -1: extend under other exclusive-zone surfaces.
    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

    // Commit with no buffer to request the configure event from the compositor.
    wl_surface.commit();

    display.roundtrip()?;

    let (serial, configured_w, configured_h) = display
        .state
        .configure
        .take()
        .ok_or(WaylandError::SurfaceCreation)?;

    layer_surface.ack_configure(serial);
    wl_surface.commit();

    let width = if configured_w > 0 {
        configured_w
    } else {
        config.width
    };
    let height = if configured_h > 0 {
        configured_h
    } else {
        config.height
    };

    // Retrieve the raw wl_display pointer for raw-window-handle support.
    let display_ptr = display.connection.backend().display_ptr() as *mut std::ffi::c_void;

    let monitor_name = config
        .monitor
        .as_ref()
        .map(|m| m.name.as_str())
        .unwrap_or("(default)");
    tracing::info!(
        monitor = %monitor_name,
        width,
        height,
        "created layer-shell surface",
    );

    Ok(WaylandSurface {
        wl_surface,
        layer_surface,
        width,
        height,
        display_ptr,
    })
}
