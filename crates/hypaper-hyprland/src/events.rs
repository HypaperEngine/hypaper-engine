//! Parsing of raw Hyprland IPC event lines into typed [`HyprlandEvent`] values.
//!
//! Hyprland emits events on `.socket2.sock` in the format `event>>data\n`.

use hypaper_types::hyprland::HyprlandEvent;

use crate::error::HyprlandError;

/// Parses one line from the Hyprland event socket into a [`HyprlandEvent`].
///
/// Expected line format: `event>>data` (no trailing newline).
///
/// Supported events: `workspace`, `focusedwindow`, `fullscreen`,
/// `monitoradded`, `monitorremoved`, `activemon`.
///
/// # Errors
///
/// Returns [`HyprlandError::ParseError`] for malformed lines, unrecognised
/// events, or data that cannot be converted to the expected types.
pub fn parse_event(line: &str) -> Result<HyprlandEvent, HyprlandError> {
    let (event, data) = line
        .split_once(">>")
        .ok_or_else(|| HyprlandError::ParseError(format!("malformed event line: {line:?}")))?;

    match event {
        "workspace" => {
            let id = data.trim().parse::<u32>().map_err(|_| {
                HyprlandError::ParseError(format!("invalid workspace id: {data:?}"))
            })?;
            Ok(HyprlandEvent::WorkspaceChanged { id })
        }

        "focusedwindow" => {
            // Format: class,title — title may itself contain commas.
            let (class, title) = data.split_once(',').ok_or_else(|| {
                HyprlandError::ParseError(format!("malformed focusedwindow data: {data:?}"))
            })?;
            Ok(HyprlandEvent::WindowFocused {
                class: class.to_owned(),
                title: title.to_owned(),
            })
        }

        "fullscreen" => match data.trim() {
            "1" => Ok(HyprlandEvent::FullscreenEntered),
            "0" => Ok(HyprlandEvent::FullscreenExited),
            other => Err(HyprlandError::ParseError(format!(
                "invalid fullscreen value: {other:?}"
            ))),
        },

        "monitoradded" => Ok(HyprlandEvent::MonitorAdded {
            name: data.trim().to_owned(),
        }),

        "monitorremoved" => Ok(HyprlandEvent::MonitorRemoved {
            name: data.trim().to_owned(),
        }),

        // Format: monitorname,workspacename — we only need the monitor name.
        "activemon" => {
            let (name, _workspace) = data.split_once(',').ok_or_else(|| {
                HyprlandError::ParseError(format!("malformed activemon data: {data:?}"))
            })?;
            Ok(HyprlandEvent::MonitorFocused {
                name: name.to_owned(),
            })
        }

        other => Err(HyprlandError::ParseError(format!(
            "unknown event: {other:?}"
        ))),
    }
}
