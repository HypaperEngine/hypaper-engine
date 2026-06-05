//! Hyprland Unix socket discovery and connection helpers.

use std::path::PathBuf;

use crate::error::HyprlandError;

/// Returns the path to Hyprland's event socket (`.socket2.sock`).
///
/// Reads `$HYPRLAND_INSTANCE_SIGNATURE` from the environment and constructs
/// `/tmp/hypr/<signature>/.socket2.sock`.
///
/// # Errors
///
/// Returns [`HyprlandError::SocketNotFound`] if the environment variable is
/// not set.
pub fn get_socket_path() -> Result<PathBuf, HyprlandError> {
    let sig =
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| HyprlandError::SocketNotFound)?;
    Ok(PathBuf::from(format!("/tmp/hypr/{sig}/.socket2.sock")))
}

/// Connects to Hyprland's event socket and returns an async Unix stream.
///
/// # Errors
///
/// Returns [`HyprlandError::SocketNotFound`] if the socket file does not exist,
/// or [`HyprlandError::Io`] for other connection failures.
pub async fn connect_event_socket() -> Result<tokio::net::UnixStream, HyprlandError> {
    let path = get_socket_path()?;
    tokio::net::UnixStream::connect(&path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HyprlandError::SocketNotFound
        } else {
            HyprlandError::Io(e)
        }
    })
}
