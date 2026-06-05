//! Shared types and data structures used across all Hypaper Engine crates.
//!
//! This crate has zero external dependencies beyond `serde` for serialization,
//! making it the stable foundation the rest of the workspace builds on.

#![warn(missing_docs)]

pub mod audio;
pub mod hyprland;
pub mod ipc;
pub mod layer;
pub mod scene;
