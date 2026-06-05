//! IPC message types exchanged between `hypaperctl` and `hypaperd` over the Unix socket.

use crate::layer::UniformValue;

/// Commands sent by `hypaperctl` to the daemon.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DaemonCommand {
    /// Load and display a scene file, optionally on a specific monitor.
    SetWallpaper {
        /// Absolute path to the `.hyscene` file.
        path: String,
        /// Connector name of the target monitor, or `None` for all monitors.
        monitor: Option<String>,
    },
    /// Shut down the daemon gracefully.
    Stop,
    /// Pause rendering (the last rendered frame remains visible).
    Pause,
    /// Resume previously paused rendering.
    Resume,
    /// Reload the current scene from disk.
    Reload,
    /// Request a [`StatusInfo`] snapshot.
    GetStatus,
    /// Adjust a layer's opacity at runtime without reloading the scene.
    SetLayerOpacity {
        /// Target layer identifier.
        id: String,
        /// New opacity value, clamped to `[0.0, 1.0]`.
        opacity: f32,
    },
    /// Override a shader uniform value at runtime.
    SetUniform {
        /// Identifier of the target shader layer.
        layer: String,
        /// Name of the uniform variable in the shader source.
        name: String,
        /// New value to assign.
        value: UniformValue,
    },
}

/// Responses sent by the daemon back to `hypaperctl`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DaemonResponse {
    /// Command was accepted and executed successfully.
    Ok,
    /// Response to a [`DaemonCommand::GetStatus`] request.
    Status(StatusInfo),
    /// The command failed; contains a human-readable description.
    Error(String),
}

/// Point-in-time snapshot of the daemon's runtime state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatusInfo {
    /// Whether the daemon process is currently running.
    pub daemon_running: bool,
    /// Path to the currently loaded scene, if any.
    pub wallpaper: Option<String>,
    /// Number of seconds since the daemon started.
    pub uptime_secs: u64,
}
