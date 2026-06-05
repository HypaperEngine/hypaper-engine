//! `hypaperctl` — command-line interface for controlling the Hypaper Engine daemon.
//!
//! Connects to the daemon Unix socket and serialises user commands (load scene,
//! list monitors, stop, …) into msgpack IPC messages.

mod cli;
mod client;

use clap::Parser;
use cli::{Cli, Commands, DaemonAction};
use hypaper_types::ipc::{DaemonCommand, DaemonResponse};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .expect("failed to initialise tracing");

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
        .map(|d| std::path::PathBuf::from(d).join("hypaper.sock"))
        .unwrap_or_else(|_| std::path::PathBuf::from("/run/user/1000/hypaper.sock"));

    let json_output = matches!(args.command, Commands::Status { json: true });

    let Some(daemon_cmd) = to_daemon_command(args.command) else {
        return Ok(());
    };

    let response = client::send_command(&socket_path, daemon_cmd).await?;
    print_response(response, json_output)
}

/// Converts a CLI [`Commands`] variant into a [`DaemonCommand`], if applicable.
///
/// Returns `None` for commands that are handled locally (completions) or that
/// are not yet implemented (list, daemon start).
fn to_daemon_command(cmd: Commands) -> Option<DaemonCommand> {
    match cmd {
        Commands::Set { wallpaper, monitor } => Some(DaemonCommand::SetWallpaper {
            path: wallpaper,
            monitor,
        }),
        Commands::Stop => Some(DaemonCommand::Stop),
        Commands::Pause => Some(DaemonCommand::Pause),
        Commands::Resume => Some(DaemonCommand::Resume),
        Commands::Reload => Some(DaemonCommand::Reload),
        Commands::Status { .. } => Some(DaemonCommand::GetStatus),
        Commands::List { .. } => {
            println!("list command not yet implemented");
            None
        }
        Commands::Daemon { action } => match action {
            DaemonAction::Start => {
                println!("daemon start not yet implemented");
                None
            }
            DaemonAction::Stop => Some(DaemonCommand::Stop),
            DaemonAction::Status => Some(DaemonCommand::GetStatus),
        },
        Commands::Completions { .. } => None,
    }
}

/// Prints `response` to stdout — human-readable by default, JSON if `json` is set.
fn print_response(response: DaemonResponse, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }
    match response {
        DaemonResponse::Ok => println!("OK"),
        DaemonResponse::Status(info) => {
            println!("daemon running: {}", info.daemon_running);
            println!(
                "wallpaper:      {}",
                info.wallpaper.as_deref().unwrap_or("(none)")
            );
            println!("uptime:         {}s", info.uptime_secs);
        }
        DaemonResponse::Error(e) => eprintln!("Error: {e}"),
    }
    Ok(())
}
