//! Storage and mount model for the `mounts` widget.
//!
//! The model is deliberately GTK-free so the scanner, serializer, path guard
//! and device classifier can be unit-tested without a display.

pub mod detect;
pub mod fs;
pub mod paths;
pub mod row;

pub use row::{
    scan_file_systems, serialize_file_systems, ExtraField, Identity, MountRow,
};
