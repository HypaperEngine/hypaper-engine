//! Sandboxed Rhai engine with per-callback scene-mutation collection.

use crate::{
    api::{register_api, SceneApi},
    error::ScriptError,
};

/// A sandboxed Rhai scripting engine with scene-mutation callbacks.
///
/// Scripts define any subset of the optional callbacks `on_init`,
/// `on_workspace_change`, `on_fullscreen`, and `on_tick`. Each `call_*`
/// method resets the internal [`SceneApi`] accumulator, invokes the
/// corresponding Rhai function (silently skipping it when absent), and
/// returns the mutations collected during that invocation.
///
/// Resource limits (max operations, string/array/map sizes) and the removal
/// of `eval` are applied at construction time to prevent runaway or
/// malicious scripts.
pub struct ScriptEngine {
    engine: rhai::Engine,
    ast: Option<rhai::AST>,
    api: std::sync::Arc<std::sync::Mutex<SceneApi>>,
}

impl ScriptEngine {
    /// Creates a new sandboxed engine with resource limits and the Hypaper
    /// scene API pre-registered.
    pub fn new() -> Self {
        let api = std::sync::Arc::new(std::sync::Mutex::new(SceneApi::default()));
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(100_000);
        engine.set_max_string_size(1024);
        engine.set_max_array_size(1000);
        engine.set_max_map_size(1000);
        engine.disable_symbol("eval");
        register_api(&mut engine, api.clone());
        Self {
            engine,
            ast: None,
            api,
        }
    }

    /// Compiles `source` as a Rhai script, replacing any previously loaded one.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptError::Compile`] if the source contains syntax errors.
    pub fn load_script(&mut self, source: &str) -> Result<(), ScriptError> {
        let ast = self
            .engine
            .compile(source)
            .map_err(|e| ScriptError::Compile(e.to_string()))?;
        self.ast = Some(ast);
        Ok(())
    }

    /// Calls `on_init()` in the loaded script if it is defined.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptError::Runtime`] on execution errors.
    pub fn call_on_init(&mut self) -> Result<SceneApi, ScriptError> {
        self.reset_api()?;
        self.call_fn_if_exists("on_init", ())?;
        self.take_api()
    }

    /// Calls `on_workspace_change(from, to)` if defined.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptError::Runtime`] on execution errors.
    pub fn call_on_workspace_change(
        &mut self,
        from: i64,
        to: i64,
    ) -> Result<SceneApi, ScriptError> {
        self.reset_api()?;
        self.call_fn_if_exists("on_workspace_change", (from, to))?;
        self.take_api()
    }

    /// Calls `on_fullscreen(active)` if defined.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptError::Runtime`] on execution errors.
    pub fn call_on_fullscreen(&mut self, active: bool) -> Result<SceneApi, ScriptError> {
        self.reset_api()?;
        self.call_fn_if_exists("on_fullscreen", (active,))?;
        self.take_api()
    }

    /// Calls `on_tick(delta)` with the elapsed seconds since the last tick.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptError::Runtime`] on execution errors.
    pub fn call_on_tick(&mut self, delta: f64) -> Result<SceneApi, ScriptError> {
        self.reset_api()?;
        self.call_fn_if_exists("on_tick", (delta,))?;
        self.take_api()
    }

    /// Resets the shared [`SceneApi`] accumulator to an empty state.
    fn reset_api(&self) -> Result<(), ScriptError> {
        let mut guard = self
            .api
            .lock()
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;
        *guard = SceneApi::default();
        Ok(())
    }

    /// Atomically swaps the [`SceneApi`] accumulator with a fresh default and
    /// returns the mutations collected during the last callback.
    fn take_api(&self) -> Result<SceneApi, ScriptError> {
        let mut guard = self
            .api
            .lock()
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;
        Ok(std::mem::take(&mut *guard))
    }

