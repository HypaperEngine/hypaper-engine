//! Parsing and validation of `.hyscene` scene files.
//!
//! A `.hyscene` file is a ZIP archive containing a `manifest.toml` that
//! describes the scene metadata and asset references. This crate handles
//! reading, deserializing, and validating that format.

#![warn(missing_docs)]
