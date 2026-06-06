//! Sandboxed Rhai scripting engine for Hypaper scene automation.
//!
//! Scripts define optional callbacks (`on_init`, `on_workspace_change`,
//! `on_fullscreen`, `on_tick`) and use the registered API to queue
//! scene mutations that the engine applies after each invocation.

pub mod api;
pub mod engine;
pub mod error;

pub use api::SceneApi;
pub use engine::ScriptEngine;
pub use error::ScriptError;
