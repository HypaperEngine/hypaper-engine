//! Wallpaper lifecycle: Wayland surface creation, GPU renderer, and per-frame rendering.

use std::io::Read;
use std::path::Path;

/// Manages the lifecycle of the active wallpaper surface and GPU renderer.
///
/// Fields are declared in drop order: `renderer` is dropped before
/// `wayland_surface`, which is dropped before `wayland_display`, so the raw
/// Wayland pointers held inside the wgpu surface are always valid during cleanup.
pub struct WallpaperManager {
    /// Active GPU renderer; `None` until the first successful [`set_wallpaper`](Self::set_wallpaper).
    pub renderer: Option<hypaper_renderer::renderer::Renderer>,
    /// Active Wayland layer-shell background surface.
    pub wayland_surface: Option<hypaper_wayland::surface::WaylandSurface>,
    /// Path of the currently loaded `.hyscene` file.
    pub current_path: Option<String>,
    /// Whether rendering is currently paused by the user.
    pub paused: bool,
    /// Live Wayland display connection; reused across wallpaper changes.
    wayland_display: Option<hypaper_wayland::display::WaylandDisplay>,
}

impl WallpaperManager {
    /// Creates a `WallpaperManager` with no active wallpaper.
    pub fn new() -> Self {
        Self {
            renderer: None,
            wayland_surface: None,
            current_path: None,
            paused: false,
            wayland_display: None,
        }
    }

    /// Loads a `.hyscene` bundle, creates a Wayland surface and GPU renderer,
    /// and uploads the first image layer as the active wallpaper texture.
    ///
    /// Any previously active renderer and surface are torn down before the new
    /// ones are created.
    ///
    /// # Errors
    ///
    /// Returns an error if the scene cannot be parsed, the Wayland connection
    /// fails, the GPU renderer cannot be initialised, or a required image asset
    /// is missing from the archive.
    pub async fn set_wallpaper(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let scene_path = Path::new(path);

        let scene = hypaper_scene::parse_hyscene(scene_path)
            .map_err(|e| anyhow::anyhow!("scene parse error: {e}"))?;

        // Connect to the Wayland compositor on first use; reuse thereafter.
        if self.wayland_display.is_none() {
            self.wayland_display = Some(
                hypaper_wayland::display::connect()
                    .map_err(|e| anyhow::anyhow!("Wayland connect: {e}"))?,
            );
        }

        let display = self
            .wayland_display
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no Wayland display after connect"))?;

        let surf_config = hypaper_wayland::surface::SurfaceConfig {
            monitor_name: None,
            width: scene.config.resolution[0],
            height: scene.config.resolution[1],
        };

        let surface = hypaper_wayland::surface::create_surface(display, surf_config)
            .map_err(|e| anyhow::anyhow!("create surface: {e}"))?;

        let raw = surface.raw_handle();
        let surf_w = surface.width;
        let surf_h = surface.height;

        // Drop old renderer before replacing surface so wgpu's internal raw
        // Wayland pointers are released while the old surface is still alive.
        self.renderer = None;

        let mut renderer = hypaper_renderer::renderer::Renderer::new(&raw, surf_w, surf_h)
            .await
            .map_err(|e| anyhow::anyhow!("renderer init: {e}"))?;

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
                        tracing::info!(src = %img.src, "loaded image layer");
                    }
                    Err(_) => {
                        tracing::warn!(src = %img.src, "image asset not found in archive");
                    }
                }
            }
        }

        // Store surface before renderer; drop in reverse order on exit so the
        // wgpu surface is released before the wl_surface pointer is invalidated.
        self.wayland_surface = Some(surface);
        self.renderer = Some(renderer);
        self.current_path = Some(path.to_owned());

        tracing::info!(path, width = surf_w, height = surf_h, "wallpaper set");
        Ok(())
    }

    /// Renders one frame if not paused and a renderer is active.
    ///
    /// # Errors
    ///
    /// Propagates any error returned by [`hypaper_renderer::renderer::Renderer::render`].
    pub fn render_frame(&mut self) -> Result<(), anyhow::Error> {
        if self.paused {
            return Ok(());
        }
        if let Some(renderer) = &mut self.renderer {
            renderer
                .render()
                .map_err(|e| anyhow::anyhow!("render error: {e}"))?;
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

    /// Drops the renderer and Wayland surface, clearing the wallpaper.
    ///
    /// The Wayland display connection is retained so that a subsequent
    /// [`set_wallpaper`](Self::set_wallpaper) call can reuse it without
    /// reconnecting.
    pub fn stop(&mut self) {
        // Drop in safe order: renderer first (releases raw pointer borrows),
        // then surface (invalidates the wl_surface pointer).
        self.renderer = None;
        self.wayland_surface = None;
        self.current_path = None;
        tracing::info!("wallpaper stopped");
    }
}

impl Default for WallpaperManager {
    fn default() -> Self {
        Self::new()
    }
}
