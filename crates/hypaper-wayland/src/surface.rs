//! Layer-shell surface creation and lifecycle management.

use crate::error::WaylandError;

/// Configuration used when creating a [`WaylandSurface`].
#[derive(Debug, Clone)]
pub struct SurfaceConfig {
    /// Connector name of the target monitor (e.g. `"DP-1"`), or `None` for
    /// the compositor's default output.
    pub monitor_name: Option<String>,
    /// Desired surface width in pixels.
    pub width: u32,
    /// Desired surface height in pixels.
    pub height: u32,
}

/// A `zwlr_layer_surface_v1` surface anchored behind all windows on a monitor.
///
/// Created via [`create_surface`] and destroyed when dropped.
pub struct WaylandSurface {
    // TODO: hold the wlr_layer_surface_v1 proxy and associated wl_surface
    // once the full Wayland backend is implemented.
    _priv: (),
}

/// Creates a layer-shell surface for a monitor according to `config`.
///
/// # Errors
///
/// Returns [`WaylandError::Connection`] until the Wayland backend is fully
/// implemented.
pub fn create_surface(config: SurfaceConfig) -> Result<WaylandSurface, WaylandError> {
    tracing::info!(
        monitor = ?config.monitor_name,
        width = config.width,
        height = config.height,
        "creating layer-shell surface (stub)",
    );
    Err(WaylandError::Connection)
}
