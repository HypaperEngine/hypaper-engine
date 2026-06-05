//! Wallpaper lifecycle: per-monitor Wayland surfaces, GPU renderers, and frame rendering.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use hypaper_renderer::renderer::Renderer;
use hypaper_wayland::surface::WaylandSurface;

/// Per-monitor rendering state.
///
/// `renderer` is declared before `surface` so that it is dropped first on
/// cleanup, releasing wgpu's internal raw `wl_surface` pointer before the
/// underlying `WaylandSurface` is destroyed.
pub struct MonitorState {
    /// The active GPU renderer for this monitor.
    renderer: Renderer,
    /// The Wayland layer-shell background surface for this monitor.
    ///
    /// Held for RAII: must outlive `renderer` to keep the raw `wl_surface` pointer valid.
    #[allow(dead_code)]
    surface: WaylandSurface,
}

/// Manages the lifecycle of per-monitor wallpaper surfaces and GPU renderers.
///
/// `monitors` is declared before `wayland_display` so that all `MonitorState`
/// entries (and thus all raw Wayland pointers held inside wgpu) are dropped
/// before the underlying `wl_display` connection is closed.
pub struct WallpaperManager {
    /// Per-monitor rendering state, keyed by connector name (e.g. `"DP-1"`).
    pub monitors: HashMap<String, MonitorState>,
    /// Path of the most recently loaded `.hyscene` file.
    pub current_path: Option<String>,
    /// Whether rendering is currently paused by the user.
    pub paused: bool,
    /// Live Wayland display connection; reused across wallpaper changes.
    wayland_display: Option<hypaper_wayland::display::WaylandDisplay>,
}

impl WallpaperManager {
    /// Creates a `WallpaperManager` with no active wallpapers.
    pub fn new() -> Self {
        Self {
            monitors: HashMap::new(),
            current_path: None,
            paused: false,
            wayland_display: None,
        }
    }

    /// Loads a `.hyscene` bundle and displays it on the specified monitor.
    ///
    /// If `monitor` is `None`, the wallpaper is set on **every** detected
    /// monitor.  If `monitor` is `Some(name)`, only that monitor is updated.
    ///
    /// Per-monitor errors are logged but do not abort other monitors.
    ///
    /// # Errors
    ///
    /// Returns an error if the Wayland connection cannot be established.
    pub async fn set_wallpaper(
        &mut self,
        path: &str,
        monitor: Option<String>,
    ) -> Result<(), anyhow::Error> {
        // Ensure the Wayland connection is open.
        if self.wayland_display.is_none() {
            self.wayland_display = Some(
                hypaper_wayland::display::connect()
                    .map_err(|e| anyhow::anyhow!("Wayland connect: {e}"))?,
            );
        }

        // Collect target monitor names before the mutable borrow below.
        let target_monitors: Vec<String> = match monitor {
            Some(name) => vec![name],
            None => self
                .wayland_display
                .as_ref()
                .map(|d| d.list_monitors().into_iter().map(|m| m.name).collect())
                .unwrap_or_default(),
        };

        if target_monitors.is_empty() {
            tracing::warn!("no monitors detected; wallpaper not set");
        }

        for name in &target_monitors {
            if let Err(e) = self.set_wallpaper_for_monitor(path, name).await {
                tracing::error!(monitor = %name, "failed to set wallpaper: {e}");
            }
        }

        self.current_path = Some(path.to_owned());
        Ok(())
    }

