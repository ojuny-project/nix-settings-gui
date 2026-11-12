use std::cmp::Ordering;
use std::rc::Rc;

use gtk::prelude::*;

use crate::model::MenuModel;

use super::controls::{build_context, build_control_row, control_cmp, evaluate_dependencies};
use super::dialogs::{open_add_option, open_snippet_dialog};
use super::save::{rescan, rescan_after_pull};
use super::state::{App, ControlRow, InsertButton, MenuPage, Tx};

// ---------------------------------------------------------------------------
// scan and view
// ---------------------------------------------------------------------------

pub(super) fn open_flake(app: &Rc<App>, tx: &Tx, path: &str) {
    *app.base.borrow_mut() = path.to_string();
    app.plugin_registry.reset();
    app.input_checks.borrow_mut().clear();
    app.input_checking.set(false);
    app.input_check_cancel.borrow_mut().take();
    app.input_check_epoch.set(app.input_check_epoch.get() + 1);
    app.invalidate_wallpaper();
    app.config.borrow_mut().remember(path);
    let _ = app.config.borrow().save();
    crate::nixd::warm(Some(path));
    let auto_sync = app.config.borrow().auto_git_sync;
    if auto_sync {
        rescan_after_pull(app, tx);
    } else {
        rescan(app, tx);
    }
}

pub(super) fn rebuild_view(app: &Rc<App>) {
    app.pages_ready.set(false);
    choose_current_page(app);
    rebuild_menu_list(app);
    rebuild_pages(app);
    app.pages_ready.set(true);
    let current = app.current_file.borrow().clone();
    if !current.is_empty() {
        select_file(app, &current);
    }
}

fn choose_current_page(app: &Rc<App>) {
    let menus = sorted_menus(app);
    let current = app.current_file.borrow().clone();
    let pattern = app.page_pattern.borrow().clone();
    let resolve_pattern = app.page_pending.get() && app.scan.borrow().is_some();
    if resolve_pattern {
        app.page_pending.set(false);
        if let Some(pattern) = pattern.as_ref() {
            if pattern_matches_inputs(pattern) {
                app.showing_inputs.set(true);
                app.current_file.borrow_mut().clear();
            } else if let Some(file) = select_menu_file(&menus, pattern) {
                app.showing_inputs.set(false);
                *app.current_file.borrow_mut() = file;
            } else if !page_is_open(app) {
                app.showing_inputs.set(true);
                app.current_file.borrow_mut().clear();
            }
            return;
        }
    }
    if app.showing_inputs.get() {
        return;
    }
    if !menus.iter().any(|menu| menu.file == current) {
        if let Some(first) = menus.first() {
            *app.current_file.borrow_mut() = first.file.clone();
        }
    }
}

fn page_is_open(app: &Rc<App>) -> bool {
    app.showing_inputs.get() || !app.current_file.borrow().is_empty()
}

fn pattern_matches_inputs(pattern: &regex::Regex) -> bool {
    pattern.is_match(super::inputs::INPUTS_LABEL)
}

/// Navigate to the page selected by a second launch's `--page`, or leave the
/// current page alone when the pattern matches neither a visible menu nor the
/// built-in Flake Inputs page.
pub(super) fn apply_page_pattern(app: &Rc<App>, pattern: &regex::Regex) {
    *app.page_pattern.borrow_mut() = Some(pattern.clone());
    if app.scan.borrow().is_none() {
        app.page_pending.set(true);
        app.window.present();
        return;
    }
    app.page_pending.set(false);
    let menus = sorted_menus(app);
    if pattern_matches_inputs(pattern) {
        show_inputs_page(app);
    } else if let Some(file) = select_menu_file(&menus, pattern) {
        app.showing_inputs.set(false);
        app.inputs_list.unselect_all();
        ensure_page(app, &file);
        app.show_page(&file);
        *app.current_file.borrow_mut() = file.clone();
        select_file(app, &file);
        super::controls::apply_control_filter(app);
        super::enrich::query_current_page(app, &app.tx);
    } else if !page_is_open(app) {
        show_inputs_page(app);
    }
    app.window.present();
}

