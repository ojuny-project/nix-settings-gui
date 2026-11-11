use super::basic::build_dropdown;
use super::context::{BuildContext, ChangeHook};
use super::dispatch::build;
use super::editor::Editor;
use super::fields::field;
use super::monitor_features::{self, DisabledSet, Feature, FeatureControl};
use super::monitor_geometry::{
    monitor_orig, resolve_release, row_footprint, scene_bounds, Viewport, SNAP_TOLERANCE, VIEW_PAD,
};
use super::monitor_picker::open_monitor_picker;
use super::nix::{attr_key, normalize_path, number_literal, quote};
use crate::model::Record;
use crate::monitor_detect::DetectedMode;
use gtk::glib;
use gtk::prelude::*;
use rnix::ast::{Expr, HasEntry};
use rnix::SyntaxNode;
use rowan::ast::AstNode;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

const TRANSFORMS: &[&str] = &[
    "normal",
    "90",
    "180",
    "270",
    "flipped",
    "flipped-90",
    "flipped-180",
    "flipped-270",
];

const GEOMETRY_PATHS: &[&str] = &[
    "width",
    "height",
    "refresh",
    "xPos",
    "yPos",
    "scale",
    "transform",
];

/// One feature editor below an output. Core geometry keeps its own fields so
/// the canvas can read them without going through the registry.
#[derive(Clone)]
pub(crate) struct FeatureRow {
    pub(crate) path: String,
    pub(crate) editor: Editor,
    pub(crate) orig: String,
    pub(crate) present: bool,
    pub(crate) dirty: Rc<Cell<bool>>,
}

/// One output of `services.nix-settings-gui.monitors`: the attribute name is
/// the stable identity, plus the core geometry and every enabled feature.
pub(crate) struct MonitorRow {
    pub(crate) name: gtk::Entry,
    pub(crate) width: gtk::SpinButton,
    pub(crate) height: gtk::SpinButton,
    pub(crate) refresh: gtk::SpinButton,
    pub(crate) x: gtk::SpinButton,
    pub(crate) y: gtk::SpinButton,
    pub(crate) scale: gtk::SpinButton,
    pub(crate) rr: Editor,
    pub(crate) features: Vec<FeatureRow>,
    pub(crate) disabled: DisabledSet,
    pub(crate) orig: Vec<(String, String)>,
    pub(crate) passthrough: Vec<(String, String)>,
    pub(crate) form_box: gtk::Box,
}

#[derive(Clone, Debug)]
pub(crate) struct OutputScan {
    pub(crate) name: String,
    pub(crate) fields: Vec<(String, String)>,
}

pub(crate) fn build_transform(value_text: &str) -> Editor {
    let choices: Vec<String> = TRANSFORMS
        .iter()
        .map(|value| (*value).to_string())
        .collect();
    build_dropdown(&choices, value_text)
}

fn number_spin(
    value: &str,
    default: f64,
    min: f64,
    max: f64,
    step: f64,
    digits: u32,
) -> gtk::SpinButton {
    let spin = gtk::SpinButton::with_range(min, max, step);
    spin.set_digits(digits);
    spin.set_width_chars(7);
    spin.set_value(value.parse::<f64>().unwrap_or(default));
    spin
}

fn feature_editor(feature: &Feature, source: &str, ctx: &BuildContext) -> Editor {
    if let FeatureControl::NumberDropdown(values) = feature.control {
        let labels: Vec<String> = values.iter().map(|value| value.to_string()).collect();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let combo = gtk::DropDown::from_strings(&refs);
        let current = source.trim();
        let selected = values
            .iter()
            .position(|value| value.to_string() == current)
            .unwrap_or(0);
        combo.set_selected(selected as u32);
        combo.set_halign(gtk::Align::Start);
        return Editor::Dropdown {
            combo,
            choices: labels,
        };
    }
    let spec = monitor_features::spec(feature);
    let record = Record {
        value: source.to_string(),
        spec,
        ..Record::default()
    };
    build(&record, None, ctx)
}

fn feature_source<'a>(fields: &'a [(String, String)], path: &str) -> &'a str {
    field(fields, path).unwrap_or("")
}

fn output_scan(value: &str) -> Result<Vec<OutputScan>, String> {
    let Ok(root) = rnix::Root::parse(value).ok() else {
        return Err("services.nix-settings-gui.monitors is not valid Nix".to_string());
    };
    let Some(expr) = root.expr() else {
        return Err("services.nix-settings-gui.monitors is empty".to_string());
    };
    let Expr::AttrSet(set) = expr else {
        return Err(
            "services.nix-settings-gui.monitors must be a literal attribute set keyed by output name. Do not wrap it in map or another function call."
                .to_string(),
        );
    };
    let mut outputs = Vec::new();
    for binding in set.attrpath_values() {
        let Some(path) = binding.attrpath() else {
            continue;
        };
        let components: Vec<String> = path
            .attrs()
            .filter_map(|attr| crate::scan::attr_component(&attr))
            .collect();
        if components.is_empty() {
            continue;
        }
        let name = components.join(".");
        let Some(value) = binding.value() else {
            continue;
        };
        if !matches!(Expr::cast(value.syntax().clone()), Some(Expr::AttrSet(_))) {
            return Err(format!(
                "output '{name}' must be a literal attribute set. Do not wrap it in a function call."
            ));
        }
        let mut fields = Vec::new();
        flatten_fields(value.syntax(), "", &mut fields);
        outputs.push(OutputScan { name, fields });
    }
    Ok(outputs)
}

