//! Scene-mutation API exposed to Rhai scripts.

/// Accumulated mutations produced by a single script callback invocation.
///
/// Each `call_*` method on [`crate::ScriptEngine`] resets this state before
/// executing the Rhai function, then returns the collected mutations so the
/// caller can apply them to the live scene.
#[derive(Debug, Default)]
pub struct SceneApi {
    /// Per-layer opacity overrides queued by the script: `(layer_id, opacity)`.
    pub layer_opacity: Vec<(String, f32)>,
    /// Per-layer visibility overrides queued by the script: `(layer_id, visible)`.
    pub layer_visible: Vec<(String, bool)>,
    /// Shader uniform overrides: `(layer_id, uniform_name, value)`.
    pub uniforms: Vec<(String, String, hypaper_types::layer::UniformValue)>,
    /// Target audio volume requested by the script, if any.
    pub audio_volume: Option<f32>,
    /// Audio mute state requested by the script, if any.
    pub audio_muted: Option<bool>,
}

/// Registers the Hypaper scene-mutation API into `engine`.
///
/// The following functions become available inside Rhai scripts:
///
/// - `set_layer_opacity(id, opacity)` — queue an opacity change for a layer.
/// - `set_layer_visible(id, visible)` — queue a visibility change for a layer.
/// - `set_audio_muted(muted)` — mute or unmute audio.
/// - `fade_audio(volume, duration)` — set a target volume (duration is reserved).
///
/// All mutations are accumulated in `api` and retrieved after the callback
/// returns via [`std::mem::take`].
pub fn register_api(engine: &mut rhai::Engine, api: std::sync::Arc<std::sync::Mutex<SceneApi>>) {
    let a = api.clone();
    engine.register_fn("set_layer_opacity", move |id: &str, opacity: f32| {
        if let Ok(mut guard) = a.lock() {
            guard.layer_opacity.push((id.to_string(), opacity));
        }
    });

    let a = api.clone();
    engine.register_fn("set_layer_visible", move |id: &str, visible: bool| {
        if let Ok(mut guard) = a.lock() {
            guard.layer_visible.push((id.to_string(), visible));
        }
    });

    let a = api.clone();
    engine.register_fn("set_audio_muted", move |muted: bool| {
        if let Ok(mut guard) = a.lock() {
            guard.audio_muted = Some(muted);
        }
    });

    let a = api.clone();
    engine.register_fn("fade_audio", move |volume: f32, _duration: f32| {
        if let Ok(mut guard) = a.lock() {
            guard.audio_volume = Some(volume);
        }
    });
}
