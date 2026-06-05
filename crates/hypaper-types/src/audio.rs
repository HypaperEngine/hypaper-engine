//! Audio configuration for a scene's background track and reactivity settings.

/// Configuration for a scene's audio track.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioConfig {
    /// Path to the audio asset inside the `.hyscene` bundle.
    pub src: String,
    /// Playback volume, clamped to `[0.0, 1.0]`.
    pub volume: f32,
    /// Whether the track loops when it reaches the end.
    #[serde(rename = "loop")]
    pub loop_: bool,
    /// Duration of the fade-in effect in seconds.
    pub fade_in: f32,
    /// Duration of the fade-out effect in seconds.
    pub fade_out: f32,
    /// Pause playback when the wallpaper is paused.
    pub pause_with_wallpaper: bool,
    /// Whether audio-reactive features are enabled.
    pub reactive: bool,
    /// Name of the audio capture source (e.g. a PipeWire sink name).
    pub reactive_source: String,
    /// Signal level `[0.0, 1.0]` that triggers reactivity.
    pub reactive_threshold: f32,
    /// Gain multiplier applied to the reactive signal.
    pub reactive_gain: f32,
}
