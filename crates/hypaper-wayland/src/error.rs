//! Error types for Wayland display and surface operations.

/// All errors that can occur when interacting with the Wayland compositor.
#[derive(Debug, thiserror::Error)]
pub enum WaylandError {
    /// The process could not connect to a running Wayland compositor.
    #[error("Failed to connect to Wayland display")]
    Connection,

    /// The compositor does not advertise the `zwlr_layer_shell_v1` global.
    #[error("wlr-layer-shell protocol not supported by compositor")]
    LayerShellNotSupported,

    /// The compositor rejected the layer surface creation request.
    #[error("Failed to create layer surface")]
    SurfaceCreation,

    /// An unclassified Wayland error with a human-readable description.
    #[error("Wayland error: {0}")]
    Other(String),
}
