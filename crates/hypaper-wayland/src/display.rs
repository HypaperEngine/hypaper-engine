//! Wayland display connection management.

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, BindError, GlobalListContents},
    protocol::{wl_compositor, wl_output, wl_registry, wl_surface},
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

use crate::error::WaylandError;

// ---------------------------------------------------------------------------
// Public monitor description type
// ---------------------------------------------------------------------------

/// Metadata describing a single connected monitor as reported by the compositor.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// Connector name as reported by the compositor (e.g. `"DP-1"`).
    ///
    /// Falls back to `"output-N"` on compositors that do not advertise
    /// `wl_output` version 4 (which introduced the `name` event).
    pub name: String,
    /// Current mode width in pixels, or `0` if no mode event was received.
    pub width: u32,
    /// Current mode height in pixels, or `0` if no mode event was received.
    pub height: u32,
}

// ---------------------------------------------------------------------------
// Internal per-output tracking
// ---------------------------------------------------------------------------

/// Mutable metadata accumulated from `wl_output` events for a single output.
pub(crate) struct OutputData {
    /// Output name; initialised to `"output-N"`, updated by `Name` events.
    pub(crate) name: String,
    /// Current mode width.
    pub(crate) width: u32,
    /// Current mode height.
    pub(crate) height: u32,
}

// ---------------------------------------------------------------------------
// Wayland event-dispatch state
// ---------------------------------------------------------------------------

/// Internal event-dispatch state shared across all Wayland event-queue dispatches.
///
/// Stores transient data produced by protocol events during surface initialisation
/// and monitor enumeration.
pub(crate) struct WaylandState {
    /// Serial and dimensions from the most recent `zwlr_layer_surface_v1::configure` event.
    pub(crate) configure: Option<(u32, u32, u32)>,
    /// Per-output metadata; indexed in parallel with [`WaylandDisplay::outputs`].
    pub(crate) output_data: Vec<OutputData>,
}

// ---------------------------------------------------------------------------
// WaylandDisplay
// ---------------------------------------------------------------------------

/// An active connection to the Wayland compositor.
///
/// Owns the `wayland-client` connection, the event queue, and the global protocol
/// objects needed to create wlr-layer-shell surfaces.
pub struct WaylandDisplay {
    /// The underlying wayland-client connection.
    pub(crate) connection: Connection,
    /// The `wl_compositor` global used to allocate `wl_surface` objects.
    pub(crate) compositor: wl_compositor::WlCompositor,
    /// The `zwlr_layer_shell_v1` global used to create layer surfaces.
    pub(crate) layer_shell: ZwlrLayerShellV1,
    /// All `wl_output` globals advertised by the compositor at connect time.
    pub(crate) outputs: Vec<wl_output::WlOutput>,
    /// The event queue used to dispatch Wayland protocol events.
    pub(crate) event_queue: EventQueue<WaylandState>,
    /// Queue handle used when constructing new protocol objects.
    pub(crate) qh: QueueHandle<WaylandState>,
    /// Current event-dispatch state.
    pub(crate) state: WaylandState,
}

impl WaylandDisplay {
    /// Performs a synchronous roundtrip, processing all pending events.
    pub(crate) fn roundtrip(&mut self) -> Result<(), WaylandError> {
        let WaylandDisplay {
            event_queue, state, ..
        } = self;
        event_queue
            .roundtrip(state)
            .map(|_| ())
            .map_err(|e| WaylandError::Other(e.to_string()))
    }

