//! Hyprland-specific configuration and IPC event types.

use std::collections::HashMap;

/// Hyprland integration settings embedded in a scene.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HyprlandConfig {
    /// Pause the wallpaper when any window enters fullscreen.
    pub pause_on_fullscreen: bool,
    /// Pause the wallpaper when the system is on battery power.
    pub pause_on_battery: bool,
    /// Enable cursor-driven parallax effect.
    pub parallax: bool,
    /// Parallax motion intensity multiplier.
    pub parallax_intensity: f32,
    /// Per-workspace scene path overrides, keyed by workspace ID.
    pub workspaces: HashMap<u32, String>,
    /// Scene path used for workspaces not listed in `workspaces`.
    pub default_workspace: Option<String>,
}

/// Events emitted by the Hyprland IPC socket.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HyprlandEvent {
    /// The active workspace changed.
    WorkspaceChanged {
        /// Numeric workspace identifier.
        id: u32,
    },
    /// A window received focus.
    WindowFocused {
        /// Window class (application name).
        class: String,
        /// Window title.
        title: String,
    },
    /// A window entered fullscreen mode.
    FullscreenEntered,
    /// The fullscreen window was closed or restored.
    FullscreenExited,
    /// A new monitor was connected.
    MonitorAdded {
        /// Connector name (e.g. `"DP-1"`).
        name: String,
    },
    /// A monitor was disconnected.
    MonitorRemoved {
        /// Connector name of the removed monitor.
        name: String,
    },
    /// The focused monitor changed.
    MonitorFocused {
        /// Connector name of the newly focused monitor.
        name: String,
    },
}
