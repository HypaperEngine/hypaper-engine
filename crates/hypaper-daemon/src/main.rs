//! `hypaperd` — main orchestrator daemon for Hypaper Engine.
//!
//! Spawns the Wayland, renderer, and Hyprland subsystems, then exposes a Unix
//! domain socket for `hypaperctl` to send IPC commands at runtime.

mod config;
mod ipc;
mod state;

use hypaper_types::ipc::DaemonCommand;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .expect("failed to initialise tracing subscriber");

    let cfg = config::load_config();
    tracing::info!(
        socket = %cfg.socket_path.display(),
        max_fps = cfg.max_fps,
        "hypaperd starting",
    );

    let mut daemon_state = state::DaemonState::new();

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<DaemonCommand>(64);
    let (event_tx, mut event_rx) = mpsc::channel(64);

    // Spawn the Hyprland event listener.
    tokio::spawn(async move {
        if let Err(e) = hypaper_hyprland::start_listener(event_tx).await {
            tracing::error!("Hyprland listener stopped: {e}");
        }
    });

    // Spawn the IPC server.
    let ipc_tx = cmd_tx.clone();
    let socket_path = cfg.socket_path.clone();
    tokio::spawn(async move {
        if let Err(e) = ipc::start_ipc_server(&socket_path, ipc_tx).await {
            tracing::error!("IPC server stopped: {e}");
        }
    });

    // Main event loop — processes daemon commands and Hyprland events.
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(DaemonCommand::Stop) => {
                        tracing::info!("received Stop, shutting down");
                        break;
                    }
                    Some(DaemonCommand::Pause) => {
                        daemon_state.paused = true;
                        tracing::info!("wallpaper paused");
                    }
                    Some(DaemonCommand::Resume) => {
                        daemon_state.paused = false;
                        tracing::info!("wallpaper resumed");
                    }
                    Some(DaemonCommand::GetStatus) => {
                        let s = daemon_state.to_status_info();
                        tracing::info!(
                            wallpaper = ?s.wallpaper,
                            uptime_secs = s.uptime_secs,
                            paused = daemon_state.paused,
                            "daemon status",
                        );
                    }
                    Some(_) => {
                        tracing::warn!("Command not yet implemented");
                    }
                    None => break,
                }
            }
            event = event_rx.recv() => {
                match event {
                    Some(ev) => tracing::debug!(?ev, "Hyprland event"),
                    None => tracing::warn!("Hyprland event channel closed"),
                }
            }
        }
    }

    tracing::info!("hypaperd stopped");
    Ok(())
}
