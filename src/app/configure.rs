//! The app's own settings, shared by the Configure dialog and the inline
//! `Nix Settings GUI` page in the category view.

use std::rc::Rc;

use gtk::prelude::*;

use crate::config::ViewMode;

use super::dialogs::{non_empty, select_folder};
use super::menu::open_flake;
use super::state::App;

pub(super) struct ConfigureForm {
    root: gtk::Box,
    flake_entry: gtk::Entry,
    terminal_entry: gtk::Entry,
    rebuild_entry: gtk::Entry,
    reload_entry: gtk::Entry,
    style_picker: gtk::DropDown,
}

impl ConfigureForm {
    pub(super) fn new(app: &Rc<App>) -> Self {
        let grid = gtk::Grid::builder()
            .row_spacing(6)
            .column_spacing(8)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let config = app.config.borrow().clone();
        let flake_entry = gtk::Entry::new();
        flake_entry.set_text(config.flake_path.as_deref().unwrap_or(""));
        flake_entry.set_hexpand(true);
        let flake_browse = gtk::Button::with_label("Browse...");
        {
            let entry = flake_entry.clone();
            let window = app.window.clone();
            flake_browse.connect_clicked(move |_| select_folder(&window, &entry));
        }
        let flake_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        flake_row.append(&flake_entry);
        flake_row.append(&flake_browse);
        let terminal_entry = gtk::Entry::new();
        terminal_entry.set_text(config.terminal_command.as_deref().unwrap_or(""));
        terminal_entry.set_hexpand(true);
        let rebuild_entry = gtk::Entry::new();
        rebuild_entry.set_text(config.rebuild_command.as_deref().unwrap_or(""));
        rebuild_entry.set_hexpand(true);
        let reload_entry = gtk::Entry::new();
        reload_entry.set_text(config.environment_reload_command.as_deref().unwrap_or(""));
        reload_entry.set_hexpand(true);
        reload_entry.set_placeholder_text(Some(
            "Optional command run after a successful rebuild, for example dms kill && mmsg dispatch spawn_shell,dms run",
        ));

        grid.attach(&gtk::Label::new(Some("Flake path")), 0, 0, 1, 1);
        grid.attach(&flake_row, 1, 0, 1, 1);
        grid.attach(&gtk::Label::new(Some("Terminal command")), 0, 1, 1, 1);
        grid.attach(&terminal_entry, 1, 1, 1, 1);
        grid.attach(&gtk::Label::new(Some("Rebuild command")), 0, 2, 1, 1);
        grid.attach(&rebuild_entry, 1, 2, 1, 1);
        grid.attach(&gtk::Label::new(Some("Environment reload command")), 0, 3, 1, 1);
        grid.attach(&reload_entry, 1, 3, 1, 1);

        let style_picker = gtk::DropDown::from_strings(&["List", "Categories"]);
        style_picker.set_selected(style_index(config.view_mode));
        style_picker.set_halign(gtk::Align::Start);
        let style_note = gtk::Label::builder()
            .label("Requires restarting nix-settings-gui to apply.")
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label", "caption"])
            .build();
        let style_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        style_box.append(&style_picker);
        style_box.append(&style_note);
        grid.attach(&gtk::Label::new(Some("GUI style")), 0, 4, 1, 1);
        grid.attach(&style_box, 1, 4, 1, 1);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&grid);
        Self {
            root,
            flake_entry,
            terminal_entry,
            rebuild_entry,
            reload_entry,
            style_picker,
        }
    }

    pub(super) fn root(&self) -> &gtk::Box {
        &self.root
    }

    pub(super) fn apply(&self, app: &Rc<App>) {
        let flake = self.flake_entry.text().to_string();
        {
            let mut config = app.config.borrow_mut();
            config.terminal_command = non_empty(self.terminal_entry.text().to_string());
            config.rebuild_command = non_empty(self.rebuild_entry.text().to_string());
            config.environment_reload_command =
                non_empty(self.reload_entry.text().to_string());
            config.view_mode = style_mode(self.style_picker.selected());
            if !flake.trim().is_empty() {
                config.remember(flake.trim());
            }
        }
        let _ = app.config.borrow().save();
        if !flake.trim().is_empty() {
            let tx = app.tx.clone();
            open_flake(app, &tx, flake.trim());
        }
    }
}

