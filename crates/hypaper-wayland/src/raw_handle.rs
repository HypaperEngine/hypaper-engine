//! Raw Wayland handle types for wgpu surface creation.

use std::ffi::c_void;

use wayland_client::Proxy;

use crate::surface::WaylandSurface;

/// Raw Wayland pointers needed to create a wgpu surface.
///
/// Both pointers are valid for the lifetime of the [`WaylandSurface`] that
/// produced them.  Callers must not use them after the originating surface is
/// dropped.
#[derive(Debug, Clone, Copy)]
pub struct RawWindowHandle {
    /// Raw pointer to the `wl_display` connection object.
    pub display_ptr: *mut c_void,
    /// Raw pointer to the `wl_surface` Wayland proxy object.
    pub surface_ptr: *mut c_void,
}

// SAFETY: Both pointers refer to stable Wayland objects owned by the connection
// thread.  Callers moving a RawWindowHandle across threads must ensure that
// the originating WaylandSurface (and thus the connection) is not accessed
// concurrently from multiple threads.
unsafe impl Send for RawWindowHandle {}

impl WaylandSurface {
    /// Returns the raw Wayland pointers for this surface, suitable for passing
    /// to [`hypaper_renderer::context::create_context_for_surface`].
    ///
    /// The returned [`RawWindowHandle`] is valid as long as `self` is alive.
    pub fn raw_handle(&self) -> RawWindowHandle {
        // Retrieve the underlying wl_proxy pointer for the wl_surface.
        // ObjectId::as_ptr() is available on the system (libwayland) backend.
        let surface_ptr = self.wl_surface.id().as_ptr().cast::<c_void>();

        RawWindowHandle {
            display_ptr: self.display_ptr,
            surface_ptr,
        }
    }
}
