//! Error types for GPU initialisation and asset loading.

/// All errors that can occur in the renderer.
#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    /// No suitable wgpu adapter was found on this system.
    #[error("Failed to get wgpu adapter")]
    NoAdapter,

    /// The wgpu logical device could not be created.
    #[error("Failed to create wgpu device: {0}")]
    DeviceCreation(String),

    /// A texture could not be decoded or uploaded to the GPU.
    #[error("Texture error: {0}")]
    Texture(String),

    /// A wgpu surface could not be created from the provided Wayland handles.
    #[error("Surface error: {0}")]
    Surface(String),

    /// A per-frame rendering error (surface texture acquisition, encoding, etc.).
    #[error("Render error: {0}")]
    Render(String),

    /// An I/O error while reading an asset from disk.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