fn flatten_fields(node: &SyntaxNode, prefix: &str, fields: &mut Vec<(String, String)>) {
    let Some(Expr::AttrSet(set)) = Expr::cast(node.clone()) else {
        return;
    };
    for binding in set.attrpath_values() {
        let Some(path) = binding.attrpath() else {
            continue;
        };
        let components: Vec<String> = path
            .attrs()
            .filter_map(|attr| crate::scan::attr_component(&attr))
            .collect();
        if components.is_empty() {
            continue;
        }
        let key = components.join(".");
        let full = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        let Some(value) = binding.value() else {
            continue;
        };
        if prefix.is_empty()
            && monitor_features::is_group(&key)
            && matches!(Expr::cast(value.syntax().clone()), Some(Expr::AttrSet(_)))
        {
            flatten_fields(value.syntax(), &key, fields);
        } else {
            fields.push((full, value.syntax().text().to_string()));
        }
    }
}

fn disabled_set(record: &Record) -> Result<DisabledSet, String> {
    let paths = record.spec.disabled_features.clone().unwrap_or_default();
    DisabledSet::from_paths(&paths)
}

fn monitors_error(message: &str, original: &str) -> Editor {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_hexpand(true);
    let heading = gtk::Label::builder()
        .label("Monitors cannot be shown")
        .xalign(0.0)
        .css_classes(["title-5"])
        .build();
    let detail = gtk::Label::builder()
        .label(message)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    root.append(&heading);
    root.append(&detail);
    Editor::Monitors {
        root,
        rows: Rc::new(RefCell::new(Vec::new())),
        on_change: Rc::new(RefCell::new(None)),
        error: Some(message.to_string()),
        original: original.to_string(),
    }
}

pub(crate) fn monitor_row_unchanged(row: &MonitorRow) -> bool {
    let spin_same = |spin: &gtk::SpinButton, path: &str, default: f64| match monitor_orig(row, path)
        .parse::<f64>()
    {
        Ok(value) => (spin.value() - value).abs() < 1e-9,
        Err(_) => (spin.value() - default).abs() < 1e-9,
    };
    let geometry = (row.disabled.contains("width") || spin_same(&row.width, "width", 0.0))
        && (row.disabled.contains("height") || spin_same(&row.height, "height", 0.0))
        && (row.disabled.contains("refresh") || spin_same(&row.refresh, "refresh", 60.0))
        && (row.disabled.contains("xPos") || spin_same(&row.x, "xPos", 0.0))
        && (row.disabled.contains("yPos") || spin_same(&row.y, "yPos", 0.0))
        && (row.disabled.contains("scale") || spin_same(&row.scale, "scale", 1.0))
        && (row.disabled.contains("transform")
            || monitor_orig(row, "transform").is_empty()
            || row.rr.unchanged(monitor_orig(row, "transform")));
    let features = row.features.iter().all(|feature| {
        if row.disabled.contains(&feature.path) {
            return true;
        }
        if feature.present {
            feature.editor.unchanged(&feature.orig)
        } else {
            !feature.dirty.get()
        }
    });
    geometry && features
}

fn monitor_row_nix(row: &MonitorRow) -> String {
    let mut top: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let add_spin = |top: &mut Vec<String>, path: &str, spin: &gtk::SpinButton, digits: u32| {
        if !row.disabled.contains(path) {
            top.push(format!(
                "{} = {};",
                path,
                number_literal(spin.value(), digits)
            ));
        }
    };
    add_spin(&mut top, "width", &row.width, 0);
    add_spin(&mut top, "height", &row.height, 0);
    if !row.disabled.contains("refresh") {
        top.push(format!(
            "refresh = {};",
            refresh_literal(row.refresh.value())
        ));
    }
    add_spin(&mut top, "xPos", &row.x, 0);
    add_spin(&mut top, "yPos", &row.y, 0);
    add_spin(&mut top, "scale", &row.scale, 2);
    if !row.disabled.contains("transform") {
        top.push(format!("transform = {};", row.rr.to_nix()));
    }

    for feature in &row.features {
        if row.disabled.contains(&feature.path) {
            continue;
        }
        if !feature.present && !feature.dirty.get() {
            continue;
        }
        let literal = feature.editor.to_nix();
        match feature.path.split_once('.') {
            Some((group, child)) if monitor_features::is_group(group) => {
                groups.entry(group.to_string()).or_default().push(format!(
                    "{} = {};",
                    attr_key(child),
                    literal
                ));
            }
            _ => top.push(format!("{} = {};", attr_key(&feature.path), literal)),
        }
    }

    for (path, raw) in &row.passthrough {
        match path.split_once('.') {
            Some((group, child)) if monitor_features::is_group(group) => {
                groups.entry(group.to_string()).or_default().push(format!(
                    "{} = {};",
                    attr_key(child),
                    raw
                ));
            }
            _ => top.push(format!("{} = {};", attr_key(path), raw)),
        }
    }

    for group in monitor_features::GROUPS {
        if let Some(fields) = groups.get(group.key) {
            if !fields.is_empty() {
                top.push(format!("{} = {{ {} }};", group.key, fields.join(" ")));
            }
        }
    }

    format!("{{ {} }}", top.join(" "))
}

