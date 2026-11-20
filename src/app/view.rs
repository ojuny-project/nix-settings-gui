//! The category-grid view: navigation levels, the tile grid, and the page
//! host that the grid shares with the menu-list view.

use std::rc::Rc;

use gtk::prelude::*;

use crate::config::ViewMode;
use crate::model::MenuModel;

use super::controls::apply_control_filter;
use super::enrich::query_current_page;
use super::inputs;
use super::menu::{category_groups, menu_matches, resolve_menu_icon, sorted_menus};
use super::state::App;

pub(super) const CONFIGURE_TARGET: &str = "__nix_settings_gui";
pub(super) const CONFIGURE_LABEL: &str = "Nix Settings GUI";

/// Where the Back button returns from a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GridPlace {
    Home,
    Category(String),
    AllPages,
}

/// One visible level of the category view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GridLevel {
    Home,
    Category(String),
    AllPages,
    Page { target: String, back: GridPlace },
}

impl GridLevel {
    /// The level Back returns to.
    pub(super) fn back(&self) -> Self {
        match self {
            Self::Page { back, .. } => match back {
                GridPlace::Home => Self::Home,
                GridPlace::Category(name) => Self::Category(name.clone()),
                GridPlace::AllPages => Self::AllPages,
            },
            _ => Self::Home,
        }
    }

    /// Key for the remembered tile selection at a grid level.
    fn place_key(&self) -> Option<String> {
        match self {
            Self::Home => Some("home".to_string()),
            Self::Category(name) => Some(format!("category:{name}")),
            Self::AllPages => Some("all".to_string()),
            Self::Page { .. } => None,
        }
    }
}

/// Apply the startup view: list keeps the paned layout, categories moves the
/// page host into the grid body and opens Home.
pub(super) fn init_view(app: &Rc<App>) {
    match app.view_mode.get() {
        ViewMode::List => {
            app.paned.set_visible(true);
            app.grid_view.set_visible(false);
        }
        ViewMode::Categories => {
            app.paned.set_end_child(None::<&gtk::Widget>);
            app.paned.set_visible(false);
            app.grid_body.append(&app.page_area);
            app.page_area.set_visible(false);
            app.grid_view.set_visible(true);
            *app.grid_level.borrow_mut() = GridLevel::Home;
            rebuild_grid(app);
        }
    }
}

/// Switch the running window between the two layouts. Used by a second launch
/// whose `--view` differs from the current mode.
pub(super) fn set_view_mode(app: &Rc<App>, mode: ViewMode) {
    if app.view_mode.get() == mode {
        return;
    }
    app.view_mode.set(mode);
    match mode {
        ViewMode::List => {
            if let Some(parent) = app.page_area.parent() {
                if let Some(parent) = parent.downcast_ref::<gtk::Box>() {
                    parent.remove(&app.page_area);
                }
            }
            app.paned.set_end_child(Some(&app.page_area));
            app.page_area.set_visible(true);
            app.paned.set_visible(true);
            app.grid_view.set_visible(false);
            let target = open_target(app);
            if let Some(target) = target {
                show_page(app, &target);
            }
        }
        ViewMode::Categories => {
            app.paned.set_end_child(None::<&gtk::Widget>);
            app.paned.set_visible(false);
            app.grid_body.append(&app.page_area);
            app.grid_view.set_visible(true);
            match open_target(app) {
                Some(target) => {
                    *app.grid_level.borrow_mut() = GridLevel::Page {
                        target,
                        back: GridPlace::Home,
                    };
                }
                None => *app.grid_level.borrow_mut() = GridLevel::Home,
            }
            rebuild_grid(app);
        }
    }
}

fn open_target(app: &Rc<App>) -> Option<String> {
    if app.showing_inputs.get() {
        return Some(inputs::INPUTS_PAGE.to_string());
    }
    let current = app.current_file.borrow().clone();
    if current.is_empty() {
        None
    } else {
        Some(current)
    }
}

