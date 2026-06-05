//! Error types for scene parsing and validation.

/// All errors that can occur when loading a `.hyscene` file.
#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    /// A ZIP-level error (corrupt archive, unsupported compression, etc.).
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// An I/O error while reading the archive or its entries.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The `scene.toml` manifest could not be deserialized.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    /// The archive does not contain a `scene.toml` entry at its root.
    #[error("Missing scene.toml in archive")]
    MissingManifest,

    /// The manifest was parsed successfully but failed semantic validation.
    #[error("Invalid scene: {0}")]
    Validation(String),
}
