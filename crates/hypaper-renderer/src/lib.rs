//! GPU rendering backend for Hypaper Engine.
//!
//! Responsible for driving the render loop via `wgpu`, uploading scene assets
//! to the GPU, and producing frames that the Wayland layer surface can display.

#![warn(missing_docs)]

pub mod context;
pub mod error;
pub mod fit;
pub mod pipeline;
pub mod renderer;
pub mod shader_layer;
pub mod texture;
pub mod uniforms;

pub use context::{create_context, create_context_for_surface, RenderContext};
pub use error::RendererError;
pub use fit::{compute_uvs, FitMode};
pub use renderer::Renderer;
pub use shader_layer::ShaderLayerRenderer;
pub use uniforms::{create_uniform_buffer, AutoUniforms};
