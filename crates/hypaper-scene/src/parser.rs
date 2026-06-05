//! Parsing entry point for `.hyscene` archives.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use hypaper_types::audio::AudioConfig;
use hypaper_types::hyprland::HyprlandConfig;
use hypaper_types::layer::{
    BlendMode, EmitterMode, FitMode, ImageLayer, Layer, LayerBase, LayerKind, ParticleLayer,
    ShaderLayer, UniformValue, VideoLayer,
};
use hypaper_types::scene::{Scene, SceneConfig, SceneMeta};

use crate::error::SceneError;
use crate::manifest::{RawAudio, RawHyprland, RawLayer, RawLayerConfig, RawScene, RawSceneConfig};

/// Opens a `.hyscene` ZIP archive at `path` and returns a validated [`Scene`].
///
/// The archive must contain a `scene.toml` manifest at its root.
pub fn parse_hyscene(path: &Path) -> Result<Scene, SceneError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let contents = {
        let mut entry = archive.by_name("scene.toml").map_err(|e| match e {
            zip::result::ZipError::FileNotFound => SceneError::MissingManifest,
            other => SceneError::Zip(other),
        })?;
        let mut buf = String::new();
        entry.read_to_string(&mut buf)?;
        buf
    };

    let raw: RawScene = toml::from_str(&contents)?;
    raw_to_scene(raw)
}

fn raw_to_scene(raw: RawScene) -> Result<Scene, SceneError> {
    let meta = SceneMeta {
        name: raw.meta.name,
        author: raw.meta.author,
        version: raw.meta.version,
        engine_version: raw.meta.engine_version,
    };

    let config = validate_config(raw.config)?;

    let layers = raw
        .layers
        .into_iter()
        .map(convert_layer)
        .collect::<Result<Vec<_>, _>>()?;

    let audio = raw.audio.map(convert_audio);
    let hyprland = raw.hyprland.map(convert_hyprland).transpose()?;

    Ok(Scene {
        meta,
        config,
        layers,
        audio,
        hyprland,
    })
}

fn validate_config(raw: RawSceneConfig) -> Result<SceneConfig, SceneError> {
    if raw.fps == 0 {
        return Err(SceneError::Validation("fps must be greater than 0".into()));
    }
    if raw.resolution[0] == 0 || raw.resolution[1] == 0 {
        return Err(SceneError::Validation(
            "resolution width and height must both be greater than 0".into(),
        ));
    }
    Ok(SceneConfig {
        fps: raw.fps,
        resolution: raw.resolution,
    })
}

fn convert_layer(raw: RawLayer) -> Result<Layer, SceneError> {
    let base = LayerBase {
        id: raw.id,
        z_index: raw.z_index,
        visible: raw.visible,
        opacity: raw.opacity,
        blend_mode: parse_blend_mode(&raw.blend_mode)?,
    };
    Ok(Layer {
        base,
        kind: convert_layer_config(raw.config)?,
    })
}

fn convert_layer_config(config: RawLayerConfig) -> Result<LayerKind, SceneError> {
    match config {
        RawLayerConfig::Image(c) => Ok(LayerKind::Image(ImageLayer {
            src: c.src,
            fit_mode: parse_fit_mode(&c.fit_mode)?,
        })),
        RawLayerConfig::Video(c) => Ok(LayerKind::Video(VideoLayer {
            src: c.src,
            fit_mode: parse_fit_mode(&c.fit_mode)?,
            loop_: c.loop_,
            speed: c.speed,
            muted: c.muted,
        })),
        RawLayerConfig::Shader(c) => {
            let uniforms = c
                .uniforms
                .into_iter()
                .map(|(k, v)| convert_uniform(v).map(|u| (k, u)))
                .collect::<Result<HashMap<_, _>, _>>()?;
            Ok(LayerKind::Shader(ShaderLayer {
                src: c.src,
                uniforms,
            }))
        }
        RawLayerConfig::Particles(c) => Ok(LayerKind::Particles(ParticleLayer {
            texture: c.texture,
            count: c.count,
            emit_rate: c.emit_rate,
            lifetime: c.lifetime,
            velocity_x: c.velocity_x,
            velocity_y: c.velocity_y,
            size: c.size,
            opacity: c.opacity,
            gravity: c.gravity,
            emitter: parse_emitter_mode(&c.emitter, c.emitter_x, c.emitter_y)?,
        })),
    }
}

