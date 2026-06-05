//! Daemon configuration loading and defaults.

use std::path::PathBuf;

/// IPC serialization format used on the Unix domain socket.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum IpcFormat {
    /// MessagePack binary encoding (compact, fast).
    MsgPack,
    /// Newline-delimited JSON (human-readable, easy to debug with `nc`).
    Json,
}

/// Runtime configuration for `hypaperd`.
///
/// Loaded from `~/.config/hypaper/daemon.toml` on startup; falls back to
/// compiled-in defaults if the file is absent.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DaemonConfig {
    /// Directory where `.hyscene` bundles are searched.
    pub wallpapers_dir: PathBuf,
    /// Log filter string forwarded to `tracing-subscriber` (e.g. `"info"`).
    pub log_level: String,
    /// Path of the Unix domain socket exposed to `hypaperctl`.
    pub socket_path: PathBuf,
    /// IPC serialization format.
    pub ipc_format: IpcFormat,
    /// Maximum render frame rate in frames per second.
    pub max_fps: u32,
    /// When `true`, switch to `fps_on_battery` while on battery power.
    pub reduce_on_battery: bool,
    /// Frame rate used when `reduce_on_battery` is active.
    pub fps_on_battery: u32,
    /// Pause rendering while a fullscreen window occupies the monitor.
    pub pause_on_fullscreen: bool,
    /// Pause rendering while the system is on battery power.
    pub pause_on_battery: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let socket_path = std::env::var("XDG_RUNTIME_DIR")
            .map(|d| PathBuf::from(d).join("hypaper.sock"))
            .unwrap_or_else(|_| PathBuf::from("/tmp/hypaper.sock"));

        let wallpapers_dir = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".local/share/hypaper/scenes"))
            .unwrap_or_else(|_| PathBuf::from("/usr/share/hypaper/scenes"));

        Self {
            wallpapers_dir,
            log_level: "info".into(),
            socket_path,
            ipc_format: IpcFormat::Json,
            max_fps: 60,
            reduce_on_battery: true,
            fps_on_battery: 30,
            pause_on_fullscreen: true,
            pause_on_battery: false,
        }
    }
}

/// Loads `~/.config/hypaper/daemon.toml` and returns a [`DaemonConfig`].
///
/// Falls back to [`DaemonConfig::default`] if the file does not exist.
/// Full TOML parsing will be added once the `toml` dependency is wired in.
pub fn load_config() -> DaemonConfig {
    let config_path = std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config/hypaper/daemon.toml"));

    if let Some(ref path) = config_path {
        if path.exists() {
            tracing::info!(
                path = %path.display(),
                "found daemon.toml — TOML parsing not yet implemented, using defaults",
            );
        }
    }

    tracing::info!("using default daemon configuration");
    DaemonConfig::default()
}
