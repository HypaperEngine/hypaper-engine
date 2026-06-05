//! Parsing and validation of `.hyscene` scene files.
//!
//! A `.hyscene` file is a ZIP archive containing a `scene.toml` that
//! describes the scene metadata and asset references. This crate handles
//! reading, deserializing, and validating that format.

#![warn(missing_docs)]

pub mod error;
pub mod manifest;
pub mod parser;

pub use error::SceneError;
pub use parser::parse_hyscene;
