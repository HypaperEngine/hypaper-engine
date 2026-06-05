//! Raw TOML-deserialisable structs that mirror the `scene.toml` format exactly.
//!
//! These are intermediate representations. Use [`crate::parse_hyscene`] to
//! obtain a validated [`hypaper_types::scene::Scene`] instead.

use std::collections::HashMap;

/// Top-level structure of a `scene.toml` manifest.
#[derive(Debug, serde::Deserialize)]
pub struct RawScene {
    /// `[meta]` section.
    pub meta: RawMeta,
    /// `[config]` section.
    pub config: RawSceneConfig,
    /// `[[layers]]` array.
    pub layers: Vec<RawLayer>,
    /// Optional `[audio]` section.
    pub audio: Option<RawAudio>,
    /// Optional `[hyprland]` section.
    pub hyprland: Option<RawHyprland>,
}

/// Raw `[meta]` section.
#[derive(Debug, serde::Deserialize)]
pub struct RawMeta {
    /// Human-readable scene name.
    pub name: String,
    /// Scene author.
    pub author: String,
    /// Scene version string.
    pub version: String,
    /// Minimum engine version required.
    pub engine_version: String,
}

/// Raw `[config]` section.
#[derive(Debug, serde::Deserialize)]
pub struct RawSceneConfig {
    /// Target frame rate in frames per second.
    pub fps: u32,
    /// Output resolution as `[width, height]`.
    pub resolution: [u32; 2],
}

/// One entry from the `[[layers]]` array.
#[derive(Debug, serde::Deserialize)]
pub struct RawLayer {
    /// Unique layer identifier.
    pub id: String,
    /// Rendering order; higher values drawn on top.
    pub z_index: i32,
    /// Whether the layer is rendered.
    pub visible: bool,
    /// Layer opacity `[0.0, 1.0]`.
    pub opacity: f32,
    /// Blend mode string (`"Normal"` or `"Additive"`).
    pub blend_mode: String,
    /// Kind-specific configuration, externally-tagged by variant name.
    pub config: RawLayerConfig,
}

/// Kind-specific layer configuration.
///
/// Externally tagged by variant name in TOML:
/// ```toml
/// [layers.config.image]
/// src = "bg.png"
/// fit_mode = "Fill"
/// ```
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawLayerConfig {
    /// Static image configuration.
    Image(RawImageConfig),
    /// Video clip configuration.
    Video(RawVideoConfig),
    /// Shader configuration.
    Shader(RawShaderConfig),
    /// Particle system configuration.
    Particles(RawParticlesConfig),
}

/// Raw image layer fields.
#[derive(Debug, serde::Deserialize)]
pub struct RawImageConfig {
    /// Asset path inside the bundle.
    pub src: String,
    /// Scale mode string (`"Fill"`, `"Fit"`, or `"Stretch"`).
    pub fit_mode: String,
}

/// Raw video layer fields.
#[derive(Debug, serde::Deserialize)]
pub struct RawVideoConfig {
    /// Asset path inside the bundle.
    pub src: String,
    /// Scale mode string.
    pub fit_mode: String,
    /// Whether the video loops.
    #[serde(rename = "loop")]
    pub loop_: bool,
    /// Playback speed multiplier.
    pub speed: f32,
    /// Whether the video's audio is silenced.
    pub muted: bool,
}

/// Raw shader layer fields.
#[derive(Debug, serde::Deserialize)]
pub struct RawShaderConfig {
    /// Shader source path inside the bundle.
    pub src: String,
    /// Uniform values; each must be a number or array of 2–4 numbers.
    #[serde(default)]
    pub uniforms: HashMap<String, toml::Value>,
}

/// Raw particle system layer fields.
#[derive(Debug, serde::Deserialize)]
pub struct RawParticlesConfig {
    /// Particle texture path inside the bundle.
    pub texture: String,
    /// Maximum live particle count.
    pub count: u32,
    /// Particles emitted per second.
    pub emit_rate: f32,
    /// Particle lifetime in seconds.
    pub lifetime: f32,
    /// Initial horizontal velocity.
    pub velocity_x: f32,
    /// Initial vertical velocity.
    pub velocity_y: f32,
    /// Initial particle size in pixels.
    pub size: f32,
    /// Per-particle opacity `[0.0, 1.0]`.
    pub opacity: f32,
    /// Downward acceleration per second.
    pub gravity: f32,
    /// Emitter mode string (`"Top"`, `"Bottom"`, `"Edges"`, `"Fullscreen"`, `"Point"`).
    pub emitter: String,
    /// X coordinate for `Point` emitter in normalised `[0, 1]` space.
    pub emitter_x: Option<f32>,
    /// Y coordinate for `Point` emitter in normalised `[0, 1]` space.
    pub emitter_y: Option<f32>,
}

/// Raw `[audio]` section.
#[derive(Debug, serde::Deserialize)]
pub struct RawAudio {
    /// Audio asset path inside the bundle.
    pub src: String,
    /// Playback volume `[0.0, 1.0]`.
    pub volume: f32,
    /// Whether the track loops.
    #[serde(rename = "loop")]
    pub loop_: bool,
    /// Fade-in duration in seconds.
    pub fade_in: f32,
    /// Fade-out duration in seconds.
    pub fade_out: f32,
    /// Pause audio when the wallpaper is paused.
    pub pause_with_wallpaper: bool,
    /// Whether audio-reactive features are active.
    pub reactive: bool,
    /// Name of the audio capture source.
    pub reactive_source: String,
    /// Signal threshold that triggers reactivity.
    pub reactive_threshold: f32,
    /// Gain applied to the reactive signal.
    pub reactive_gain: f32,
}

/// Raw `[hyprland]` section.
#[derive(Debug, serde::Deserialize)]
pub struct RawHyprland {
    /// Pause on fullscreen windows.
    pub pause_on_fullscreen: bool,
    /// Pause on battery power.
    pub pause_on_battery: bool,
    /// Enable cursor parallax effect.
    pub parallax: bool,
    /// Parallax intensity multiplier.
    pub parallax_intensity: f32,
    /// Per-workspace scene overrides; keys are workspace ID strings.
    #[serde(default)]
    pub workspaces: HashMap<String, String>,
    /// Fallback scene for unlisted workspaces.
    pub default_workspace: Option<String>,
}