    /// Calls `name` in the loaded AST if it exists; silently skips missing functions.
    fn call_fn_if_exists<A: rhai::FuncArgs>(&self, name: &str, args: A) -> Result<(), ScriptError> {
        let ast = match self.ast.as_ref() {
            Some(a) => a,
            None => return Ok(()),
        };
        let mut scope = rhai::Scope::new();
        match self.engine.call_fn::<()>(&mut scope, ast, name, args) {
            Ok(_) => Ok(()),
            Err(e) => {
                if matches!(*e, rhai::EvalAltResult::ErrorFunctionNotFound(..)) {
                    Ok(())
                } else {
                    Err(ScriptError::Runtime(e.to_string()))
                }
            }
        }
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_script_empty_source() {
        // Arrange
        let mut engine = ScriptEngine::new();

        // Act
        let result = engine.load_script("");

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_script_invalid_syntax_returns_compile_error() {
        // Arrange
        let mut engine = ScriptEngine::new();

        // Act
        let result = engine.load_script("fn {{{");

        // Assert
        assert!(matches!(result, Err(ScriptError::Compile(_))));
    }

    #[test]
    fn test_call_on_init_without_function_returns_empty_api() {
        // Arrange
        let mut engine = ScriptEngine::new();
        engine.load_script("").unwrap();

        // Act
        let result = engine.call_on_init();

        // Assert
        let api = result.expect("call_on_init should succeed when on_init is absent");
        assert!(api.layer_opacity.is_empty());
        assert!(api.layer_visible.is_empty());
        assert!(api.uniforms.is_empty());
        assert!(api.audio_volume.is_none());
        assert!(api.audio_muted.is_none());
    }

    #[test]
    fn test_call_on_init_set_layer_opacity() {
        // Arrange
        let mut engine = ScriptEngine::new();
        engine
            .load_script(r#"fn on_init() { set_layer_opacity("bg", 0.5); }"#)
            .unwrap();

        // Act
        let api = engine
            .call_on_init()
            .expect("on_init should run without error");

        // Assert
        assert_eq!(api.layer_opacity.len(), 1);
        let (id, opacity) = &api.layer_opacity[0];
        assert_eq!(id, "bg");
        assert!((opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_call_on_workspace_change_layer_visible() {
        // Arrange
        let mut engine = ScriptEngine::new();
        engine
            .load_script(
                r#"fn on_workspace_change(from, to) { set_layer_visible("particles", to == 2); }"#,
            )
            .unwrap();

        // Act — switch to workspace 2
        let api_to_2 = engine
            .call_on_workspace_change(1, 2)
            .expect("callback should succeed");

        // Assert — particles visible on workspace 2
        assert_eq!(api_to_2.layer_visible.len(), 1);
        let (id, visible) = &api_to_2.layer_visible[0];
        assert_eq!(id, "particles");
        assert!(*visible);

        // Act — switch away from workspace 2
        let api_from_2 = engine
            .call_on_workspace_change(2, 1)
            .expect("callback should succeed");

        // Assert — particles hidden on other workspaces
        assert_eq!(api_from_2.layer_visible.len(), 1);
        let (id, visible) = &api_from_2.layer_visible[0];
        assert_eq!(id, "particles");
        assert!(!*visible);
    }

    #[test]
    fn test_call_on_fullscreen_sets_audio_muted() {
        // Arrange
        let mut engine = ScriptEngine::new();
        engine
            .load_script(r#"fn on_fullscreen(active) { set_audio_muted(active); }"#)
            .unwrap();

        // Act
        let api = engine
            .call_on_fullscreen(true)
            .expect("on_fullscreen should succeed");

        // Assert
        assert_eq!(api.audio_muted, Some(true));
    }

    #[test]
    fn test_sandboxing_infinite_loop_returns_runtime_error() {
        // Arrange
        let mut engine = ScriptEngine::new();
        engine.load_script("fn on_init() { loop {} }").unwrap();

        // Act
        let result = engine.call_on_init();

        // Assert
        assert!(
            matches!(result, Err(ScriptError::Runtime(_))),
            "infinite loop should be terminated by the operations limit"
        );
    }

    #[test]
    fn test_call_on_tick_with_delta() {
        // Arrange
        let mut engine = ScriptEngine::new();
        engine.load_script("fn on_tick(delta) { }").unwrap();

        // Act
        let result = engine.call_on_tick(0.016);

        // Assert
        assert!(result.is_ok());
    }
}
