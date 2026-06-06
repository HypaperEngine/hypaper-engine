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
