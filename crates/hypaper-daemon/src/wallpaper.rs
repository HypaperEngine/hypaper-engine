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
    /// Rhai scripting engine loaded from `scripts/logic.rhai` in the scene bundle.
    script_engine: Option<hypaper_script::ScriptEngine>,
    /// Last known Hyprland workspace id; used as `from` in `on_workspace_change`.
    current_workspace: i64,
}

impl WallpaperManager {
    /// Creates a `WallpaperManager` with no active wallpapers.
    pub fn new() -> Self {
        Self {
            monitors: HashMap::new(),
            current_path: None,
            paused: false,
            wayland_display: None,
            script_engine: None,
            current_workspace: 0,
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

        self.load_scene_script(path);
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

    /// Reads `scripts/logic.rhai` from the `.hyscene` ZIP at `path`, compiles it,
    /// and calls `on_init`.  Clears the engine on any error or if no script is
    /// present so stale state is never carried over.
    fn load_scene_script(&mut self, path: &str) {
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                tracing::debug!("scene not opened for script check: {e}");
                self.script_engine = None;
                return;
            }
        };
        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                tracing::debug!("archive not readable for script check: {e}");
                self.script_engine = None;
                return;
            }
        };

        let source = match archive.by_name("scripts/logic.rhai") {
            Ok(mut entry) => {
                let mut s = String::new();
                match entry.read_to_string(&mut s) {
                    Ok(_) => s,
                    Err(e) => {
                        tracing::warn!("could not read scripts/logic.rhai: {e}");
                        self.script_engine = None;
                        return;
                    }
                }
            }
            Err(_) => {
                // No script in this scene — not an error.
                self.script_engine = None;
                return;
            }
        };

        let mut engine = hypaper_script::ScriptEngine::new();
        if let Err(e) = engine.load_script(&source) {
            tracing::warn!("script compile error: {e}");
            self.script_engine = None;
            return;
        }
        tracing::info!("script loaded: scripts/logic.rhai");
        self.script_engine = Some(engine);

        let init_api = {
            let e = match self.script_engine.as_mut() {
                Some(e) => e,
                None => return,
            };
            match e.call_on_init() {
                Ok(api) => api,
                Err(e) => {
                    tracing::warn!("on_init error: {e}");
                    return;
                }
            }
        };
        self.apply_scene_api(&init_api);
    }

    /// Applies the mutations collected by a script callback to the live scene.
    ///
    /// Per-layer opacity and visibility changes are logged at DEBUG level until
    /// the renderer exposes per-layer mutation APIs.
    fn apply_scene_api(&mut self, api: &hypaper_script::SceneApi) {
        for (id, opacity) in &api.layer_opacity {
            tracing::debug!(layer = %id, opacity, "script: set_layer_opacity");
        }
        for (id, visible) in &api.layer_visible {
            tracing::debug!(layer = %id, visible, "script: set_layer_visible");
        }
        if let Some(vol) = api.audio_volume {
            tracing::debug!(volume = vol, "script: fade_audio");
        }
        if let Some(muted) = api.audio_muted {
            tracing::debug!(muted, "script: set_audio_muted");
        }
    }

    /// Dispatches a Hyprland event to the matching Rhai callback and applies
    /// the resulting [`SceneApi`](hypaper_script::SceneApi) mutations.
    ///
    /// Unhandled event variants are silently ignored.  A missing or unloaded
    /// script engine is also a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if the Rhai callback itself returns a runtime error.
    pub fn on_hyprland_event(
        &mut self,
        event: &hypaper_types::hyprland::HyprlandEvent,
    ) -> Result<(), anyhow::Error> {
        use hypaper_types::hyprland::HyprlandEvent;

        // Extract the value before the mutable engine borrow so the fields
        // remain disjoint from the script_engine borrow inside the block.
        let from = self.current_workspace;

        let api = {
            let engine = match self.script_engine.as_mut() {
                Some(e) => e,
                None => return Ok(()),
            };
            match event {
                HyprlandEvent::WorkspaceChanged { id } => engine
                    .call_on_workspace_change(from, *id as i64)
                    .map_err(|e| anyhow::anyhow!("script on_workspace_change: {e}"))?,
                HyprlandEvent::FullscreenEntered => engine
                    .call_on_fullscreen(true)
                    .map_err(|e| anyhow::anyhow!("script on_fullscreen: {e}"))?,
                HyprlandEvent::FullscreenExited => engine
                    .call_on_fullscreen(false)
                    .map_err(|e| anyhow::anyhow!("script on_fullscreen: {e}"))?,
                _ => return Ok(()),
            }
        };
        // engine borrow ends here

        if let HyprlandEvent::WorkspaceChanged { id } = event {
            self.current_workspace = *id as i64;
        }
        self.apply_scene_api(&api);
        Ok(())
    }
}

impl Default for WallpaperManager {
    fn default() -> Self {
        Self::new()
    }
}