fn show_inputs_page(app: &Rc<App>) {
    app.showing_inputs.set(true);
    app.current_file.borrow_mut().clear();
    app.list.unselect_all();
    super::inputs::refresh(app);
    app.show_page(super::inputs::INPUTS_PAGE);
}

fn visible_menus(app: &Rc<App>) -> Vec<crate::model::MenuModel> {
    let scan = app.scan.borrow();
    let Some(scan) = scan.as_ref() else {
        return Vec::new();
    };
    let host = app.current_host.borrow().clone();
    let menus: Vec<&crate::model::MenuModel> = if app.system_only.get() {
        scan.menus_for(&host)
    } else {
        scan.menus.iter().collect()
    };
    menus
        .into_iter()
        .filter(|menu| menu.has_menu)
        .filter(|menu| app.advanced.get() || !menu.spec.hidden)
        .cloned()
        .collect()
}

fn menu_cmp(a: &MenuModel, b: &MenuModel) -> Ordering {
    let ac = a.spec.category.as_deref().unwrap_or_default();
    let bc = b.spec.category.as_deref().unwrap_or_default();
    ac.cmp(bc)
        .then_with(|| a.order().cmp(&b.order()))
        .then_with(|| a.display_name().cmp(&b.display_name()))
}

fn sorted_menus(app: &Rc<App>) -> Vec<MenuModel> {
    let mut menus = visible_menus(app);
    menus.sort_by(menu_cmp);
    menus
}

fn select_menu_file(menus: &[MenuModel], pattern: &regex::Regex) -> Option<String> {
    menus
        .iter()
        .find(|menu| pattern.is_match(&menu.display_name()) || pattern.is_match(&menu.file))
        .map(|menu| menu.file.clone())
}

fn menu_matches(menu: &crate::model::MenuModel, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    if menu.display_name().to_lowercase().contains(query)
        || menu.file.to_lowercase().contains(query)
    {
        return true;
    }
    menu.records
        .iter()
        .any(|record| super::controls::record_matches_query(record, query))
}

fn resolve_menu_icon(icon: Option<&str>) -> Option<String> {
    let display = gtk::gdk::Display::default()?;
    let theme = gtk::IconTheme::for_display(&display);
    crate::widgets::menu_icon::pick_icon_name(|name| theme.has_icon(name), icon)
}

pub(super) fn rebuild_menu_list(app: &Rc<App>) {
    while let Some(child) = app.list.first_child() {
        app.list.remove(&child);
    }
    let query = app.search.text().to_lowercase();
    let menus = sorted_menus(app);
    let mut last_category: Option<String> = None;
    for menu in &menus {
        if !menu_matches(menu, &query) {
            continue;
        }
        let category = menu.spec.category.clone().unwrap_or_default();
        if last_category.as_deref() != Some(category.as_str()) {
            last_category = Some(category.clone());
            if !category.is_empty() {
                let header = gtk::Label::builder()
                    .label(&category)
                    .xalign(0.0)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .margin_top(6)
                    .css_classes(["group-header"])
                    .build();
                let row = gtk::ListBoxRow::builder()
                    .activatable(false)
                    .selectable(false)
                    .child(&header)
                    .build();
                app.list.append(&row);
            }
        }
        let name = menu.display_name();
        let label = gtk::Label::builder()
            .label(&name)
            .xalign(0.0)
            .hexpand(true)
            .visible(!app.sidebar_icons_only.get())
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(&menu.file);
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        if let Some(icon) = resolve_menu_icon(menu.spec.icon.as_deref()) {
            content.append(&gtk::Image::from_icon_name(icon.as_str()));
        }
        content.append(&label);
        row.set_child(Some(&content));
        app.list.append(&row);
    }

    let current = app.current_file.borrow().clone();
    select_file(app, &current);
}

fn select_file(app: &Rc<App>, file: &str) {
    if app.showing_inputs.get() {
        return;
    }
    let mut child = app.list.first_child();
    let mut first_selectable: Option<gtk::ListBoxRow> = None;
    while let Some(widget) = child {
        if let Some(row) = widget.downcast_ref::<gtk::ListBoxRow>() {
            if !row.widget_name().is_empty() && first_selectable.is_none() {
                first_selectable = Some(row.clone());
            }
            if row.widget_name() == file {
                app.list.select_row(Some(row));
                return;
            }
        }
        child = widget.next_sibling();
    }
    if let Some(row) = first_selectable {
        app.list.select_row(Some(&row));
    }
}