fn convert_audio(raw: RawAudio) -> AudioConfig {
    AudioConfig {
        src: raw.src,
        volume: raw.volume,
        loop_: raw.loop_,
        fade_in: raw.fade_in,
        fade_out: raw.fade_out,
        pause_with_wallpaper: raw.pause_with_wallpaper,
        reactive: raw.reactive,
        reactive_source: raw.reactive_source,
        reactive_threshold: raw.reactive_threshold,
        reactive_gain: raw.reactive_gain,
    }
}

fn convert_hyprland(raw: RawHyprland) -> Result<HyprlandConfig, SceneError> {
    let workspaces = raw
        .workspaces
        .into_iter()
        .map(|(k, v)| {
            k.parse::<u32>()
                .map(|id| (id, v))
                .map_err(|_| SceneError::Validation(format!("invalid workspace id: {k:?}")))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;

    Ok(HyprlandConfig {
        pause_on_fullscreen: raw.pause_on_fullscreen,
        pause_on_battery: raw.pause_on_battery,
        parallax: raw.parallax,
        parallax_intensity: raw.parallax_intensity,
        workspaces,
        default_workspace: raw.default_workspace,
    })
}

fn parse_blend_mode(s: &str) -> Result<BlendMode, SceneError> {
    match s {
        "Normal" => Ok(BlendMode::Normal),
        "Additive" => Ok(BlendMode::Additive),
        other => Err(SceneError::Validation(format!(
            "unknown blend_mode: {other:?}"
        ))),
    }
}

fn parse_fit_mode(s: &str) -> Result<FitMode, SceneError> {
    match s {
        "Fill" => Ok(FitMode::Fill),
        "Fit" => Ok(FitMode::Fit),
        "Stretch" => Ok(FitMode::Stretch),
        other => Err(SceneError::Validation(format!(
            "unknown fit_mode: {other:?}"
        ))),
    }
}

fn parse_emitter_mode(s: &str, x: Option<f32>, y: Option<f32>) -> Result<EmitterMode, SceneError> {
    match s {
        "Top" => Ok(EmitterMode::Top),
        "Bottom" => Ok(EmitterMode::Bottom),
        "Edges" => Ok(EmitterMode::Edges),
        "Fullscreen" => Ok(EmitterMode::Fullscreen),
        "Point" => {
            let x =
                x.ok_or_else(|| SceneError::Validation("Point emitter requires emitter_x".into()))?;
            let y =
                y.ok_or_else(|| SceneError::Validation("Point emitter requires emitter_y".into()))?;
            Ok(EmitterMode::Point(x, y))
        }
        other => Err(SceneError::Validation(format!(
            "unknown emitter mode: {other:?}"
        ))),
    }
}

fn convert_uniform(value: toml::Value) -> Result<UniformValue, SceneError> {
    match value {
        toml::Value::Float(f) => Ok(UniformValue::Float(f as f32)),
        toml::Value::Integer(i) => Ok(UniformValue::Float(i as f32)),
        toml::Value::Array(arr) => {
            let floats = arr
                .into_iter()
                .map(|v| match v {
                    toml::Value::Float(f) => Ok(f as f32),
                    toml::Value::Integer(i) => Ok(i as f32),
                    _ => Err(SceneError::Validation(
                        "uniform array must contain only numbers".into(),
                    )),
                })
                .collect::<Result<Vec<f32>, _>>()?;
            match floats.as_slice() {
                [x, y] => Ok(UniformValue::Vec2([*x, *y])),
                [x, y, z] => Ok(UniformValue::Vec3([*x, *y, *z])),
                [x, y, z, w] => Ok(UniformValue::Vec4([*x, *y, *z, *w])),
                _ => Err(SceneError::Validation(
                    "uniform array must have 2, 3, or 4 elements".into(),
                )),
            }
        }
        _ => Err(SceneError::Validation(
            "uniform value must be a number or an array of numbers".into(),
        )),
    }
}
