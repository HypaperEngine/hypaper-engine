//! Error types for Hyprland IPC operations.

/// All errors that can occur when communicating with the Hyprland compositor.
#[derive(Debug, thiserror::Error)]
pub enum HyprlandError {
    /// No Hyprland socket was found; the compositor is likely not running.
    #[error("Hyprland socket not found — is Hyprland running?")]
    SocketNotFound,

    /// An I/O error on the Unix socket.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// An event line from the socket could not be parsed.
    #[error("Failed to parse Hyprland event: {0}")]
    ParseError(String),
}