    /// Loads a `.hyscene` bundle and displays it on a single named monitor.
    ///
    /// Any previously active renderer and surface for this monitor are torn
    /// down (in safe order) before the new ones are created.
    ///
    /// # Errors
    ///
    /// Returns an error if the scene cannot be parsed, the Wayland surface
    /// creation fails, the GPU renderer cannot be initialised, or the display
    /// connection is absent.
    pub async fn set_wallpaper_for_monitor(
        &mut self,
        path: &str,
        monitor_name: &str,
    ) -> Result<(), anyhow::Error> {
        let scene_path = Path::new(path);

        let scene = hypaper_scene::parse_hyscene(scene_path)
            .map_err(|e| anyhow::anyhow!("scene parse error: {e}"))?;

        // Resolve the MonitorInfo for the requested connector name.
        let monitor_info = self.wayland_display.as_ref().and_then(|d| {
            d.list_monitors()
                .into_iter()
                .find(|m| m.name == monitor_name)
        });

        if monitor_info.is_none() {
            tracing::warn!(
                monitor = %monitor_name,
                "monitor not found in display list; surface will use compositor default",
            );
        }

        let display = self
            .wayland_display
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no Wayland display"))?;

        let surf_config = hypaper_wayland::surface::SurfaceConfig {
            monitor: monitor_info,
            width: scene.config.resolution[0],
            height: scene.config.resolution[1],
        };

        let surface = hypaper_wayland::surface::create_surface(display, surf_config)
            .map_err(|e| anyhow::anyhow!("create surface for {monitor_name}: {e}"))?;

        let raw = surface.raw_handle();
        let surf_w = surface.width;
        let surf_h = surface.height;

        // Drop old MonitorState for this monitor (renderer first, then surface)
        // while the old surface is still alive so wgpu's raw pointers stay valid.
        self.monitors.remove(monitor_name);

        let mut renderer = Renderer::new(&raw, surf_w, surf_h)
            .await
            .map_err(|e| anyhow::anyhow!("renderer init for {monitor_name}: {e}"))?;

        // Read image-layer assets from the ZIP archive and upload to the GPU.
        let file =
            std::fs::File::open(scene_path).map_err(|e| anyhow::anyhow!("open archive: {e}"))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| anyhow::anyhow!("read archive: {e}"))?;

        for layer in &scene.layers {
            if let hypaper_types::layer::LayerKind::Image(img) = &layer.kind {
                match archive.by_name(&img.src) {
                    Ok(mut entry) => {
                        let mut bytes = Vec::new();
                        entry
                            .read_to_end(&mut bytes)
                            .map_err(|e| anyhow::anyhow!("read asset {}: {e}", img.src))?;
                        renderer
                            .set_image(&bytes)
                            .map_err(|e| anyhow::anyhow!("set image {}: {e}", img.src))?;
                        tracing::info!(
                            src = %img.src,
                            monitor = %monitor_name,
                            "loaded image layer",
                        );
                    }
                    Err(_) => {
                        tracing::warn!(
                            src = %img.src,
                            monitor = %monitor_name,
                            "image asset not found in archive",
                        );
                    }
                }
            }
        }

        self.monitors
            .insert(monitor_name.to_owned(), MonitorState { renderer, surface });

        tracing::info!(
            monitor = %monitor_name,
            width = surf_w,
            height = surf_h,
            "wallpaper set",
        );
        Ok(())
    }

    /// Renders one frame on every active monitor, skipping monitors that fail.
    ///
    /// Does nothing when [`paused`](Self::paused) is `true`.
    pub fn render_frame(&mut self) -> Result<(), anyhow::Error> {
        if self.paused {
            return Ok(());
        }
        for (name, ms) in &mut self.monitors {
            if let Err(e) = ms.renderer.render() {
                tracing::error!(monitor = %name, "render error: {e}");
            }
        }
        Ok(())
    }

    /// Pauses rendering; the last displayed frame remains on screen.
    pub fn pause(&mut self) {
        self.paused = true;
        tracing::info!("wallpaper paused");
    }

    /// Resumes rendering after a previous [`pause`](Self::pause).
    pub fn resume(&mut self) {
        self.paused = false;
        tracing::info!("wallpaper resumed");
    }

    /// Drops all per-monitor renderers and surfaces, clearing the wallpaper.
    ///
    /// The Wayland display connection is retained so that a subsequent
    /// [`set_wallpaper`](Self::set_wallpaper) call can reuse it.
    pub fn stop(&mut self) {
        // HashMap::clear() drops each MonitorState in unspecified order, but
        // within each entry the renderer drops before the surface (declaration
        // order), which is the safe order for raw-pointer cleanup.
        self.monitors.clear();
        self.current_path = None;
        tracing::info!("wallpaper stopped");
    }
}

impl Default for WallpaperManager {
    fn default() -> Self {
        Self::new()
    }
}