pub(crate) fn monitors_nix(rows: &[MonitorRow]) -> String {
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in rows {
        let name = row.name.text().trim().to_string();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        entries.push(format!("{} = {};", attr_key(&name), monitor_row_nix(row)));
    }
    format!("{{ {} }}", entries.join(" "))
}

pub(crate) fn refresh_literal(value: f64) -> String {
    if value.fract() == 0.0 {
        return format!("{}", value as i64);
    }
    let mut text = format!("{value:.3}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn feature_literal(row: &MonitorRow, path: &str) -> String {
    row.features
        .iter()
        .find(|feature| feature.path == path)
        .map(|feature| feature.editor.to_nix())
        .unwrap_or_else(|| monitor_features::default_for(path).to_string())
}

fn output_enabled(row: &MonitorRow) -> bool {
    feature_literal(row, "enabled") != "false"
}

fn output_primary(row: &MonitorRow) -> bool {
    feature_literal(row, "primary") == "true"
}

fn attach_cell(grid: &gtk::Grid, row: i32, col: i32, label: &str, widget: &impl IsA<gtk::Widget>) {
    let caption = gtk::Label::builder()
        .xalign(1.0)
        .label(label)
        .css_classes(["dim-label", "caption"])
        .build();
    grid.attach(&caption, col, row, 1, 1);
    grid.attach(widget, col + 1, row, 1, 1);
}

struct MonitorPalette {
    background: gtk::gdk::RGBA,
    monitor: gtk::gdk::RGBA,
    selected: gtk::gdk::RGBA,
    outline: gtk::gdk::RGBA,
    dashed: gtk::gdk::RGBA,
    text: gtk::gdk::RGBA,
    selected_text: gtk::gdk::RGBA,
}

impl MonitorPalette {
    fn from_style(style: &gtk::StyleContext) -> Self {
        let color = |names: &[&str], fallback: (f32, f32, f32, f32)| {
            names
                .iter()
                .find_map(|name| style.lookup_color(name))
                .unwrap_or_else(|| {
                    gtk::gdk::RGBA::new(fallback.0, fallback.1, fallback.2, fallback.3)
                })
        };
        Self {
            background: color(
                &["window_bg_color", "theme_bg_color"],
                (0.08, 0.08, 0.10, 1.0),
            ),
            monitor: color(
                &["card_bg_color", "theme_base_color", "theme_bg_color"],
                (0.42, 0.42, 0.47, 1.0),
            ),
            selected: color(
                &["accent_bg_color", "theme_selected_bg_color"],
                (0.27, 0.45, 0.75, 1.0),
            ),
            outline: color(
                &["window_fg_color", "theme_fg_color", "theme_text_color"],
                (1.0, 1.0, 1.0, 1.0),
            ),
            dashed: color(
                &[
                    "warning_bg_color",
                    "warning_color",
                    "theme_selected_bg_color",
                ],
                (0.95, 0.65, 0.25, 1.0),
            ),
            text: color(
                &["card_fg_color", "window_fg_color", "theme_fg_color"],
                (1.0, 1.0, 1.0, 1.0),
            ),
            selected_text: color(
                &["accent_fg_color", "theme_selected_fg_color"],
                (1.0, 1.0, 1.0, 1.0),
            ),
        }
    }
}

fn set_color(cr: &gtk::cairo::Context, color: &gtk::gdk::RGBA, alpha: f64) {
    cr.set_source_rgba(
        color.red() as f64,
        color.green() as f64,
        color.blue() as f64,
        color.alpha() as f64 * alpha,
    );
}

fn draw_monitor_label(
    cr: &gtk::cairo::Context,
    label: &str,
    px: f64,
    py: f64,
    pw: f64,
    ph: f64,
    color: &gtk::gdk::RGBA,
    alpha: f64,
) {
    if label.is_empty() || pw <= 1.0 || ph <= 1.0 {
        return;
    }
    cr.save().ok();
    cr.rectangle(px, py, pw, ph);
    cr.clip();
    set_color(cr, color, alpha);
    let font_size = 14.0;
    cr.set_font_size(font_size);
    if let Ok(extents) = cr.text_extents(label) {
        let max_width = (pw - 8.0).max(1.0);
        let max_height = (ph - 8.0).max(1.0);
        let scale = (max_width / extents.width().max(1.0))
            .min(max_height / extents.height().max(1.0))
            .min(1.0);
        if scale < 1.0 {
            cr.set_font_size((font_size * scale).max(9.0));
        }
    }
    if let Ok(extents) = cr.text_extents(label) {
        let x = px + (pw - extents.width()) / 2.0 - extents.x_bearing();
        let y = py + (ph - extents.height()) / 2.0 - extents.y_bearing();
        cr.move_to(x, y);
        let _ = cr.show_text(label);
    }
    cr.restore().ok();
}

fn make_output_row(scan: &OutputScan, ctx: &BuildContext, disabled: &DisabledSet) -> MonitorRow {
    let get = |path: &str| feature_source(&scan.fields, path);
    let name = gtk::Entry::new();
    name.set_text(&scan.name);
    name.set_editable(scan.name.is_empty());
    name.set_placeholder_text(Some("Output name"));

    let width = number_spin(get("width"), 0.0, 0.0, 16384.0, 1.0, 0);
    let height = number_spin(get("height"), 0.0, 0.0, 16384.0, 1.0, 0);
    let refresh = number_spin(get("refresh"), 60.0, 0.0, 1000.0, 0.01, 3);
    let x = number_spin(get("xPos"), 0.0, -16384.0, 16384.0, 1.0, 0);
    let y = number_spin(get("yPos"), 0.0, -16384.0, 16384.0, 1.0, 0);
    let scale = number_spin(get("scale"), 1.0, 0.01, 16.0, 0.05, 2);
    let rr = build_transform(get("transform"));

    let mut features = Vec::new();
    let mut passthrough = Vec::new();
    for feature in monitor_features::FEATURES {
        if GEOMETRY_PATHS.contains(&feature.path) {
            continue;
        }
        let raw = get(feature.path);
        let source = if raw.is_empty() { feature.default } else { raw };
        let editor = feature_editor(feature, source, ctx);
        features.push(FeatureRow {
            path: feature.path.to_string(),
            editor,
            orig: raw.to_string(),
            present: !raw.trim().is_empty() && raw.trim() != "\"\"",
            dirty: Rc::new(Cell::new(false)),
        });
    }
    for (path, raw) in &scan.fields {
        let known = monitor_features::find(path).is_some();
        let grouped = path
            .split_once('.')
            .map(|(group, _)| monitor_features::is_group(group))
            .unwrap_or(false);
        if !known && !grouped {
            passthrough.push((path.clone(), raw.clone()));
        }
    }

    let form_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    form_box.set_margin_top(4);
    form_box.set_margin_bottom(4);
    form_box.set_hexpand(true);

    if scan.name.is_empty() {
        let name_grid = gtk::Grid::new();
        name_grid.set_column_spacing(12);
        name_grid.set_row_spacing(6);
        attach_cell(&name_grid, 0, 0, "Name", &name);
        form_box.append(&name_grid);
    } else {
        let heading = gtk::Label::builder()
            .label(&scan.name)
            .xalign(0.0)
            .css_classes(["title-4"])
            .build();
        form_box.append(&heading);
    }

    let geometry_grid = gtk::Grid::new();
    geometry_grid.set_column_spacing(12);
    geometry_grid.set_row_spacing(6);
    let mut geometry_count = 0i32;
    let mut add_geometry = |caption: &str, path: &str, widget: &gtk::Widget, column: i32| {
        if disabled.contains(path) {
            return;
        }
        attach_cell(&geometry_grid, geometry_count / 2, column, caption, widget);
        geometry_count += 1;
    };
    add_geometry("Width", "width", width.upcast_ref(), 0);
    add_geometry("Height", "height", height.upcast_ref(), 2);
    add_geometry("Refresh", "refresh", refresh.upcast_ref(), 0);
    add_geometry("Scale", "scale", scale.upcast_ref(), 2);
    add_geometry("X", "xPos", x.upcast_ref(), 0);
    add_geometry("Y", "yPos", y.upcast_ref(), 2);
    let rr_root = rr.root();
    add_geometry("Transform", "transform", &rr_root, 0);
    if geometry_count > 0 {
        form_box.append(&geometry_grid);
    }

    let top_features: Vec<&FeatureRow> = features
        .iter()
        .filter(|feature| {
            !disabled.contains(&feature.path)
                && monitor_features::find(&feature.path)
                    .map(|feature| feature.group.is_empty())
                    .unwrap_or(false)
        })
        .collect();
    if !top_features.is_empty() {
        let grid = gtk::Grid::new();
        grid.set_column_spacing(12);
        grid.set_row_spacing(6);
        for (index, feature) in top_features.iter().enumerate() {
            let label = monitor_features::find(&feature.path)
                .map(|feature| feature.label)
                .unwrap_or("Option");
            let caption = gtk::Label::builder()
                .xalign(1.0)
                .label(label)
                .css_classes(["dim-label", "caption"])
                .build();
            grid.attach(&caption, 0, index as i32, 1, 1);
            grid.attach(&feature.editor.root(), 1, index as i32, 1, 1);
        }
        form_box.append(&grid);
    }

    for group in monitor_features::GROUPS {
        if !disabled.any_visible(group.key) {
            continue;
        }
        let visible: Vec<&FeatureRow> = features
            .iter()
            .filter(|feature| {
                !disabled.contains(&feature.path)
                    && monitor_features::find(&feature.path)
                        .map(|feature| feature.group == group.key)
                        .unwrap_or(false)
            })
            .collect();
        if visible.is_empty() {
            continue;
        }
        let grid = gtk::Grid::new();
        grid.set_column_spacing(12);
        grid.set_row_spacing(6);
        for (index, feature) in visible.iter().enumerate() {
            let label = monitor_features::find(&feature.path)
                .map(|feature| feature.label)
                .unwrap_or("Option");
            let caption = gtk::Label::builder()
                .xalign(1.0)
                .label(label)
                .css_classes(["dim-label", "caption"])
                .build();
            grid.attach(&caption, 0, index as i32, 1, 1);
            grid.attach(&feature.editor.root(), 1, index as i32, 1, 1);
        }
        let expander = gtk::Expander::new(Some(group.label));
        expander.set_child(Some(&grid));
        expander.set_expanded(false);
        form_box.append(&expander);
    }

    MonitorRow {
        name,
        width,
        height,
        refresh,
        x,
        y,
        scale,
        rr,
        features,
        disabled: disabled.clone(),
        orig: scan.fields.clone(),
        passthrough,
        form_box,
    }
}

#[derive(Clone, Copy)]
pub(crate) enum DragMode {
    None,
    Pan {
        origin_x: f64,
        origin_y: f64,
    },
    Monitor {
        start_px: f64,
        start_py: f64,
        grab_dx: f64,
        grab_dy: f64,
    },
}

pub(crate) struct MonitorCanvas {
    pub(crate) canvas: gtk::DrawingArea,
    pub(crate) view: Rc<Cell<Viewport>>,
    pub(crate) last_size: Rc<Cell<(f64, f64)>>,
}

impl Clone for MonitorCanvas {
    fn clone(&self) -> Self {
        MonitorCanvas {
            canvas: self.canvas.clone(),
            view: self.view.clone(),
            last_size: self.last_size.clone(),
        }
    }
}

impl MonitorCanvas {
    pub(crate) fn refit(&self, rows: &[MonitorRow]) {
        if let Some((min_x, min_y, max_x, max_y)) = scene_bounds(rows) {
            let (w, h) = self.last_size.get();
            self.view
                .set(Viewport::fit(min_x, min_y, max_x, max_y, w, h));
        }
        self.canvas.queue_draw();
    }
    pub(crate) fn redraw(&self) {
        self.canvas.queue_draw();
    }
}

pub(crate) fn attach_monitor_row(
    form: &gtk::Box,
    rows: &Rc<RefCell<Vec<MonitorRow>>>,
    mc: &MonitorCanvas,
    row: MonitorRow,
) -> usize {
    let redraw: Rc<dyn Fn()> = {
        let mc = mc.clone();
        Rc::new(move || mc.redraw())
    };
    connect_monitor_row(&row, &redraw);
    form.append(&row.form_box);
    let index = rows.borrow().len();
    rows.borrow_mut().push(row);
    index
}

pub(crate) fn connect_monitor_row(row: &MonitorRow, update: &Rc<dyn Fn()>) {
    let name_update = update.clone();
    row.name.connect_changed(move |_| name_update());
    row.rr.connect_changed(update);
    for spin in [
        &row.width,
        &row.height,
        &row.refresh,
        &row.x,
        &row.y,
        &row.scale,
    ] {
        let update = update.clone();
        spin.connect_value_changed(move |_| update());
    }
    for feature in &row.features {
        let dirty = feature.dirty.clone();
        let update = update.clone();
        let hook: Rc<dyn Fn()> = Rc::new(move || {
            dirty.set(true);
            update();
        });
        feature.editor.connect_changed(&hook);
    }
}

pub(crate) fn build_monitors(record: &Record) -> Editor {
    let mut ctx = BuildContext::default();
    ctx.base = Path::new(&record.file)
        .parent()
        .map(normalize_path)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();

    let disabled = match disabled_set(record) {
        Ok(disabled) => disabled,
        Err(error) => return monitors_error(&error, &record.value),
    };
    let scans = match output_scan(&record.value) {
        Ok(scans) => scans,
        Err(error) => return monitors_error(&error, &record.value),
    };

    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
    root.set_hexpand(true);
    let on_change: ChangeHook = Rc::new(RefCell::new(None));
    let rows = Rc::new(RefCell::new(Vec::new()));
    let selected = Rc::new(Cell::new(0usize));
    let view = Rc::new(Cell::new(Viewport {
        scale: 1.0,
        origin_x: 0.0,
        origin_y: 0.0,
    }));
    let last_size = Rc::new(Cell::new((640.0f64, 400.0f64)));

    let canvas = gtk::DrawingArea::builder()
        .content_width(1)
        .content_height(400)
        .build();
    canvas.set_hexpand(true);
    canvas.set_size_request(-1, 400);
    canvas.set_draw_func({
        let rows = rows.clone();
        let selected = selected.clone();
        let view = view.clone();
        let last_size = last_size.clone();
        move |area, cr, width, height| {
            let (last_w, last_h) = last_size.get();
            if last_w != width as f64 || last_h != height as f64 {
                last_size.set((width as f64, height as f64));
                let rows_snapshot = rows.borrow();
                if let Some((min_x, min_y, max_x, max_y)) = scene_bounds(&rows_snapshot) {
                    view.set(Viewport::fit(
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                        width as f64,
                        height as f64,
                    ));
                }
            }
            let palette = MonitorPalette::from_style(&area.style_context());
            set_color(cr, &palette.background, 1.0);
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            cr.fill().ok();
            let v = view.get();
            let rows = rows.borrow();
            for (index, row) in rows.iter().enumerate() {
                let (x, y, w, h, label, sized) = row_footprint(row);
                let (px, py, pw, ph) = v.rect_on_screen(x, y, w, h);
                let is_selected = index == selected.get();
                let disabled = !output_enabled(row);
                let primary = output_primary(row);
                let label = if primary { format!("* {label}") } else { label };
                if disabled {
                    if is_selected {
                        set_color(cr, &palette.selected, 0.35);
                    } else {
                        set_color(cr, &palette.monitor, 0.35);
                    }
                } else if is_selected {
                    set_color(cr, &palette.selected, 1.0);
                } else {
                    set_color(cr, &palette.monitor, 1.0);
                }
                cr.rectangle(px, py, pw, ph);
                cr.fill_preserve().ok();
                if sized {
                    set_color(cr, &palette.outline, if disabled { 0.3 } else { 0.6 });
                    cr.set_line_width(1.0);
                } else {
                    set_color(cr, &palette.dashed, if disabled { 0.35 } else { 1.0 });
                    cr.set_line_width(2.0);
                    cr.set_dash(&[4.0, 4.0], 0.0);
                }
                cr.stroke().ok();
                cr.set_dash(&[], 0.0);
                let (text_color, text_alpha) = if disabled {
                    (&palette.text, 0.45)
                } else if is_selected {
                    (&palette.selected_text, 1.0)
                } else {
                    (&palette.text, 1.0)
                };
                draw_monitor_label(cr, &label, px, py, pw, ph, text_color, text_alpha);
            }
        }
    });
    root.append(&canvas);

    let mc = MonitorCanvas {
        canvas: canvas.clone(),
        view: view.clone(),
        last_size: last_size.clone(),
    };

    let last_pointer = Rc::new(Cell::new((320.0f64, 200.0f64)));
    {
        let last_pointer = last_pointer.clone();
        let motion = gtk::EventControllerMotion::new();
        motion.connect_motion(move |_, x, y| last_pointer.set((x, y)));
        canvas.add_controller(motion);
    }
    {
        let canvas = canvas.clone();
        let view = view.clone();
        let last_pointer = last_pointer.clone();
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let redraw = canvas.clone();
        scroll.connect_scroll(move |_, _dx, dy| {
            if dy == 0.0 {
                return glib::Propagation::Proceed;
            }
            let factor = if dy < 0.0 { 1.1 } else { 1.0 / 1.1 };
            let v = view.get();
            let new_scale = (v.scale * factor).clamp(0.02, 8.0);
            let (px, py) = last_pointer.get();
            let cfg_x = v.to_config_x(px);
            let cfg_y = v.to_config_y(py);
            view.set(Viewport {
                scale: new_scale,
                origin_x: cfg_x - (px - VIEW_PAD) / new_scale,
                origin_y: cfg_y - (py - VIEW_PAD) / new_scale,
            });
            redraw.queue_draw();
            glib::Propagation::Stop
        });
        canvas.add_controller(scroll);
    }

    let actions = gtk::FlowBox::new();
    actions.set_selection_mode(gtk::SelectionMode::None);
    actions.set_homogeneous(false);
    actions.set_max_children_per_line(8);
    actions.set_column_spacing(6);
    actions.set_row_spacing(6);
    actions.set_halign(gtk::Align::Start);
    let snap_tolerance = Rc::new(Cell::new(SNAP_TOLERANCE));
    let snap_label = gtk::Label::builder()
        .label("Snap distance")
        .xalign(0.0)
        .css_classes(["dim-label"])
        .build();
    let snap = gtk::SpinButton::with_range(0.0, 200.0, 1.0);
    snap.set_value(SNAP_TOLERANCE);
    snap.set_tooltip_text(Some("Snap distance in configuration pixels"));
    snap.connect_value_changed({
        let snap_tolerance = snap_tolerance.clone();
        move |spin| snap_tolerance.set(spin.value())
    });
    let recenter = gtk::Button::with_label("Re-center view");
    let recenter_monitor = gtk::Button::with_label("Re-center monitor");
    let remove = gtk::Button::with_label("Remove monitor");
    remove.add_css_class("destructive-action");
    let add = gtk::Button::with_label("Add monitor");
    actions.insert(&recenter, -1);
    actions.insert(&recenter_monitor, -1);
    actions.insert(&remove, -1);
    actions.insert(&add, -1);
    actions.insert(&snap_label, -1);
    actions.insert(&snap, -1);
    root.append(&actions);
    {
        let mc_recenter = mc.clone();
        let rows = rows.clone();
        recenter.connect_clicked(move |_| {
            mc_recenter.refit(&rows.borrow());
        });
    }
    {
        let rows = rows.clone();
        let selected = selected.clone();
        let on_change = on_change.clone();
        let mc = mc.clone();
        let snap_tolerance = snap_tolerance.clone();
        recenter_monitor.connect_clicked(move |_| {
            let rows = rows.borrow();
            let Some(row) = rows.get(selected.get()) else {
                return;
            };
            let (_, _, w, h, _, _) = row_footprint(row);
            let others: Vec<(f64, f64, f64, f64)> = rows
                .iter()
                .filter(|other| !std::ptr::eq(*other, row))
                .map(|other| {
                    let (ox, oy, ow, oh, _, _) = row_footprint(other);
                    (ox, oy, ow, oh)
                })
                .collect();
            let (nx, ny) = resolve_release(0.0, 0.0, w, h, &others, snap_tolerance.get());
            row.x.set_value(nx.round());
            row.y.set_value(ny.round());
            drop(rows);
            mc.redraw();
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });
    }

    let form = gtk::Box::new(gtk::Orientation::Vertical, 6);
    form.set_hexpand(true);
    root.append(&form);

    for (index, scan) in scans.iter().enumerate() {
        let row = make_output_row(scan, &ctx, &disabled);
        row.form_box.set_visible(index == 0);
        attach_monitor_row(&form, &rows, &mc, row);
    }

    {
        let click = gtk::GestureClick::new();
        let rows = rows.clone();
        let selected = selected.clone();
        let view = view.clone();
        let mc = mc.clone();
        click.connect_pressed(move |_, _, px, py| {
            let v = view.get();
            let hit = rows.borrow().iter().position(|row| {
                let (x, y, w, h, _, _) = row_footprint(row);
                let (left, top, width, height) = v.rect_on_screen(x, y, w, h);
                px >= left && px <= left + width && py >= top && py <= top + height
            });
            if let Some(index) = hit {
                selected.set(index);
                for (row_index, row) in rows.borrow().iter().enumerate() {
                    row.form_box.set_visible(row_index == index);
                }
                mc.redraw();
            }
        });
        canvas.add_controller(click);
    }

    {
        let drag = gtk::GestureDrag::new();
        let drag_mode = Rc::new(Cell::new(DragMode::None));
        let rows = rows.clone();
        let selected = selected.clone();
        let view = view.clone();
        let mc = mc.clone();
        drag.connect_drag_begin({
            let rows = rows.clone();
            let selected = selected.clone();
            let view = view.clone();
            let drag_mode = drag_mode.clone();
            let mc = mc.clone();
            move |_, x, y| {
                let v = view.get();
                let rows = rows.borrow();
                let hit = rows.iter().position(|row| {
                    let (rx, ry, rw, rh, _, _) = row_footprint(row);
                    let (left, top, width, height) = v.rect_on_screen(rx, ry, rw, rh);
                    x >= left && x <= left + width && y >= top && y <= top + height
                });
                drag_mode.set(match hit {
                    Some(index) => {
                        selected.set(index);
                        for (i, r) in rows.iter().enumerate() {
                            r.form_box.set_visible(i == index);
                        }
                        let row = &rows[index];
                        let cfg_x = v.to_config_x(x);
                        let cfg_y = v.to_config_y(y);
                        DragMode::Monitor {
                            start_px: x,
                            start_py: y,
                            grab_dx: cfg_x - row.x.value(),
                            grab_dy: cfg_y - row.y.value(),
                        }
                    }
                    None => DragMode::Pan {
                        origin_x: v.origin_x,
                        origin_y: v.origin_y,
                    },
                });
                mc.redraw();
            }
        });
        drag.connect_drag_update({
            let rows = rows.clone();
            let selected = selected.clone();
            let view = view.clone();
            let on_change = on_change.clone();
            let mc = mc.clone();
            let drag_mode = drag_mode.clone();
            move |gesture, _, _| {
                let (offset_x, offset_y) = gesture.offset().unwrap_or((0.0, 0.0));
                match drag_mode.get() {
                    DragMode::Monitor {
                        start_px,
                        start_py,
                        grab_dx,
                        grab_dy,
                    } => {
                        let v = view.get();
                        let px = start_px + offset_x;
                        let py = start_py + offset_y;
                        let new_x = (v.to_config_x(px) - grab_dx).round();
                        let new_y = (v.to_config_y(py) - grab_dy).round();
                        let rows = rows.borrow();
                        let Some(row) = rows.get(selected.get()) else {
                            return;
                        };
                        row.x.set_value(new_x);
                        row.y.set_value(new_y);
                        drop(rows);
                        mc.redraw();
                        if let Some(cb) = on_change.borrow().as_ref() {
                            cb();
                        }
                    }
                    DragMode::Pan { origin_x, origin_y } => {
                        let v = view.get();
                        view.set(Viewport {
                            scale: v.scale,
                            origin_x: origin_x - offset_x / v.scale,
                            origin_y: origin_y - offset_y / v.scale,
                        });
                        mc.redraw();
                    }
                    DragMode::None => {}
                }
            }
        });
        drag.connect_drag_end({
            let rows = rows.clone();
            let selected = selected.clone();
            let on_change = on_change.clone();
            let mc = mc.clone();
            let snap_tolerance = snap_tolerance.clone();
            move |_, _, _| {
                if let DragMode::Monitor { .. } = drag_mode.get() {
                    let rows = rows.borrow();
                    let Some(row) = rows.get(selected.get()) else {
                        return;
                    };
                    let (x, y, w, h, _, _) = row_footprint(row);
                    let others: Vec<(f64, f64, f64, f64)> = rows
                        .iter()
                        .filter(|other| !std::ptr::eq(*other, row))
                        .map(|other| {
                            let (ox, oy, ow, oh, _, _) = row_footprint(other);
                            (ox, oy, ow, oh)
                        })
                        .collect();
                    let (nx, ny) = resolve_release(x, y, w, h, &others, snap_tolerance.get());
                    row.x.set_value(nx.round());
                    row.y.set_value(ny.round());
                    drop(rows);
                    mc.redraw();
                    if let Some(cb) = on_change.borrow().as_ref() {
                        cb();
                    }
                }
                drag_mode.set(DragMode::None);
            }
        });
        canvas.add_controller(drag);
    }

    {
        let form = form.clone();
        let rows = rows.clone();
        let selected = selected.clone();
        let on_change = on_change.clone();
        let mc = mc.clone();
        remove.connect_clicked(move |_| {
            let index = selected.get();
            let removed = {
                let mut rows = rows.borrow_mut();
                if index >= rows.len() {
                    return;
                }
                rows.remove(index)
            };
            form.remove(&removed.form_box);
            let count = rows.borrow().len();
            let next = if count == 0 { 0 } else { index.min(count - 1) };
            selected.set(next);
            for (position, item) in rows.borrow().iter().enumerate() {
                item.form_box.set_visible(position == next);
            }
            mc.refit(&rows.borrow());
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });
    }
    {
        let form = form.clone();
        let rows = rows.clone();
        let on_change = on_change.clone();
        let mc = mc.clone();
        let selected = selected.clone();
        let ctx = ctx.clone();
        let disabled = disabled.clone();
        add.connect_clicked(move |button| {
            let anchor = button.clone().upcast::<gtk::Widget>();
            let existing_names: Vec<String> = rows
                .borrow()
                .iter()
                .map(|row| row.name.text().to_string())
                .collect();
            let (tx, rx) =
                async_channel::bounded::<Option<Vec<crate::monitor_detect::DetectedMonitor>>>(1);
            std::thread::spawn(move || {
                let _ = tx.send_blocking(crate::monitor_detect::detect());
            });
            let form = form.clone();
            let rows = rows.clone();
            let on_change = on_change.clone();
            let mc = mc.clone();
            let selected = selected.clone();
            let ctx = ctx.clone();
            let disabled = disabled.clone();
            open_monitor_picker(&anchor, existing_names, rx, move |picked| {
                if let Some((name, _)) = &picked {
                    if rows.borrow().iter().any(|row| row.name.text() == *name) {
                        return;
                    }
                }
                let scan = monitor_scaffold(&rows.borrow(), picked.as_ref());
                let row = make_output_row(&scan, &ctx, &disabled);
                let index = attach_monitor_row(&form, &rows, &mc, row);
                selected.set(index);
                for (position, item) in rows.borrow().iter().enumerate() {
                    item.form_box.set_visible(position == index);
                }
                mc.refit(&rows.borrow());
                if let Some(cb) = on_change.borrow().as_ref() {
                    cb();
                }
            });
        });
    }

    Editor::Monitors {
        root,
        rows,
        on_change,
        error: None,
        original: record.value.clone(),
    }
}

pub(crate) fn monitor_scaffold(
    rows: &[MonitorRow],
    picked: Option<&(String, DetectedMode)>,
) -> OutputScan {
    let (name, width, height, refresh) = match picked {
        Some((name, mode)) => (name.clone(), mode.width, mode.height, mode.refresh),
        None => (String::new(), 1920, 1080, 60.0),
    };
    let x = rows
        .iter()
        .map(|row| {
            let (rx, _, rw, _, _, _) = row_footprint(row);
            rx + rw
        })
        .fold(0.0f64, f64::max);
    let fields = vec![
        ("width".to_string(), width.to_string()),
        ("height".to_string(), height.to_string()),
        ("refresh".to_string(), format!("{refresh}")),
        ("xPos".to_string(), (x as i64).to_string()),
        ("yPos".to_string(), "0".to_string()),
        ("scale".to_string(), "1.0".to_string()),
        ("transform".to_string(), quote("normal")),
    ];
    OutputScan { name, fields }
}

#[cfg(test)]
mod tests {
    use super::{output_scan, refresh_literal};

    #[test]
    fn keeps_fractional_refresh_rates() {
        assert_eq!(refresh_literal(59.94), "59.94");
        assert_eq!(refresh_literal(144.0), "144");
        assert_eq!(refresh_literal(165.02), "165.02");
    }

    #[test]
    fn reads_grouped_and_unknown_output_fields() {
        let value = r#"{
          "eDP-1" = {
            width = 1920;
            hdr = { enabled = true; minLuminance = 0.5; };
            customKey = 3;
          };
        }"#;
        let outputs = output_scan(value).unwrap();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name, "eDP-1");
        assert_eq!(
            outputs[0]
                .fields
                .iter()
                .find(|(key, _)| key == "width")
                .map(|(_, value)| value.as_str()),
            Some("1920")
        );
        assert_eq!(
            outputs[0]
                .fields
                .iter()
                .find(|(key, _)| key == "hdr.enabled")
                .map(|(_, value)| value.as_str()),
            Some("true")
        );
        assert_eq!(
            outputs[0]
                .fields
                .iter()
                .find(|(key, _)| key == "customKey")
                .map(|(_, value)| value.as_str()),
            Some("3")
        );
    }

    #[test]
    fn rejects_apply_wrapped_outputs() {
        let value = "map withIcc [ { name = \"eDP-1\"; width = 1920; } ]";
        let error = output_scan(value).unwrap_err();
        assert!(error.contains("literal attribute set"));
    }

    #[test]
    fn rejects_non_attrset_output() {
        let value = "{ \"eDP-1\" = [ 1920 ]; }";
        let error = output_scan(value).unwrap_err();
        assert!(error.contains("output 'eDP-1'"));
    }
}
