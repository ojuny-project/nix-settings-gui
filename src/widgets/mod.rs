#[cfg(feature = "ojuny-widgets")]
mod bar;
mod basic;
mod context;
mod cursor_theme;
mod details;
mod dispatch;
#[cfg(feature = "ojuny-widgets")]
mod dms_catalog;
#[cfg(feature = "ojuny-widgets")]
mod docker;
mod editor;
mod facter;
pub mod facter_detect;
mod fields;
mod file;
mod font;
#[cfg(feature = "ojuny-widgets")]
mod grid;
#[cfg(feature = "ojuny-widgets")]
mod grid_details;
#[cfg(feature = "ojuny-widgets")]
mod grid_gestures;
#[cfg(feature = "ojuny-widgets")]
mod grid_layout;
mod icon;
mod icon_theme;
mod keybinds;
#[cfg(feature = "ojuny-widgets")]
mod menu;
pub(crate) mod menu_icon;
mod module_list;
mod monitor_geometry;
mod monitor_picker;
mod monitors;
mod names;
pub(crate) mod nix;
mod popout;
mod secureboot;
mod secureboot_wizard;
mod style_catalog;
mod style_choice;
mod submodule;
mod table;
mod theme;
mod theme_catalog;
#[cfg(feature = "ojuny-widgets")]
mod tile;
#[cfg(feature = "ojuny-widgets")]
mod tile_list;
#[cfg(feature = "ojuny-widgets")]
mod tile_nix;
mod users;
mod wallpaper;
mod xcursor;

pub(crate) use basic::{build_entry, build_spin, build_switch};
pub use context::BuildContext;
pub use dispatch::build;
pub(crate) use context::{
    ApplyHook, ChangeHook, FileActions, IconActions, PackageSearch,
    PasswordPromptHook, PluginRegistryHook, PluginSettingsHook, SecureBootExec, SecureBootRebuild,
    SecureBootRun, SecureBootStatus, SecureBootWrite, WallpaperActions, WallpaperSetHook,
};
pub(crate) use details::InfoDetails;
pub use editor::Editor;
pub(crate) use dispatch::is_package;
#[cfg(feature = "ojuny-widgets")]
pub(crate) use grid::is_dashboard_card_id;
pub(crate) use nix::{attr_key, literal_for_widget};
pub(crate) use submodule::{build_bespoke_child, hook_notify, scan_children};
