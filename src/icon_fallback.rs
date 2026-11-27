//! Keeps the selected icon theme while falling back to Adwaita and hicolor.
//!
//! GTK only resolves one theme name plus hicolor. Themes such as Tela do not
//! carry every symbolic name the settings app uses, so the app installs a
//! wrapper theme whose `Inherits` line names the active theme first, then
//! Adwaita and hicolor. The wrapper has no icons of its own.

use std::path::{Path, PathBuf};

pub const FALLBACK_THEME: &str = "ojuny-icon-fallback";

pub fn inherits_chain(active: &str) -> Option<String> {
    let active = active.trim();
    if active.is_empty() {
        return Some("Adwaita,hicolor".to_string());
    }
    if active == FALLBACK_THEME {
        return None;
    }
    Some(format!("{active},Adwaita,hicolor"))
}

pub fn install(display: &gtk::gdk::Display) {
    let Some(theme_dir) = theme_dir() else {
        return;
    };
    install_in(display, &theme_dir);
}

fn install_in(display: &gtk::gdk::Display, theme_dir: &Path) {
    let settings = gtk::Settings::default()
        .unwrap_or_else(|| gtk::Settings::for_display(display));
    let active = settings
        .gtk_icon_theme_name()
        .map(|name| name.to_string())
        .unwrap_or_default();
    let Some(inherits) = inherits_chain(&active) else {
        return;
    };
    if write_index(theme_dir, &inherits).is_err() {
        return;
    }
    let icon_theme = gtk::IconTheme::for_display(display);
    if let Some(parent) = theme_dir.parent() {
        icon_theme.add_search_path(parent);
    }
    settings.set_gtk_icon_theme_name(Some(FALLBACK_THEME));
}

fn theme_dir() -> Option<PathBuf> {
    let dir = gtk::glib::user_cache_dir()
        .join("nix-settings-gui")
        .join("icons")
        .join(FALLBACK_THEME);
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn write_index(dir: &Path, inherits: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("16x16/apps"))?;
    std::fs::write(
        dir.join("index.theme"),
        format!(
            "[Icon Theme]\nName=Ojuny icon fallback\nComment=Active theme with Adwaita and hicolor fallbacks\nInherits={inherits}\nDirectories=16x16/apps\n\n[16x16/apps]\nSize=16\nType=Fixed\nContext=Applications\n"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_theme_comes_first_then_adwaita_and_hicolor() {
        assert_eq!(
            inherits_chain("Tela").as_deref(),
            Some("Tela,Adwaita,hicolor")
        );
    }

    #[test]
    fn an_empty_theme_still_gets_the_standard_fallback() {
        assert_eq!(
            inherits_chain("  ").as_deref(),
            Some("Adwaita,hicolor")
        );
    }

    #[test]
    fn the_wrapper_is_not_wrapped_again() {
        assert_eq!(inherits_chain(FALLBACK_THEME), None);
    }

    #[test]
    #[ignore = "needs a display; run with `cargo test -- --ignored`"]
    fn wrapper_finds_symbolic_icons_missing_from_the_selected_theme() {
        if gtk::init().is_err() {
            eprintln!("skipping icon fallback check: no display");
            return;
        }
        let Some(display) = gtk::gdk::Display::default() else {
            return;
        };
        let root = std::env::temp_dir().join(format!(
            "nsg-icon-fallback-{}",
            std::process::id()
        ));
        let bare = root.join("Bare");
        std::fs::create_dir_all(bare.join("16x16/apps")).unwrap();
        std::fs::write(
            bare.join("index.theme"),
            "[Icon Theme]\nName=Bare\nDirectories=16x16/apps\n\n[16x16/apps]\nSize=16\nType=Fixed\nContext=Applications\n",
        )
        .unwrap();
        let adwaita = root.join("Adwaita");
        std::fs::create_dir_all(adwaita.join("16x16/apps")).unwrap();
        std::fs::write(
            adwaita.join("index.theme"),
            "[Icon Theme]\nName=Adwaita\nDirectories=16x16/apps\n\n[16x16/apps]\nSize=16\nType=Fixed\nContext=Applications\n",
        )
        .unwrap();
        std::fs::write(
            adwaita.join("16x16/apps/ojuny-test-symbolic.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\"><rect width=\"16\" height=\"16\"/></svg>",
        )
        .unwrap();
        let fallback = root.join(FALLBACK_THEME);
        std::fs::create_dir_all(&fallback).unwrap();

        let theme = gtk::IconTheme::new();
        theme.set_display(Some(&display));
        theme.add_search_path(&root);
        theme.set_theme_name(Some("Bare"));
        assert!(!theme.has_icon("ojuny-test-symbolic"));

        let settings = gtk::Settings::default()
            .unwrap_or_else(|| gtk::Settings::for_display(&display));
        let original = settings.gtk_icon_theme_name();
        settings.set_gtk_icon_theme_name(Some("Bare"));
        install_in(&display, &fallback);
        assert_eq!(
            settings.gtk_icon_theme_name().as_deref(),
            Some(FALLBACK_THEME)
        );

        let wrapped = gtk::IconTheme::new();
        wrapped.set_display(Some(&display));
        wrapped.add_search_path(&root);
        wrapped.set_theme_name(Some(FALLBACK_THEME));
        assert!(wrapped.has_icon("ojuny-test-symbolic"));

        if let Some(original) = original {
            settings.set_gtk_icon_theme_name(Some(original.as_str()));
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