/// Re-show the current page, or step back to Home when a rescan or filter
/// removed the category or page that was open.
pub(super) fn refresh_after_scan(app: &Rc<App>) {
    if app.view_mode.get() != ViewMode::Categories {
        return;
    }
    let valid = match &*app.grid_level.borrow() {
        GridLevel::Page { target, .. } => page_exists(app, target),
        GridLevel::Category(name) => category_groups(app)
            .iter()
            .any(|(category, _)| category == name),
        _ => true,
    };
    if !valid {
        *app.grid_level.borrow_mut() = GridLevel::Home;
    }
    rebuild_grid(app);
}

/// Rebuild the grid after the search text changes. Pages keep their scroll and
/// selection, because the control filter already handles a page search.
pub(super) fn refresh_search(app: &Rc<App>) {
    if app.view_mode.get() != ViewMode::Categories {
        return;
    }
    if matches!(*app.grid_level.borrow(), GridLevel::Page { .. }) {
        return;
    }
    let search_focused = app.search.has_focus();
    rebuild_grid(app);
    if search_focused {
        app.search.grab_focus();
    }
}

/// Make a page requested by `--page` the visible level.
pub(super) fn focus_page(app: &Rc<App>, target: &str) {
    if app.view_mode.get() != ViewMode::Categories {
        return;
    }
    set_page_level(app, target);
    rebuild_grid(app);
}

/// Record a page level without rendering it. Used while the scan that resolves
/// `--page` is still rebuilding the pages.
pub(super) fn set_page_level(app: &Rc<App>, target: &str) {
    if app.view_mode.get() != ViewMode::Categories {
        return;
    }
    *app.grid_level.borrow_mut() = GridLevel::Page {
        target: target.to_string(),
        back: GridPlace::Home,
    };
}

pub(super) fn back(app: &Rc<App>) {
    let next = app.grid_level.borrow().back();
    set_level(app, next);
}

pub(super) fn home(app: &Rc<App>) {
    set_level(app, GridLevel::Home);
}

pub(super) fn all_pages(app: &Rc<App>) {
    set_level(app, GridLevel::AllPages);
}

fn set_level(app: &Rc<App>, level: GridLevel) {
    *app.grid_level.borrow_mut() = level;
    rebuild_grid(app);
}

fn page_exists(app: &Rc<App>, target: &str) -> bool {
    if target == inputs::INPUTS_PAGE || target == CONFIGURE_TARGET {
        return true;
    }
    sorted_menus(app).iter().any(|menu| menu.file == target)
}

/// Attach a page to the shared page host. The host lives in the paned in list
/// view and in the grid body in category view.
pub(super) fn show_page(app: &Rc<App>, name: &str) {
    while let Some(child) = app.stack.first_child() {
        app.stack.remove(&child);
    }
    if name == inputs::INPUTS_PAGE {
        app.stack.append(&app.inputs_page);
    } else if name == CONFIGURE_TARGET {
        let page = super::configure::build_page(app);
        app.stack.append(&page);
    } else if let Some(page) = app.pages.borrow().iter().find(|page| page.file == name) {
        app.stack.append(&page.root);
    }
}

pub(super) fn rebuild_grid(app: &Rc<App>) {
    let level = app.grid_level.borrow().clone();
    update_chrome(app, &level);
    match level {
        GridLevel::Page { target, .. } => {
            app.grid_scroller.set_visible(false);
            app.grid_empty.set_visible(false);
            app.page_area.set_visible(true);
            if target == inputs::INPUTS_PAGE {
                inputs::refresh(app);
            }
            show_page(app, &target);
        }
        _ => {
            app.page_area.set_visible(false);
            app.grid_scroller.set_visible(true);
            fill_tiles(app, &level);
        }
    }
}

fn update_chrome(app: &Rc<App>, level: &GridLevel) {
    let title = match level {
        GridLevel::Home => "Categories".to_string(),
        GridLevel::Category(name) => name.clone(),
        GridLevel::AllPages => "All pages".to_string(),
        GridLevel::Page { target, .. } => page_title(app, target),
    };
    app.grid_title.set_text(&title);
    let at_home = matches!(level, GridLevel::Home);
    app.grid_back.set_sensitive(!at_home);
    app.grid_top.set_sensitive(!at_home);
    app.grid_all
        .set_sensitive(!matches!(level, GridLevel::AllPages));
}

