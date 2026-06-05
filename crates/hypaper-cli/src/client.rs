//! Unix socket client for sending commands to `hypaperd`.

use std::path::Path;
use std::time::Duration;

use hypaper_types::ipc::{DaemonCommand, DaemonResponse};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const TIMEOUT: Duration = Duration::from_secs(5);

/// Sends `command` to the daemon socket at `socket_path` and returns the response.
///
/// The command is serialised as newline-delimited JSON. After writing, the write
/// half is shut down to signal EOF, then the function attempts to read a JSON
/// response line. If the daemon closes the connection without replying,
/// [`DaemonResponse::Ok`] is returned as an implicit acknowledgement.
///
/// The entire operation (connect + send + receive) is bounded by a 5-second
/// timeout; if it expires, an error is returned indicating the daemon is not
/// reachable.
///
/// # Errors
///
/// Returns an error if the socket cannot be reached within the timeout,
/// serialization fails, or the response cannot be deserialized.
pub async fn send_command(
    socket_path: &Path,
    command: DaemonCommand,
) -> Result<DaemonResponse, anyhow::Error> {
    let socket_path = socket_path.to_owned();

    let fut = async move {
        let stream = tokio::net::UnixStream::connect(&socket_path)
            .await
            .map_err(|_| {
                anyhow::anyhow!("Daemon is not running. Start it with: hypaperctl daemon start")
            })?;

        let (reader, mut writer) = tokio::io::split(stream);

        let mut payload = serde_json::to_string(&command)?;
        payload.push('\n');
        writer.write_all(payload.as_bytes()).await?;
        writer.shutdown().await?;

        let mut line = String::new();
        let n = BufReader::new(reader).read_line(&mut line).await?;

        if n == 0 {
            return Ok(DaemonResponse::Ok);
        }

        let response: DaemonResponse = serde_json::from_str(line.trim())?;
        Ok(response)
    };

    tokio::time::timeout(TIMEOUT, fut).await.map_err(|_| {
        anyhow::anyhow!("Daemon is not running. Start it with: hypaperctl daemon start")
    })?
}

/// Returns `true` if a Wayland socket is already bound at `socket_path`.
///
/// Performs a non-blocking connect attempt; any error (file not found, refused
/// connection, permission denied) is treated as "not running".
pub async fn check_daemon_running(socket_path: &Path) -> bool {
    tokio::net::UnixStream::connect(socket_path).await.is_ok()
}
