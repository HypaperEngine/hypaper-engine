//! Unix domain socket IPC server for receiving `hypaperctl` commands.

use std::path::Path;

use hypaper_types::ipc::DaemonCommand;
use tokio::io::AsyncBufReadExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc;

/// Binds a Unix domain socket at `socket_path` and forwards deserialised
/// [`DaemonCommand`] messages to `tx`.
///
/// Each accepted connection is served by a dedicated Tokio task. Commands are
/// expected as newline-delimited JSON; the connection is closed on the first
/// malformed line. MsgPack support will follow once format negotiation is
/// implemented.
///
/// # Errors
///
/// Returns an error if the socket cannot be bound or a fatal `accept` failure
/// occurs.
pub async fn start_ipc_server(
    socket_path: &Path,
    tx: mpsc::Sender<DaemonCommand>,
) -> Result<(), anyhow::Error> {
    // Remove a stale socket file left by a previous crash.
    let _ = tokio::fs::remove_file(socket_path).await;

    let listener = UnixListener::bind(socket_path)?;
    tracing::info!(socket = %socket_path.display(), "IPC server listening");

    loop {
        let (stream, _addr) = listener.accept().await?;
        let tx = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, tx).await {
                tracing::warn!("IPC connection closed: {e}");
            }
        });
    }
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    tx: mpsc::Sender<DaemonCommand>,
) -> Result<(), anyhow::Error> {
    let mut lines = tokio::io::BufReader::new(stream).lines();
    while let Some(line) = lines.next_line().await? {
        let cmd: DaemonCommand = serde_json::from_str(&line)?;
        tracing::debug!(?cmd, "received IPC command");
        tx.send(cmd).await?;
    }
    Ok(())
}
