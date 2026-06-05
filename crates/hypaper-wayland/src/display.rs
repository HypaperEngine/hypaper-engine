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

/// Internal event-dispatch state shared across all Wayland event-queue dispatches.
///
/// Stores transient data produced by protocol events during surface initialisation.
pub(crate) struct WaylandState {
    /// Serial and dimensions from the most recent `zwlr_layer_surface_v1::configure` event.
    pub(crate) configure: Option<(u32, u32, u32)>,
}

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
}

/// Connects to the Wayland compositor via the `WAYLAND_DISPLAY` socket.
///
/// Retrieves the `wl_compositor`, `zwlr_layer_shell_v1`, and all `wl_output` globals
/// and stores them in the returned [`WaylandDisplay`].
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
    let registry = globals.registry().clone();
    let output_globals: Vec<_> = globals
        .contents()
        .clone_list()
        .into_iter()
        .filter(|g| g.interface == "wl_output")
        .collect();

    let mut state = WaylandState { configure: None };
    let mut event_queue = event_queue;

    let outputs: Vec<wl_output::WlOutput> = output_globals
        .into_iter()
        .map(|g| registry.bind(g.name, g.version.min(3), &qh, ()))
        .collect();

    event_queue
        .roundtrip(&mut state)
        .map_err(|_| WaylandError::Connection)?;

    tracing::info!(outputs = outputs.len(), "connected to Wayland display");

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

// Dispatch implementations -------------------------------------------------

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

delegate_noop!(WaylandState: ignore wl_compositor::WlCompositor);
delegate_noop!(WaylandState: ignore ZwlrLayerShellV1);
delegate_noop!(WaylandState: ignore wl_output::WlOutput);
delegate_noop!(WaylandState: ignore wl_surface::WlSurface);