fn rebuild_pages(app: &Rc<App>) {
    let started = crate::timing::start();
    let old_pages = app.pages.take();
    drop(old_pages);

    let menus = sorted_menus(app);
    let mut pages = Vec::new();
    if !app.showing_inputs.get() {
        let current = app.current_file.borrow().clone();
        if let Some(menu) = menus.iter().find(|menu| menu.file == current) {
            pages.push(build_page(app, menu));
            let file = menu.file.clone();
            app.pages.replace(pages);
            app.inputs_list.unselect_all();
            app.show_page(&file);
            *app.current_file.borrow_mut() = file.clone();
            select_file(app, &file);
        } else {
            app.pages.replace(pages);
        }
    } else {
        app.pages.replace(pages);
    }

    super::controls::apply_control_filter(app);
    crate::timing::report("rebuild_pages", started);
    super::enrich::query_current_page(app, &app.tx);
}

/// Build the page for `file` when the sidebar selects it for the first time.
pub(super) fn ensure_page(app: &Rc<App>, file: &str) {
    if app.pages.borrow().iter().any(|page| page.file == file) {
        return;
    }
    let menu = sorted_menus(app).into_iter().find(|menu| menu.file == file);
    let Some(menu) = menu else {
        return;
    };
    let page = build_page(app, &menu);
    app.pages.borrow_mut().push(page);
}

