//! `hypaperd` — main orchestrator daemon for Hypaper Engine.
//!
//! Spawns the Wayland, renderer, and Hyprland subsystems, then exposes a Unix
//! domain socket for `hypaperctl` to send IPC commands at runtime.

mod config;
mod ipc;
mod state;
mod wallpaper;

use std::time::Duration;

use hypaper_types::ipc::DaemonCommand;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| anyhow::anyhow!("failed to init tracing: {e}"))?;

    let cfg = config::load_config();
    tracing::info!(
        socket = %cfg.socket_path.display(),
        max_fps = cfg.max_fps,
        "hypaperd starting",
    );

    let daemon_state = state::DaemonState::new();
    let mut wallpaper_manager = wallpaper::WallpaperManager::new();

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

    // Render ticker: fires at `max_fps` frames per second.
    let frame_ns = 1_000_000_000u64 / cfg.max_fps as u64;
    let mut render_ticker = tokio::time::interval(Duration::from_nanos(frame_ns));
    render_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Main event loop — interleaves frame rendering with command and event handling.
    loop {
        tokio::select! {
            _ = render_ticker.tick() => {
                if let Err(e) = wallpaper_manager.render_frame() {
                    tracing::error!("render frame error: {e}");
                }
            }

            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(DaemonCommand::SetWallpaper { path, monitor }) => {
                        tracing::info!(%path, monitor = ?monitor, "setting wallpaper");
                        if let Err(e) = wallpaper_manager.set_wallpaper(&path, monitor).await {
                            tracing::error!("set_wallpaper failed: {e}");
                        }
                    }
                    Some(DaemonCommand::Stop) => {
                        tracing::info!("received Stop, shutting down");
                        wallpaper_manager.stop();
                        break;
                    }
                    Some(DaemonCommand::Pause) => {
                        wallpaper_manager.pause();
                    }
                    Some(DaemonCommand::Resume) => {
                        wallpaper_manager.resume();
                    }
                    Some(DaemonCommand::GetStatus) => {
                        let uptime_secs = daemon_state.start_time.elapsed().as_secs();
                        tracing::info!(
                            wallpaper = ?wallpaper_manager.current_path,
                            uptime_secs,
                            paused = wallpaper_manager.paused,
                            "daemon status",
                        );
                    }
                    Some(DaemonCommand::Reload) => {
                        if let Some(path) = wallpaper_manager.current_path.clone() {
                            tracing::info!(%path, "reloading wallpaper");
                            if let Err(e) = wallpaper_manager.set_wallpaper(&path, None).await {
                                tracing::error!("reload failed: {e}");
                            }
                        } else {
                            tracing::warn!("Reload requested but no wallpaper is loaded");
                        }
                    }
                    Some(cmd) => {
                        tracing::warn!(?cmd, "command not yet implemented");
                    }
                    None => break,
                }
            }

            event = event_rx.recv() => {
                match event {
                    Some(ev) => {
                        tracing::debug!(?ev, "Hyprland event");
                        if let Err(e) = wallpaper_manager.on_hyprland_event(&ev) {
                            tracing::warn!("script error on Hyprland event: {e}");
                        }
                    }
                    None => tracing::warn!("Hyprland event channel closed"),
                }
            }
        }
    }

    tracing::info!("hypaperd stopped");
    Ok(())
}
