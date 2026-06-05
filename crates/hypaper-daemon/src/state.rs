//! Runtime state of the `hypaperd` daemon.

use std::time::Instant;

use hypaper_types::ipc::StatusInfo;

/// Internal runtime state of the `hypaperd` daemon.
///
/// Mutated by the main event loop in response to [`DaemonCommand`] messages.
///
/// [`DaemonCommand`]: hypaper_types::ipc::DaemonCommand
#[derive(Debug)]
pub struct DaemonState {
    /// Path to the currently loaded `.hyscene` file, if any.
    pub current_wallpaper: Option<String>,
    /// Whether rendering is currently paused.
    pub paused: bool,
    /// Instant at which the daemon was started; used to compute uptime.
    pub start_time: Instant,
}

impl DaemonState {
    /// Creates a fresh `DaemonState` with no wallpaper loaded and rendering active.
    pub fn new() -> Self {
        Self {
            current_wallpaper: None,
            paused: false,
            start_time: Instant::now(),
        }
    }

    /// Builds a [`StatusInfo`] snapshot from the current state.
    pub fn to_status_info(&self) -> StatusInfo {
        StatusInfo {
            daemon_running: true,
            wallpaper: self.current_wallpaper.clone(),
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }
}

impl Default for DaemonState {
    fn default() -> Self {
        Self::new()
    }
}