    /// Returns metadata for every monitor currently connected to the compositor.
    ///
    /// The list is populated during [`connect`] and reflects the state at
    /// connection time.  Hotplug events are not tracked after connect.
    pub fn list_monitors(&self) -> Vec<MonitorInfo> {
        self.state
            .output_data
            .iter()
            .map(|d| MonitorInfo {
                name: d.name.clone(),
                width: d.width,
                height: d.height,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// connect()
// ---------------------------------------------------------------------------

/// Connects to the Wayland compositor via the `WAYLAND_DISPLAY` socket.
///
/// Retrieves the `wl_compositor`, `zwlr_layer_shell_v1`, and all `wl_output` globals,
/// performs a roundtrip to collect monitor metadata, and returns a [`WaylandDisplay`].
///
/// # Errors
///
/// - [`WaylandError::Connection`] if no Wayland compositor is reachable or the
///   initial global enumeration fails.
/// - [`WaylandError::LayerShellNotSupported`] if the compositor does not advertise
///   the `zwlr_layer_shell_v1` protocol extension.
pub fn connect() -> Result<WaylandDisplay, WaylandError> {
    let connection = Connection::connect_to_env().map_err(|_| WaylandError::Connection)?;

    let (globals, event_queue) =
        registry_queue_init::<WaylandState>(&connection).map_err(|_| WaylandError::Connection)?;

    let qh = event_queue.handle();

    let compositor: wl_compositor::WlCompositor = globals
        .bind(&qh, 4..=6, ())
        .map_err(|_| WaylandError::Connection)?;

    let layer_shell: ZwlrLayerShellV1 = globals.bind(&qh, 1..=4, ()).map_err(|e| match e {
        BindError::NotPresent | BindError::UnsupportedVersion => {
            WaylandError::LayerShellNotSupported
        }
    })?;

    // wl_output is a multi-instance global; bind each advertised output manually.
    // We request up to version 4 so that the compositor sends the `Name` event,
    // which gives us the human-readable connector name (e.g. "DP-1").
    let registry = globals.registry().clone();
    let output_globals: Vec<_> = globals
        .contents()
        .clone_list()
        .into_iter()
        .filter(|g| g.interface == "wl_output")
        .collect();

    let mut state = WaylandState {
        configure: None,
        output_data: Vec::new(),
    };
    let mut event_queue = event_queue;

    let outputs: Vec<wl_output::WlOutput> = output_globals
        .into_iter()
        .enumerate()
        .map(|(index, g)| {
            // Pre-populate with a fallback name; will be overwritten by the
            // `Name` event on compositors that support wl_output v4.
            state.output_data.push(OutputData {
                name: format!("output-{index}"),
                width: 0,
                height: 0,
            });
            registry.bind(g.name, g.version.min(4), &qh, index)
        })
        .collect();

    event_queue
        .roundtrip(&mut state)
        .map_err(|_| WaylandError::Connection)?;

    tracing::info!(
        outputs = outputs.len(),
        monitors = ?state
            .output_data
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>(),
        "connected to Wayland display",
    );

    Ok(WaylandDisplay {
        connection,
        compositor,
        layer_shell,
        outputs,
        event_queue,
        qh,
        state,
    })
}

// ---------------------------------------------------------------------------
// Dispatch implementations
// ---------------------------------------------------------------------------

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WaylandState {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Dynamic global events are handled internally by GlobalListContents.
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                state.configure = Some((serial, width, height));
            }
            zwlr_layer_surface_v1::Event::Closed => {
                tracing::debug!(?surface, "layer surface closed by compositor");
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, usize> for WaylandState {
    fn event(
        state: &mut Self,
        _output: &wl_output::WlOutput,
        event: wl_output::Event,
        data: &usize,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let index = *data;
        if let Some(info) = state.output_data.get_mut(index) {
            match event {
                // Available on wl_output v4+: the connector name (e.g. "DP-1").
                wl_output::Event::Name { name } => {
                    info.name = name;
                }
                // Sent for every advertised output mode; we keep the last one
                // (compositors send the current mode last).
                wl_output::Event::Mode { width, height, .. } if width > 0 && height > 0 => {
                    info.width = width as u32;
                    info.height = height as u32;
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(WaylandState: ignore wl_compositor::WlCompositor);
delegate_noop!(WaylandState: ignore ZwlrLayerShellV1);
delegate_noop!(WaylandState: ignore wl_surface::WlSurface);
