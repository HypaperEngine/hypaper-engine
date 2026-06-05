//! Wayland integration layer for Hypaper Engine.
//!
//! Manages `wlr-layer-shell` surfaces, handles monitor enumeration, and
//! provides the platform abstraction through which the renderer outputs frames.

#![warn(missing_docs)]

pub mod display;
pub mod error;
pub mod raw_handle;
pub mod surface;

pub use error::WaylandError;