fn page_title(app: &Rc<App>, target: &str) -> String {
    if target == inputs::INPUTS_PAGE {
        return inputs::INPUTS_LABEL.to_string();
    }
    if target == CONFIGURE_TARGET {
        return CONFIGURE_LABEL.to_string();
    }
    sorted_menus(app)
        .iter()
        .find(|menu| menu.file == target)
        .map(MenuModel::display_name)
        .unwrap_or_default()
}

fn fill_tiles(app: &Rc<App>, level: &GridLevel) {
    while let Some(child) = app.grid_flow.first_child() {
        app.grid_flow.remove(&child);
    }
    let query = app.search.text().to_lowercase();
    let mut count = 0;

    match level {
        GridLevel::Home => {
            for (category, menus) in category_groups(app) {
                if !menus.iter().any(|menu| menu_matches(menu, &query)) {
                    continue;
                }
                let icon = category_icon(&category, &menus);
                let key = format!("category:{category}");
                app.grid_flow
                    .append(&tile(icon.as_deref(), &category, &key, None));
                count += 1;
            }
            count += builtin_tiles(app, &query);
        }
        GridLevel::Category(name) => {
            let groups = category_groups(app);
            if let Some((_, menus)) = groups.iter().find(|(category, _)| category == name) {
                for menu in menus.iter().filter(|menu| menu_matches(menu, &query)) {
                    app.grid_flow.append(&page_tile(menu));
                    count += 1;
                }
            }
        }
        GridLevel::AllPages => {
            for menu in sorted_menus(app)
                .iter()
                .filter(|menu| menu_matches(menu, &query))
            {
                app.grid_flow.append(&page_tile(menu));
                count += 1;
            }
            count += builtin_tiles(app, &query);
        }
        GridLevel::Page { .. } => {}
    }

    app.grid_empty.set_visible(count == 0);
    restore_selection(app, level);
}

fn builtin_tiles(app: &Rc<App>, query: &str) -> usize {
    let mut count = 0;
    if label_matches(inputs::INPUTS_LABEL, query) {
        app.grid_flow.append(&tile(
            inputs::flake_icon_name().as_deref(),
            inputs::INPUTS_LABEL,
            "inputs",
            Some("Edit the flake inputs and the nixpkgs source"),
        ));
        count += 1;
    }
    if label_matches(CONFIGURE_LABEL, query) {
        app.grid_flow.append(&tile(
            configure_icon().as_deref(),
            CONFIGURE_LABEL,
            "configure",
            Some("Paths, rebuild commands and runtime preferences"),
        ));
        count += 1;
    }
    count
}

fn label_matches(label: &str, query: &str) -> bool {
    query.is_empty() || label.to_lowercase().contains(query)
}

fn page_tile(menu: &MenuModel) -> gtk::FlowBoxChild {
    let icon = resolve_menu_icon(menu.spec.icon.as_deref());
    let key = format!("page:{}", menu.file);
    tile(
        icon.as_deref(),
        &menu.display_name(),
        &key,
        menu.spec.description.as_deref(),
    )
}

fn tile(icon: Option<&str>, label: &str, key: &str, tooltip: Option<&str>) -> gtk::FlowBoxChild {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    if let Some(icon) = icon {
        let image = gtk::Image::from_icon_name(icon);
        image.set_pixel_size(48);
        content.append(&image);
    }
    let text = gtk::Label::builder()
        .label(label)
        .wrap(true)
        .justify(gtk::Justification::Center)
        .xalign(0.5)
        .max_width_chars(20)
        .build();
    content.append(&text);

    let child = gtk::FlowBoxChild::new();
    child.set_child(Some(&content));
    child.set_widget_name(key);
    child.set_hexpand(false);
    child.set_vexpand(false);
    child.set_size_request(150, 110);
    child.add_css_class("settings-tile");
    if let Some(tooltip) = tooltip {
        child.set_tooltip_text(Some(tooltip));
    }
    child
}

fn category_icon(category: &str, menus: &[MenuModel]) -> Option<String> {
    let member_icons: Vec<Option<&str>> =
        menus.iter().map(|menu| menu.spec.icon.as_deref()).collect();
    let display = gtk::gdk::Display::default()?;
    let theme = gtk::IconTheme::for_display(&display);
    crate::widgets::category_icon::pick_category_icon_name(
        |name| theme.has_icon(name),
        category,
        &member_icons,
    )
}

