//! Unix socket client for sending commands to `hypaperd`.

use std::path::Path;

use hypaper_types::ipc::{DaemonCommand, DaemonResponse};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Sends `command` to the daemon socket at `socket_path` and returns the response.
///
/// The command is serialised as newline-delimited JSON. After writing, the write
/// half is shut down to signal EOF, then the function attempts to read a JSON
/// response line. If the daemon closes the connection without replying,
/// [`DaemonResponse::Ok`] is returned as an implicit acknowledgement.
///
/// # Errors
///
/// Returns an error if the socket cannot be reached, serialization fails, or
/// the response cannot be deserialized.
pub async fn send_command(
    socket_path: &Path,
    command: DaemonCommand,
) -> Result<DaemonResponse, anyhow::Error> {
    let stream = tokio::net::UnixStream::connect(socket_path).await?;
    let (reader, mut writer) = tokio::io::split(stream);

    // Send the command as a single JSON line.
    let mut payload = serde_json::to_string(&command)?;
    payload.push('\n');
    writer.write_all(payload.as_bytes()).await?;
    writer.shutdown().await?;

    // Read an optional response line from the daemon.
    let mut line = String::new();
    let n = BufReader::new(reader).read_line(&mut line).await?;

    if n == 0 {
        // Daemon closed the connection without sending a response.
        return Ok(DaemonResponse::Ok);
    }

    let response = serde_json::from_str(line.trim())?;
    Ok(response)
}
