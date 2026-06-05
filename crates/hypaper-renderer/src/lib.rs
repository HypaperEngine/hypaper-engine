//! GPU rendering backend for Hypaper Engine.
//!
//! Responsible for driving the render loop via `wgpu`, uploading scene assets
//! to the GPU, and producing frames that the Wayland layer surface can display.

#![warn(missing_docs)]

pub mod context;
pub mod error;
pub mod pipeline;
pub mod texture;

pub use context::{create_context, RenderContext};
pub use error::RendererError;
