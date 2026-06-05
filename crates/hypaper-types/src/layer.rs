//! Layer types: blend modes, fit modes, per-kind configuration, and the top-level `Layer` struct.

use std::collections::HashMap;

/// Available blend modes for compositing a layer onto the framebuffer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BlendMode {
    /// Standard alpha compositing.
    Normal,
    /// Additive blending (source colour added to destination).
    Additive,
}

/// Properties common to every layer kind.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayerBase {
    /// Unique identifier for this layer within the scene.
    pub id: String,
    /// Rendering order; higher values are drawn on top.
    pub z_index: i32,
    /// Whether the layer is currently rendered.
    pub visible: bool,
    /// Layer opacity, clamped to `[0.0, 1.0]`.
    pub opacity: f32,
    /// Compositing blend mode applied when drawing this layer.
    pub blend_mode: BlendMode,
}

/// How a raster asset is scaled to fill the output surface.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FitMode {
    /// Scale uniformly to cover the entire surface (may crop edges).
    Fill,
    /// Scale uniformly to fit inside the surface (may letterbox).
    Fit,
    /// Stretch non-uniformly to fill exactly.
    Stretch,
}

/// A static image layer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageLayer {
    /// Path to the image asset inside the `.hyscene` bundle.
    pub src: String,
    /// How the image is scaled to fit the output surface.
    pub fit_mode: FitMode,
}

/// A video layer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoLayer {
    /// Path to the video asset inside the `.hyscene` bundle.
    pub src: String,
    /// How the video frames are scaled to fit the output surface.
    pub fit_mode: FitMode,
    /// Whether the video loops when it reaches the end.
    #[serde(rename = "loop")]
    pub loop_: bool,
    /// Playback speed multiplier (`1.0` = normal speed).
    pub speed: f32,
    /// Whether the video's audio tracks are silenced.
    pub muted: bool,
}

/// A typed value passed as a uniform to a WGSL/GLSL shader.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum UniformValue {
    /// Single 32-bit float.
    Float(f32),
    /// Two-component float vector.
    Vec2([f32; 2]),
    /// Three-component float vector.
    Vec3([f32; 3]),
    /// Four-component float vector.
    Vec4([f32; 4]),
}

/// A GPU shader layer with configurable uniforms.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShaderLayer {
    /// Path to the shader source file inside the `.hyscene` bundle.
    pub src: String,
    /// Named uniform values passed to the shader each frame.
    pub uniforms: HashMap<String, UniformValue>,
}

/// Where new particles are spawned.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EmitterMode {
    /// Particles spawn along the top edge.
    Top,
    /// Particles spawn along the bottom edge.
    Bottom,
    /// Particles spawn along all four edges.
    Edges,
    /// Particles spawn at a fixed point `(x, y)` in normalised `[0, 1]` coordinates.
    Point(f32, f32),
    /// Particles spawn at random positions across the entire surface.
    Fullscreen,
}

/// A CPU-driven particle system layer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticleLayer {
    /// Path to the particle texture inside the `.hyscene` bundle.
    pub texture: String,
    /// Maximum number of live particles at any time.
    pub count: u32,
    /// Particles emitted per second.
    pub emit_rate: f32,
    /// How long each particle lives, in seconds.
    pub lifetime: f32,
    /// Horizontal velocity applied to newly emitted particles.
    pub velocity_x: f32,
    /// Vertical velocity applied to newly emitted particles.
    pub velocity_y: f32,
    /// Initial size of each particle in pixels.
    pub size: f32,
    /// Initial opacity of each particle, clamped to `[0.0, 1.0]`.
    pub opacity: f32,
    /// Downward acceleration applied to particles each second.
    pub gravity: f32,
    /// Emission origin strategy.
    pub emitter: EmitterMode,
}

/// Discriminated union of all supported layer kinds.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LayerKind {
    /// A static image.
    Image(ImageLayer),
    /// A video clip.
    Video(VideoLayer),
    /// A WGSL/GLSL shader.
    Shader(ShaderLayer),
    /// A particle system.
    Particles(ParticleLayer),
}

/// A single compositing layer in a scene.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Layer {
    /// Properties common to all layer kinds.
    pub base: LayerBase,
    /// Layer-kind-specific configuration.
    pub kind: LayerKind,
}
