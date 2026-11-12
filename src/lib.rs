//! Reusable engine for comment-driven Nix settings applications.
//!
//! This library exposes the scanner, model, metadata, flake input, and edit
//! layers. The GTK application shell stays in the binary target.

// GTK 4.10 deprecated the dialog and style-context APIs this shell still uses.
// The binary target already allows them; keep the library target consistent so
// the deprecation noise does not hide real warnings.
#![allow(deprecated)]

pub mod apply;
pub mod descriptors;
pub mod dsl;
pub mod edit;
pub mod error_dialog;
pub mod flake_inputs;
pub mod flake_lock;
pub mod flake_ref;
pub mod flake_update;
pub mod metadata;
pub mod model;
pub mod monitor_detect;
pub mod nixd;
pub mod nixpkgs;
#[cfg(feature = "ojuny-widgets")]
pub mod plugin_list;
#[cfg(feature = "ojuny-widgets")]
pub mod plugin_schema;
pub mod ref_suggest;
pub mod scan;
pub mod secureboot;
pub mod templates;
pub mod timing;
pub mod value;
pub mod widgets;
