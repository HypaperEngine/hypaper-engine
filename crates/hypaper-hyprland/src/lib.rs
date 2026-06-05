//! Hyprland IPC client and event listener.
//!
//! Connects to the Hyprland Unix socket, issues IPC commands, and subscribes
//! to workspace/monitor events so the daemon can react to compositor changes.

#![warn(missing_docs)]

pub mod error;
pub mod events;
pub mod listener;
pub mod socket;

pub use error::HyprlandError;
pub use listener::start_listener;
