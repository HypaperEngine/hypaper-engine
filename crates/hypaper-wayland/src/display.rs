//! Wayland display connection management.

use crate::error::WaylandError;

/// An active connection to the Wayland compositor display.
///
/// Owns the underlying `wayland-client` connection and the event queue.
/// All surface creation must go through a `WaylandDisplay`.
pub struct WaylandDisplay {
    // TODO: hold wayland_client::Connection and EventQueue once the full
    // Wayland backend is implemented.
    _priv: (),
}

/// Connects to the Wayland compositor via the `WAYLAND_DISPLAY` socket.
///
/// # Errors
///
/// Returns [`WaylandError::Connection`] if no compositor is reachable.
pub fn connect() -> Result<WaylandDisplay, WaylandError> {
    tracing::info!("connecting to Wayland display (stub)");
    Err(WaylandError::Connection)
}
