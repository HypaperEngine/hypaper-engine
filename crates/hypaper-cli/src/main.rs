//! `hypaperctl` — command-line interface for controlling the Hypaper Engine daemon.
//!
//! Connects to the daemon Unix socket and serialises user commands (load scene,
//! list monitors, stop, …) into JSON IPC messages.

mod cli;
mod client;

use std::path::PathBuf;

use clap::Parser;
use cli::{Cli, Commands, DaemonAction};
use hypaper_types::ipc::{DaemonCommand, DaemonResponse};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| anyhow::anyhow!("failed to init tracing: {e}"))?;

    let args = Cli::parse();

    // Shell completions are generated locally — no daemon connection needed.
    if let Commands::Completions { shell } = args.command {
        use clap::CommandFactory;
        clap_complete::generate(
            shell,
            &mut Cli::command(),
            "hypaperctl",
            &mut std::io::stdout(),
        );
        return Ok(());
    }

    let socket_path = std::env::var("XDG_RUNTIME_DIR")
        .map(|d| PathBuf::from(d).join("hypaper.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/hypaper.sock"));

    // Daemon::Start spawns hypaperd and exits — no running daemon required.
    if let Commands::Daemon {
        action: DaemonAction::Start,
    } = &args.command
    {
        std::process::Command::new("hypaperd")
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to start hypaperd: {e}"))?;
        println!("Daemon started");
        return Ok(());
    }

    // List reads the filesystem only — no IPC needed; skip daemon check.
    if let Commands::List { installed, system } = &args.command {
        return cmd_list(*installed, *system);
    }

    // All remaining commands require a running daemon.
    if !client::check_daemon_running(&socket_path).await {
        eprintln!("Daemon is not running. Start it with: hypaperctl daemon start");
        std::process::exit(1);
    }

    let (daemon_cmd, is_status, want_json) = match args.command {
        Commands::Set { wallpaper, monitor } => (
            DaemonCommand::SetWallpaper {
                path: wallpaper,
                monitor,
            },
            false,
            false,
        ),
        Commands::Stop => (DaemonCommand::Stop, false, false),
        Commands::Pause => (DaemonCommand::Pause, false, false),
        Commands::Resume => (DaemonCommand::Resume, false, false),
        Commands::Reload => (DaemonCommand::Reload, false, false),
        Commands::Status { json } => (DaemonCommand::GetStatus, true, json),
        Commands::Daemon {
            action: DaemonAction::Stop,
        } => (DaemonCommand::Stop, false, false),
        Commands::Daemon {
            action: DaemonAction::Status,
        } => (DaemonCommand::GetStatus, true, false),
        // Handled above.
        Commands::Completions { .. }
        | Commands::List { .. }
        | Commands::Daemon {
            action: DaemonAction::Start,
        } => unreachable!(),
    };

    let response = client::send_command(&socket_path, daemon_cmd).await?;
    print_response(response, is_status, want_json)
}

/// Lists `.hyscene` files found in the user and/or system wallpaper directories.
fn cmd_list(only_installed: bool, only_system: bool) -> anyhow::Result<()> {
    let user_dir = std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".local/share/hypaper/wallpapers"));
    let system_dir = PathBuf::from("/usr/share/hypaper/wallpapers");

    let dirs: Vec<PathBuf> = match (only_installed, only_system) {
        (true, false) => user_dir.into_iter().collect(),
        (false, true) => vec![system_dir],
        _ => user_dir
            .into_iter()
            .chain(std::iter::once(system_dir))
            .collect(),
    };

    let mut found_any = false;
    for dir in &dirs {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("hyscene") {
                println!("{}", path.display());
                found_any = true;
            }
        }
    }

    if !found_any {
        println!("No wallpapers found.");
    }
    Ok(())
}

/// Prints `response` to stdout.
///
/// When `is_status` is `true` and the response carries a [`DaemonResponse::Status`],
/// the output is formatted as an aligned table (or pretty-printed JSON when
/// `want_json` is `true`).
fn print_response(
    response: DaemonResponse,
    is_status: bool,
    want_json: bool,
) -> anyhow::Result<()> {
    match response {
        DaemonResponse::Status(ref info) if is_status => {
            if want_json {
                println!("{}", serde_json::to_string_pretty(info)?);
            } else {
                let state = if info.daemon_running {
                    "running"
                } else {
                    "stopped"
                };
                let wallpaper = info.wallpaper.as_deref().unwrap_or("(none)");
                let uptime = format!("{}s", info.uptime_secs);
                println!("{:<16} {}", "daemon:", state);
                println!("{:<16} {}", "wallpaper:", wallpaper);
                println!("{:<16} {}", "uptime:", uptime);
            }
        }
        DaemonResponse::Ok => {
            if is_status {
                // Daemon acknowledged GetStatus but sent no payload — it is running.
                if want_json {
                    println!("{{\"daemon_running\":true}}");
                } else {
                    println!("{:<16} running", "daemon:");
                    println!("{:<16} (none)", "wallpaper:");
                    println!("{:<16} unknown", "uptime:");
                }
            } else {
                println!("OK");
            }
        }
        DaemonResponse::Error(e) => eprintln!("Error: {e}"),
        // Status response when is_status == false — just print OK.
        DaemonResponse::Status(_) => println!("OK"),
    }
    Ok(())
}