/// Build the inline `Nix Settings GUI` page: the Configure form plus the runtime
/// preference switches that also live in the header popover.
pub(super) fn build_page(app: &Rc<App>) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_hexpand(true);

    let heading = gtk::Label::builder()
        .label("Nix Settings GUI")
        .xalign(0.0)
        .wrap(true)
        .css_classes(["title-2"])
        .build();
    content.append(&heading);
    let subtitle = gtk::Label::builder()
        .label("Paths, rebuild commands and the runtime preferences for this application.")
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    content.append(&subtitle);

    let form = Rc::new(ConfigureForm::new(app));
    content.append(form.root());

    let preferences_heading = gtk::Label::builder()
        .label("Preferences")
        .xalign(0.0)
        .margin_top(6)
        .css_classes(["group-header"])
        .build();
    content.append(&preferences_heading);
    content.append(&preferences(app));

    let save = gtk::Button::with_label("Save");
    save.add_css_class("suggested-action");
    save.set_halign(gtk::Align::Start);
    {
        let app = app.clone();
        save.connect_clicked(move |_| {
            form.apply(&app);
            app.status.set_text("Settings saved");
        });
    }
    content.append(&save);

    let scrolled = gtk::ScrolledWindow::builder()
        .child(&content)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_width(false)
        .vexpand(true)
        .hexpand(true)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&scrolled);
    root
}

fn preferences(app: &Rc<App>) -> gtk::Box {
    let config = app.config.borrow().clone();
    let list = gtk::Box::new(gtk::Orientation::Vertical, 4);

    let live_apply = gtk::Switch::new();
    live_apply.set_active(config.live_apply);
    {
        let app = app.clone();
        live_apply.connect_active_notify(move |switch| {
            app.config.borrow_mut().live_apply = switch.is_active();
            let _ = app.config.borrow().save();
            super::controls::refresh_live_markers(&app);
        });
    }
    list.append(&super::preference_row(
        "Apply changes immediately",
        "Run each option's applyCommand as soon as it changes",
        &live_apply,
    ));

    let expand_details = gtk::Switch::new();
    expand_details.set_active(config.expand_details_by_default);
    {
        let app = app.clone();
        expand_details.connect_active_notify(move |switch| {
            let expanded = switch.is_active();
            app.config.borrow_mut().expand_details_by_default = expanded;
            let _ = app.config.borrow().save();
            super::controls::set_all_details_expanded(&app, expanded);
        });
    }
    list.append(&super::preference_row(
        "Expand setting details",
        "Show setting descriptions and import origins by default",
        &expand_details,
    ));

    let auto_git = gtk::Switch::new();
    auto_git.set_active(config.auto_git_sync);
    {
        let app = app.clone();
        auto_git.connect_active_notify(move |switch| {
            app.config.borrow_mut().auto_git_sync = switch.is_active();
            let _ = app.config.borrow().save();
        });
    }
    list.append(&super::preference_row(
        "Sync git on open and save",
        "Pull on open, then commit and push after saving",
        &auto_git,
    ));

    list
}

fn style_index(mode: ViewMode) -> u32 {
    match mode {
        ViewMode::List => 0,
        ViewMode::Categories => 1,
    }
}

fn style_mode(index: u32) -> ViewMode {
    if index == 1 {
        ViewMode::Categories
    } else {
        ViewMode::List
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_choices_round_trip_through_the_picker_index() {
        assert_eq!(style_index(ViewMode::List), 0);
        assert_eq!(style_index(ViewMode::Categories), 1);
        assert_eq!(style_mode(0), ViewMode::List);
        assert_eq!(style_mode(1), ViewMode::Categories);
        assert_eq!(style_mode(99), ViewMode::List);
    }
}