#[cfg(feature = "ojuny-widgets")]
fn dashboard_card_option(path: &str) -> bool {
    let Some(position) = path.find("dashOptions.") else {
        return false;
    };
    let rest = &path[position + "dashOptions.".len()..];
    let mut parts = rest.split('.');
    let (Some(card), Some(key), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    key != "widgets" && crate::widgets::is_dashboard_card_id(card)
}

#[cfg(not(feature = "ojuny-widgets"))]
fn dashboard_card_option(_path: &str) -> bool {
    false
}

fn build_page(app: &Rc<App>, menu: &crate::model::MenuModel) -> MenuPage {
    let started = crate::timing::start();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_hexpand(true);

    let heading = gtk::Label::builder()
        .label(menu.display_name())
        .xalign(0.0)
        .wrap(true)
        .css_classes(["title-2"])
        .build();
    content.append(&heading);
    if let Some(description) = &menu.spec.description {
        let subtitle = gtk::Label::builder()
            .label(description)
            .xalign(0.0)
            .hexpand(true)
            .halign(gtk::Align::Fill)
            .wrap(true)
            .css_classes(["dim-label"])
            .build();
        content.append(&subtitle);
    }

    let records: Rc<std::collections::HashMap<String, crate::model::Record>> = Rc::new(
        menu.records
            .iter()
            .map(|record| {
                (
                    crate::model::normalize_namespace(&record.path()),
                    record.clone(),
                )
            })
            .collect(),
    );
    let mut base_ctx = build_context(app);
    base_ctx.records = records.clone();
    let mut ordinary = Vec::new();
    let mut advanced = Vec::new();
    let ranks = crate::model::group_ranks(&menu.records);
    for (src_index, record) in menu.records.iter().enumerate() {
        if record.spec.hidden && !app.advanced.get() {
            continue;
        }
        if dashboard_card_option(&record.path()) {
            continue;
        }
        let row = build_control_row(app, record, src_index, base_ctx.clone());
        if record.spec.advanced {
            advanced.push(row);
        } else {
            ordinary.push(row);
        }
    }

    ordinary.sort_by(|a, b| control_cmp(a, b, &ranks));
    advanced.sort_by(|a, b| control_cmp(a, b, &ranks));

    let mut inserts = build_insert_buttons(app, menu);
    let mut headers = Vec::new();
    let mut group_inserts: Vec<(String, gtk::Button)> = Vec::new();
    add_grouped(
        &content,
        &ordinary,
        &mut inserts,
        false,
        &mut headers,
        &mut group_inserts,
    );
    if !advanced.is_empty() {
        let expander = gtk::Expander::new(Some("Advanced"));
        let inner = gtk::Box::new(gtk::Orientation::Vertical, 12);
        inner.set_margin_top(6);
        let mut no_inserts = Vec::new();
        add_grouped(
            &inner,
            &advanced,
            &mut no_inserts,
            true,
            &mut headers,
            &mut group_inserts,
        );
        expander.set_child(Some(&inner));
        content.append(&expander);
    }

    inserts.sort_by(|a, b| a.group.cmp(&b.group).then_with(|| a.order.cmp(&b.order)));
    for insert in inserts {
        content.append(&insert.button);
    }

    let add = gtk::Button::with_label("Add option");
    add.add_css_class("pill");
    add.set_halign(gtk::Align::Start);
    {
        let app = app.clone();
        let file = menu.file.clone();
        let origin = menu
            .records
            .first()
            .map(|r| r.origin.clone())
            .unwrap_or_default();
        add.connect_clicked(move |_| {
            open_add_option(&app, &file, &origin);
        });
    }
    content.append(&add);

    content.append(&gtk::Box::new(gtk::Orientation::Vertical, 0));

    let scrolled = gtk::ScrolledWindow::builder()
        .child(&content)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_width(false)
        .vexpand(true)
        .hexpand(true)
        .build();

    let mut controls: Vec<Rc<ControlRow>> = ordinary;
    controls.extend(advanced);
    evaluate_dependencies(&controls);

    crate::timing::report(&format!("build_page {}", menu.display_name()), started);
    MenuPage {
        file: menu.file.clone(),
        controls,
        root: scrolled.upcast(),
        content,
        group_headers: headers,
        group_inserts,
    }
}

fn add_grouped(
    content: &gtk::Box,
    controls: &[Rc<ControlRow>],
    inserts: &mut Vec<InsertButton>,
    advanced: bool,
    headers: &mut Vec<(bool, String, gtk::Label)>,
    group_inserts: &mut Vec<(String, gtk::Button)>,
) {
    let mut last_group: Option<String> = None;
    for control in controls {
        let group = control.record.spec.group.clone().unwrap_or_default();
        if last_group.as_deref() != Some(group.as_str()) {
            if let Some(previous) = last_group.take() {
                flush_inserts(content, inserts, &previous, group_inserts);
            }
            if !group.is_empty() {
                let label = gtk::Label::builder()
                    .label(&group)
                    .xalign(0.0)
                    .margin_top(6)
                    .css_classes(["group-header"])
                    .build();
                content.append(&label);
                headers.push((advanced, group.clone(), label.clone()));
            }
            last_group = Some(group);
        }
        content.append(&control.row);
    }
    if let Some(previous) = last_group.take() {
        flush_inserts(content, inserts, &previous, group_inserts);
    }
}

/// Append the insert buttons assigned to `group`, in `order`, then drop them so
/// they are not placed again.
fn flush_inserts(
    content: &gtk::Box,
    inserts: &mut Vec<InsertButton>,
    group: &str,
    group_inserts: &mut Vec<(String, gtk::Button)>,
) {
    inserts.sort_by_key(|insert| insert.order);
    let mut index = 0;
    while index < inserts.len() {
        if inserts[index].group == group {
            let insert = inserts.remove(index);
            group_inserts.push((group.to_string(), insert.button.clone()));
            content.append(&insert.button);
        } else {
            index += 1;
        }
    }
}

/// Drop a leading `Add`/`add` from a snippet label so the button prefix does
/// not double it when the label already reads "Add ...".
pub(super) fn strip_add_prefix(label: &str) -> String {
    let trimmed = label.trim();
    let lower = trimmed.to_lowercase();
    if lower == "add" {
        String::new()
    } else if lower.starts_with("add ") {
        trimmed[4..].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Build the "Add <snippet>" button for every `@insert` marker, resolving its
/// target group from the marker, or from the record directly above it.
fn build_insert_buttons(app: &Rc<App>, menu: &MenuModel) -> Vec<InsertButton> {
    let mut out = Vec::new();
    for marker in &menu.inserts {
        let Some(snippet) = menu.snippets.iter().find(|s| s.name == marker.name) else {
            continue;
        };
        let label = marker
            .label
            .clone()
            .or_else(|| (!snippet.label.is_empty()).then(|| snippet.label.clone()))
            .unwrap_or_else(|| marker.name.clone());
        let label = strip_add_prefix(&label);
        let text = if label.is_empty() {
            "Add".to_string()
        } else {
            format!("Add {label}")
        };
        let button = gtk::Button::with_label(&text);
        button.add_css_class("pill");
        button.set_halign(gtk::Align::Start);
        {
            let app = app.clone();
            let file = menu.file.clone();
            let marker = marker.clone();
            let snippet = snippet.clone();
            button.connect_clicked(move |_| {
                open_snippet_dialog(&app, &file, &marker, &snippet);
            });
        }
        let group = marker
            .group
            .clone()
            .unwrap_or_else(|| preceding_group(menu, marker.offset));
        out.push(InsertButton {
            group,
            order: marker.order.unwrap_or(1000),
            button,
        });
    }
    out
}

/// The group of the record directly above `offset` in the file, or empty when
/// nothing precedes the marker.
fn preceding_group(menu: &MenuModel, offset: usize) -> String {
    menu.records
        .iter()
        .filter(|record| record.byte_start < offset)
        .max_by_key(|record| record.byte_start)
        .map(|record| record.spec.group.clone().unwrap_or_default())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::MenuSpec;
    use crate::widgets::menu_icon::pick_icon_name;

    fn menu(category: &str, order: i64, name: &str, file: &str) -> MenuModel {
        MenuModel {
            file: file.to_string(),
            spec: MenuSpec {
                category: Some(category.to_string()),
                order: Some(order),
                displayname: Some(name.to_string()),
                ..MenuSpec::default()
            },
            ..MenuModel::default()
        }
    }

    #[test]
    fn sorts_menus_by_category_then_order_then_name() {
        let mut menus = vec![
            menu("02-Core", 10, "Bar", "bar.nix"),
            menu("01-Users", 20, "User", "user.nix"),
            menu("01-Users", 10, "System", "system.nix"),
        ];
        menus.sort_by(menu_cmp);
        let names: Vec<_> = menus.iter().map(MenuModel::display_name).collect();
        assert_eq!(names, ["System", "User", "Bar"]);
    }

    #[test]
    fn selects_first_regex_match_in_sorted_order() {
        let menus = vec![
            menu("01-Users", 10, "User", "user.nix"),
            menu("02-Core", 10, "Bar", "bar.nix"),
            menu("02-Core", 20, "Dock", "dock.nix"),
        ];
        let pattern = regex::Regex::new("bar|dock").unwrap();
        assert_eq!(
            select_menu_file(&menus, &pattern).as_deref(),
            Some("bar.nix")
        );
    }

    #[test]
    fn returns_none_when_no_visible_page_matches() {
        let menus = vec![
            menu("01-Users", 10, "User", "user.nix"),
            menu("02-Core", 10, "Bar", "bar.nix"),
        ];
        let pattern = regex::Regex::new("missing").unwrap();
        assert_eq!(select_menu_file(&menus, &pattern), None);
    }

    #[test]
    fn matches_the_builtin_inputs_page_by_label() {
        assert!(pattern_matches_inputs(
            &regex::Regex::new("^Flake Inputs$").unwrap()
        ));
        assert!(!pattern_matches_inputs(
            &regex::Regex::new("^Patches$").unwrap()
        ));
    }

    #[test]
    fn missing_menu_icon_falls_back_to_generic() {
        let only_fallback = |name: &str| name == "preferences-system-symbolic";
        assert_eq!(
            pick_icon_name(only_fallback, Some("preferences-system-displays-symbolic")).as_deref(),
            Some("preferences-system-symbolic")
        );
        let all_present = |_: &str| true;
        assert_eq!(
            pick_icon_name(all_present, Some("web-browser-symbolic")).as_deref(),
            Some("web-browser-symbolic")
        );
        assert_eq!(pick_icon_name(only_fallback, None), None);
    }
}