fn configure_icon() -> Option<String> {
    let display = gtk::gdk::Display::default()?;
    let theme = gtk::IconTheme::for_display(&display);
    ["preferences-other-symbolic", "preferences-system-symbolic"]
        .into_iter()
        .find(|name| theme.has_icon(name))
        .map(str::to_string)
}

fn restore_selection(app: &Rc<App>, level: &GridLevel) {
    let Some(place) = level.place_key() else {
        return;
    };
    let wanted = app.grid_last.borrow().get(&place).cloned();
    if let Some(wanted) = wanted {
        let mut index = 0;
        while let Some(child) = app.grid_flow.child_at_index(index) {
            if child.widget_name().as_str() == wanted {
                app.grid_flow.select_child(&child);
                child.grab_focus();
                return;
            }
            index += 1;
        }
    }
    if let Some(child) = app.grid_flow.child_at_index(0) {
        app.grid_flow.select_child(&child);
        child.grab_focus();
    }
}

/// Handle Enter or a double click on a tile.
pub(super) fn activate_tile(app: &Rc<App>, key: &str) {
    if !app.pages_ready.get() || key.is_empty() {
        return;
    }
    let back = match &*app.grid_level.borrow() {
        GridLevel::Category(name) => GridPlace::Category(name.clone()),
        GridLevel::AllPages => GridPlace::AllPages,
        _ => GridPlace::Home,
    };
    if let Some(place) = app.grid_level.borrow().place_key() {
        app.grid_last.borrow_mut().insert(place, key.to_string());
    }

    if let Some(name) = key.strip_prefix("category:") {
        set_level(app, GridLevel::Category(name.to_string()));
    } else if let Some(file) = key.strip_prefix("page:") {
        open_page(app, file.to_string(), back);
    } else if key == "inputs" {
        open_page(app, inputs::INPUTS_PAGE.to_string(), back);
    } else if key == "configure" {
        open_page(app, CONFIGURE_TARGET.to_string(), back);
    }
}

fn open_page(app: &Rc<App>, target: String, back: GridPlace) {
    app.page_pending.set(false);
    app.inputs_list.unselect_all();
    let is_inputs = target == inputs::INPUTS_PAGE;
    let is_configure = target == CONFIGURE_TARGET;
    app.showing_inputs.set(is_inputs);
    if is_inputs || is_configure {
        app.current_file.borrow_mut().clear();
    } else {
        super::menu::ensure_page(app, &target);
        *app.current_file.borrow_mut() = target.clone();
    }
    *app.grid_level.borrow_mut() = GridLevel::Page {
        target: target.clone(),
        back,
    };
    rebuild_grid(app);
    apply_control_filter(app);
    if !is_inputs && !is_configure {
        query_current_page(app, &app.tx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(back: GridPlace) -> GridLevel {
        GridLevel::Page {
            target: "page.nix".to_string(),
            back,
        }
    }

    #[test]
    fn back_from_a_page_returns_to_its_grid() {
        assert_eq!(page(GridPlace::Home).back(), GridLevel::Home);
        assert_eq!(
            page(GridPlace::Category("07-System".to_string())).back(),
            GridLevel::Category("07-System".to_string())
        );
        assert_eq!(page(GridPlace::AllPages).back(), GridLevel::AllPages);
    }

    #[test]
    fn back_from_a_grid_returns_home() {
        assert_eq!(GridLevel::Home.back(), GridLevel::Home);
        assert_eq!(
            GridLevel::Category("01-Users".to_string()).back(),
            GridLevel::Home
        );
        assert_eq!(GridLevel::AllPages.back(), GridLevel::Home);
    }

    #[test]
    fn grid_levels_have_a_selection_key() {
        assert_eq!(GridLevel::Home.place_key().as_deref(), Some("home"));
        assert_eq!(GridLevel::AllPages.place_key().as_deref(), Some("all"));
        assert_eq!(
            GridLevel::Category("02-Core".to_string())
                .place_key()
                .as_deref(),
            Some("category:02-Core")
        );
        assert_eq!(page(GridPlace::Home).place_key(), None);
    }
}
