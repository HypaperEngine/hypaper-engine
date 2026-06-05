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

#[cfg(test)]
mod tests {
    use hypaper_types::hyprland::HyprlandEvent;

    use super::*;
    use crate::error::HyprlandError;

    #[test]
    fn test_workspace_event() {
        // Arrange
        let line = "workspace>>2";

        // Act
        let event = parse_event(line).unwrap();

        // Assert
        assert!(matches!(event, HyprlandEvent::WorkspaceChanged { id: 2 }));
    }

    #[test]
    fn test_focusedwindow_event() {
        // Arrange
        let line = "focusedwindow>>firefox,Mozilla Firefox";

        // Act
        let event = parse_event(line).unwrap();

        // Assert
        let HyprlandEvent::WindowFocused { class, title } = event else {
            panic!("expected WindowFocused");
        };
        assert_eq!(class, "firefox");
        assert_eq!(title, "Mozilla Firefox");
    }

    #[test]
    fn test_fullscreen_entered() {
        // Arrange
        let line = "fullscreen>>1";

        // Act
        let event = parse_event(line).unwrap();

        // Assert
        assert!(matches!(event, HyprlandEvent::FullscreenEntered));
    }

    #[test]
    fn test_fullscreen_exited() {
        // Arrange
        let line = "fullscreen>>0";

        // Act
        let event = parse_event(line).unwrap();

        // Assert
        assert!(matches!(event, HyprlandEvent::FullscreenExited));
    }

    #[test]
    fn test_monitor_added() {
        // Arrange
        let line = "monitoradded>>DP-1";

        // Act
        let event = parse_event(line).unwrap();

        // Assert
        let HyprlandEvent::MonitorAdded { name } = event else {
            panic!("expected MonitorAdded");
        };
        assert_eq!(name, "DP-1");
    }

    #[test]
    fn test_unknown_event_returns_error() {
        // Arrange
        let line = "unknownevent>>somedata";

        // Act
        let result = parse_event(line);

        // Assert
        assert!(matches!(result, Err(HyprlandError::ParseError(_))));
    }

    #[test]
    fn test_focusedwindow_title_with_comma() {
        // Arrange
        let line = "focusedwindow>>firefox,My Video, Part 2";

        // Act
        let event = parse_event(line).unwrap();

        // Assert
        let HyprlandEvent::WindowFocused { class, title } = event else {
            panic!("expected WindowFocused");
        };
        assert_eq!(class, "firefox");
        assert_eq!(title, "My Video, Part 2");
    }
}
