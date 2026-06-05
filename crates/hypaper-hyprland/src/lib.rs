//! Hyprland IPC client and event listener.
//!
//! Connects to the Hyprland Unix socket, issues IPC commands, and subscribes
//! to workspace/monitor events so the daemon can react to compositor changes.

#![warn(missing_docs)]
