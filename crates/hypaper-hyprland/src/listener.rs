//! Asynchronous listener that forwards Hyprland events to a tokio channel.

use hypaper_types::hyprland::HyprlandEvent;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use crate::error::HyprlandError;
use crate::events::parse_event;
use crate::socket::connect_event_socket;

/// Connects to the Hyprland event socket and forwards parsed events to `tx`.
///
/// Reads lines from `.socket2.sock`, parses each one with [`parse_event`], and
/// sends successfully parsed events to the channel. Unrecognised or malformed
/// lines are logged at `DEBUG` level and skipped without aborting the loop.
///
/// The function returns `Ok(())` when the socket is closed by the compositor
/// or when all receivers of `tx` have been dropped.
///
/// # Errors
///
/// Returns [`HyprlandError::SocketNotFound`] or [`HyprlandError::Io`] if the
/// initial connection or a subsequent read fails.
pub async fn start_listener(tx: mpsc::Sender<HyprlandEvent>) -> Result<(), HyprlandError> {
    let stream = connect_event_socket().await?;
    let mut lines = BufReader::new(stream).lines();

    while let Some(line) = lines.next_line().await? {
        match parse_event(&line) {
            Ok(event) => {
                if tx.send(event).await.is_err() {
                    // All receivers dropped; stop listening.
                    break;
                }
            }
            Err(e) => {
                tracing::debug!("skipping unrecognised Hyprland event: {e}");
            }
        }
    }

    Ok(())
}
