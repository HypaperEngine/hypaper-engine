//! Clap argument definitions for `hypaperctl`.

/// Top-level CLI entry point parsed from `argv`.
#[derive(Debug, clap::Parser)]
#[command(name = "hypaperctl", about = "Control the Hypaper Engine daemon")]
pub struct Cli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// All subcommands exposed by `hypaperctl`.
#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    /// Set a wallpaper by name or file path.
    Set {
        /// Wallpaper name or path to a `.hyscene` file.
        wallpaper: String,
        /// Target a specific monitor by connector name (e.g. `DP-1`).
        #[arg(long)]
        monitor: Option<String>,
    },
    /// Stop the current wallpaper and clear the surface.
    Stop,
    /// Pause the current wallpaper (last frame stays visible).
    Pause,
    /// Resume the paused wallpaper.
    Resume,
    /// Reload the current wallpaper from disk.
    Reload,
    /// Show daemon status.
    Status {
        /// Output the status as JSON instead of human-readable text.
        #[arg(long)]
        json: bool,
    },
    /// List available wallpapers.
    List {
        /// Show only user-installed wallpapers.
        #[arg(long)]
        installed: bool,
        /// Show only system-wide wallpapers.
        #[arg(long)]
        system: bool,
    },
    /// Manage the `hypaperd` daemon process.
    Daemon {
        /// Daemon lifecycle action.
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Print shell completion script to stdout.
    Completions {
        /// Target shell.
        shell: clap_complete::Shell,
    },
}

/// Actions available under the `daemon` subcommand.
#[derive(Debug, clap::Subcommand)]
pub enum DaemonAction {
    /// Start the daemon in the background.
    Start,
    /// Stop the running daemon.
    Stop,
    /// Show the daemon's current status.
    Status,
}
