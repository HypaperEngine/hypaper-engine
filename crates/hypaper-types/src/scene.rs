//! Scene-level types: metadata, rendering configuration, and the top-level `Scene` struct.

use crate::audio::AudioConfig;
use crate::hyprland::HyprlandConfig;
use crate::layer::Layer;

/// Metadata stored in the `.hyscene` manifest.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneMeta {
    /// Human-readable name of the scene.
    pub name: String,
    /// Author of the scene.
    pub author: String,
    /// Scene version string (e.g. `"1.0.0"`).
    pub version: String,
    /// Minimum engine version required to run this scene.
    pub engine_version: String,
}

/// Rendering configuration for a scene.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneConfig {
    /// Target frame rate in frames per second.
    pub fps: u32,
    /// Output resolution as `[width, height]` in pixels.
    pub resolution: [u32; 2],
}

/// Top-level scene descriptor loaded from a `.hyscene` bundle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scene {
    /// Scene metadata (name, author, versions).
    pub meta: SceneMeta,
    /// Rendering configuration (fps, resolution).
    pub config: SceneConfig,
    /// Ordered list of compositing layers (bottom-to-top by `z_index`).
    pub layers: Vec<Layer>,
    /// Optional background audio track.
    pub audio: Option<AudioConfig>,
    /// Optional Hyprland-specific integration settings.
    pub hyprland: Option<HyprlandConfig>,
}
