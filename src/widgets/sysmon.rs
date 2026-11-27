#![cfg_attr(not(feature = "ojuny-widgets"), allow(dead_code))]

use super::context::ChangeHook;
use super::editor::Editor;
use super::nix::{attr_key, parse_string_literal, quote};
use crate::model::Record;
use crate::scan;
use crate::value;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const DEFAULT_LAYOUT: &str = "tiles";
const DEFAULT_BAR_ORIENTATION: &str = "auto";
const DEFAULT_BAR_STYLE: &str = "brackets";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SysmonMetric {
    pub device: String,
    pub vals: Vec<String>,
    pub graph: bool,
    pub on: bool,
    pub unknown: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SysmonConfig {
    pub metrics: Vec<SysmonMetric>,
    pub layout: String,
    pub show_header: bool,
    pub bar_orientation: String,
    pub bar_names: bool,
    pub bar_style: String,
    pub unknown: Vec<(String, String)>,
}

impl Default for SysmonConfig {
    fn default() -> Self {
        Self {
            metrics: Vec::new(),
            layout: DEFAULT_LAYOUT.to_string(),
            show_header: false,
            bar_orientation: DEFAULT_BAR_ORIENTATION.to_string(),
            bar_names: false,
            bar_style: DEFAULT_BAR_STYLE.to_string(),
            unknown: Vec::new(),
        }
    }
}

impl SysmonConfig {
    /// Parse a card `config` attribute set. Unknown config keys and unknown
    /// metric fields are kept verbatim so a save never drops data the editor
    /// does not model.
    pub(crate) fn parse(text: &str) -> Self {
        let mut config = Self::default();
        for child in scan::scan_value_direct(text) {
            let name = child.segments.last().cloned().unwrap_or_default();
            match name.as_str() {
                "metrics" => {
                    config.metrics = value::list_items(&child.value)
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|item| parse_metric(item))
                        .collect();
                }
                "layout" => config.layout = parse_string_literal(&child.value),
                "showHeader" => config.show_header = child.value.trim() == "true",
                "barOrientation" => config.bar_orientation = parse_string_literal(&child.value),
                "barNames" => config.bar_names = child.value.trim() == "true",
                "barStyle" => config.bar_style = parse_string_literal(&child.value),
                _ => config.unknown.push((name, child.value.clone())),
            }
        }
        config
    }

    pub(crate) fn to_nix(&self) -> String {
        let metrics: Vec<String> = self.metrics.iter().map(SysmonMetric::to_nix).collect();
        let mut parts = vec![
            format!("metrics = [ {} ];", metrics.join(" ")),
            format!("layout = {};", quote(&self.layout)),
            format!("showHeader = {};", self.show_header),
            format!("barOrientation = {};", quote(&self.bar_orientation)),
            format!("barNames = {};", self.bar_names),
            format!("barStyle = {};", quote(&self.bar_style)),
        ];
        for (name, value) in &self.unknown {
            parts.push(format!("{} = {};", attr_key(name), value));
        }
        format!("{{ {} }}", parts.join(" "))
    }

    fn unchanged(&self, original: &str) -> bool {
        *self == Self::parse(original)
    }
}

impl SysmonMetric {
    fn to_nix(&self) -> String {
        let vals: Vec<String> = if self.vals.is_empty() {
            dms_vals(&self.device)
                .first()
                .map(|val| quote(val))
                .into_iter()
                .collect()
        } else {
            self.vals.iter().map(|val| quote(val)).collect()
        };
        let mut parts = vec![
            format!("device = {};", quote(&self.device)),
            format!("graph = {};", self.graph),
            format!("on = {};", self.on),
            format!("vals = [ {} ];", vals.join(" ")),
        ];
        for (name, value) in &self.unknown {
            parts.push(format!("{} = {};", attr_key(name), value));
        }
        format!("{{ {} }}", parts.join(" "))
    }
}

fn parse_metric(text: &str) -> Option<SysmonMetric> {
    let mut device = String::new();
    let mut vals = Vec::new();
    let mut graph = false;
    let mut on = true;
    let mut unknown = Vec::new();
    for child in scan::scan_value_direct(text) {
        let name = child.segments.last().cloned().unwrap_or_default();
        match name.as_str() {
            "device" => device = parse_string_literal(&child.value),
            "vals" => {
                vals = value::list_items(&child.value)
                    .unwrap_or_default()
                    .iter()
                    .map(|item| parse_string_literal(item))
                    .collect();
            }
            "graph" => graph = child.value.trim() == "true",
            "on" => on = child.value.trim() != "false",
            _ => unknown.push((name, child.value.clone())),
        }
    }
    if !super::dms_catalog::sysmon_device_is_valid(&device) {
        return None;
    }
    let vals = normalize_vals(&device, vals);
    Some(SysmonMetric {
        device,
        vals,
        graph,
        on,
        unknown,
    })
}

fn dms_vals(device: &str) -> &'static [&'static str] {
    super::dms_catalog::sysmon_device_type(device)
        .map(|(device_type, _)| super::dms_catalog::sysmon_vals(device_type))
        .unwrap_or(&[])
}

/// Keep only vals the device type supports, drop duplicates, and fall back to
/// the first supported value when nothing remains. Order is preserved because
/// `vals[0]` is the metric's main value.
fn normalize_vals(device: &str, vals: Vec<String>) -> Vec<String> {
    let allowed = dms_vals(device);
    let mut normalized = Vec::new();
    for val in vals {
        if allowed.contains(&val.as_str()) && !normalized.contains(&val) {
            normalized.push(val);
        }
    }
    if normalized.is_empty() {
        if let Some(first) = allowed.first() {
            normalized.push((*first).to_string());
        }
    }
    normalized
}

fn device_type(device: &str) -> &str {
    device
        .split_once(':')
        .map(|(kind, _)| kind)
        .unwrap_or(device)
}

fn device_suffix(device: &str) -> &str {
    device
        .split_once(':')
        .map(|(_, suffix)| suffix)
        .unwrap_or("")
}

fn device_for(kind: &str, suffix: &str) -> String {
    if suffix.is_empty() || kind == "cpu" || kind == "memory" {
        kind.to_string()
    } else {
        format!("{kind}:{suffix}")
    }
}

pub(crate) struct SysmonUi {
    root: gtk::Box,
    state: Rc<RefCell<SysmonConfig>>,
    on_change: ChangeHook,
    metrics_box: gtk::Box,
}

impl SysmonUi {
    fn new(state: Rc<RefCell<SysmonConfig>>) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_hexpand(true);
        let metrics_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        metrics_box.set_hexpand(true);
        let ui = Rc::new(Self {
            root,
            state,
            on_change: Rc::new(RefCell::new(None)),
            metrics_box,
        });
        let ui = ui;
        ui.build_layout();
        ui.root.append(&ui.metrics_box);
        ui.build_add_row();
        ui.rebuild_rows();
        ui
    }

    pub(crate) fn root(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    pub(crate) fn to_nix(&self) -> String {
        self.state.borrow().to_nix()
    }

    pub(crate) fn unchanged(&self, original: &str) -> bool {
        self.state.borrow().unchanged(original)
    }

    pub(crate) fn set_on_change(&self, hook: Rc<dyn Fn()>) {
        *self.on_change.borrow_mut() = Some(hook);
    }

    fn fire(&self) {
        let hook = self.on_change.borrow().clone();
        if let Some(hook) = hook {
            hook();
        }
    }

    fn schedule_rows(&self) {
        let ui = Rc::new(self.clone_ref());
        glib::idle_add_local_once(move || ui.rebuild_rows());
    }

    fn clone_ref(&self) -> Self {
        Self {
            root: self.root.clone(),
            state: self.state.clone(),
            on_change: self.on_change.clone(),
            metrics_box: self.metrics_box.clone(),
        }
    }

    fn build_layout(&self) {
        let section = gtk::Label::builder()
            .xalign(0.0)
            .label("Layout")
            .css_classes(["heading"])
            .build();
        self.root.append(&section);

        let layout = enum_dropdown(
            super::dms_catalog::SYSMON_LAYOUTS,
            &["Tiles", "List", "Bars", "Graphs"],
            &self.state.borrow().layout,
        );
        {
            let state = self.state.clone();
            let ui = self.clone_ref();
            layout.connect_selected_notify(move |combo| {
                if let Some(value) = combo.selected_item().and_then(|item| {
                    item.downcast::<gtk::StringObject>()
                        .ok()
                        .map(|s| s.string().to_string())
                }) {
                    state.borrow_mut().layout = value;
                    ui.fire();
                }
            });
        }
        self.root.append(&form_row("Layout", &layout));

        let show_header = gtk::Switch::new();
        show_header.set_active(self.state.borrow().show_header);
        show_header.set_halign(gtk::Align::Start);
        {
            let state = self.state.clone();
            let ui = self.clone_ref();
            show_header.connect_active_notify(move |switch| {
                state.borrow_mut().show_header = switch.is_active();
                ui.fire();
            });
        }
        self.root.append(&form_row("Show header", &show_header));

        let orientation = enum_dropdown(
            super::dms_catalog::SYSMON_ORIENTATIONS,
            &["Auto", "Horizontal", "Vertical"],
            &self.state.borrow().bar_orientation,
        );
        {
            let state = self.state.clone();
            let ui = self.clone_ref();
            orientation.connect_selected_notify(move |combo| {
                if let Some(value) = combo.selected_item().and_then(|item| {
                    item.downcast::<gtk::StringObject>()
                        .ok()
                        .map(|s| s.string().to_string())
                }) {
                    state.borrow_mut().bar_orientation = value;
                    ui.fire();
                }
            });
        }
        self.root.append(&form_row("Bar orientation", &orientation));

        let bar_names = gtk::Switch::new();
        bar_names.set_active(self.state.borrow().bar_names);
        bar_names.set_halign(gtk::Align::Start);
        {
            let state = self.state.clone();
            let ui = self.clone_ref();
            bar_names.connect_active_notify(move |switch| {
                state.borrow_mut().bar_names = switch.is_active();
                ui.fire();
            });
        }
        self.root.append(&form_row("Bar names", &bar_names));

        let bar_style = enum_dropdown(
            super::dms_catalog::SYSMON_BAR_STYLES,
            &["Brackets", "Tiles"],
            &self.state.borrow().bar_style,
        );
        {
            let state = self.state.clone();
            let ui = self.clone_ref();
            bar_style.connect_selected_notify(move |combo| {
                if let Some(value) = combo.selected_item().and_then(|item| {
                    item.downcast::<gtk::StringObject>()
                        .ok()
                        .map(|s| s.string().to_string())
                }) {
                    state.borrow_mut().bar_style = value;
                    ui.fire();
                }
            });
        }
        self.root.append(&form_row("Bar style", &bar_style));
    }

    fn build_add_row(&self) {
        let heading = gtk::Label::builder()
            .xalign(0.0)
            .margin_top(6)
            .label("Metrics")
            .css_classes(["heading"])
            .build();
        self.root.append(&heading);

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let kind = gtk::DropDown::from_strings(super::dms_catalog::SYSMON_DEVICE_TYPES);
        kind.set_halign(gtk::Align::Start);
        let suffix = gtk::Entry::new();
        suffix.set_placeholder_text(Some("disk or interface id (optional)"));
        suffix.set_hexpand(true);
        let add = gtk::Button::with_label("Add metric");
        row.append(&kind);
        row.append(&suffix);
        row.append(&add);
        self.root.append(&row);

        let hint = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .label("A GPU metric needs its bus id. Disk and network metrics without an id use the total.")
            .css_classes(["dim-label", "caption"])
            .build();
        self.root.append(&hint);

        let ui = self.clone_ref();
        add.connect_clicked(move |_| {
            let Some(item) = kind.selected_item() else {
                return;
            };
            let Ok(item) = item.downcast::<gtk::StringObject>() else {
                return;
            };
            let device_type = item.string().to_string();
            let device = device_for(&device_type, suffix.text().trim());
            if !super::dms_catalog::sysmon_device_is_valid(&device) {
                return;
            }
            let first = super::dms_catalog::sysmon_vals(&device_type)
                .first()
                .map(|value| (*value).to_string())
                .unwrap_or_default();
            ui.state.borrow_mut().metrics.push(SysmonMetric {
                device,
                vals: vec![first],
                graph: false,
                on: true,
                unknown: Vec::new(),
            });
            ui.schedule_rows();
            ui.fire();
        });
    }

    fn rebuild_rows(&self) {
        while let Some(child) = self.metrics_box.first_child() {
            self.metrics_box.remove(&child);
        }
        let metrics = self.state.borrow().metrics.clone();
        if metrics.is_empty() {
            let empty = gtk::Label::builder()
                .xalign(0.0)
                .wrap(true)
                .label("No metrics yet.")
                .css_classes(["dim-label"])
                .build();
            self.metrics_box.append(&empty);
            return;
        }
        for (index, metric) in metrics.iter().enumerate() {
            self.metrics_box.append(&self.metric_row(index, metric));
        }
    }

    fn metric_row(&self, index: usize, metric: &SysmonMetric) -> gtk::Widget {
        let frame = gtk::Box::new(gtk::Orientation::Vertical, 6);
        frame.set_margin_top(4);
        frame.set_margin_bottom(4);
        frame.add_css_class("card");
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);

        let kind = enum_dropdown(
            super::dms_catalog::SYSMON_DEVICE_TYPES,
            super::dms_catalog::SYSMON_DEVICE_TYPES,
            device_type(&metric.device),
        );
        {
            let ui = self.clone_ref();
            let suffix = device_suffix(&metric.device).to_string();
            kind.connect_selected_notify(move |combo| {
                let Some(item) = combo
                    .selected_item()
                    .and_then(|item| item.downcast::<gtk::StringObject>().ok())
                else {
                    return;
                };
                let kind = item.string().to_string();
                let device = device_for(&kind, &suffix);
                if let Some(metric) = ui.state.borrow_mut().metrics.get_mut(index) {
                    metric.device = device;
                    metric.vals = normalize_vals(&metric.device, std::mem::take(&mut metric.vals));
                }
                ui.schedule_rows();
                ui.fire();
            });
        }
        header.append(&kind);

        let suffix = gtk::Entry::new();
        suffix.set_text(device_suffix(&metric.device));
        suffix.set_placeholder_text(Some("id"));
        suffix.set_hexpand(true);
        {
            let ui = self.clone_ref();
            let kind = device_type(&metric.device).to_string();
            suffix.connect_changed(move |entry| {
                let device = device_for(&kind, entry.text().trim());
                if let Some(metric) = ui.state.borrow_mut().metrics.get_mut(index) {
                    metric.device = device;
                }
                ui.fire();
            });
        }
        header.append(&suffix);

        let on = gtk::Switch::new();
        on.set_active(metric.on);
        on.set_tooltip_text(Some("Show this metric"));
        {
            let ui = self.clone_ref();
            on.connect_active_notify(move |switch| {
                if let Some(metric) = ui.state.borrow_mut().metrics.get_mut(index) {
                    metric.on = switch.is_active();
                }
                ui.fire();
            });
        }
        header.append(&gtk::Label::new(Some("On")));
        header.append(&on);

        let graph = gtk::Switch::new();
        graph.set_active(metric.graph);
        graph.set_tooltip_text(Some("Draw a trend graph for this metric"));
        {
            let ui = self.clone_ref();
            graph.connect_active_notify(move |switch| {
                if let Some(metric) = ui.state.borrow_mut().metrics.get_mut(index) {
                    metric.graph = switch.is_active();
                }
                ui.fire();
            });
        }
        header.append(&gtk::Label::new(Some("Graph")));
        header.append(&graph);

        let up = gtk::Button::from_icon_name("go-up-symbolic");
        up.add_css_class("flat");
        up.set_tooltip_text(Some("Move this metric up"));
        {
            let ui = self.clone_ref();
            up.connect_clicked(move |_| {
                ui.move_metric(index, -1);
            });
        }
        let down = gtk::Button::from_icon_name("go-down-symbolic");
        down.add_css_class("flat");
        down.set_tooltip_text(Some("Move this metric down"));
        {
            let ui = self.clone_ref();
            down.connect_clicked(move |_| {
                ui.move_metric(index, 1);
            });
        }
        let remove = gtk::Button::from_icon_name("user-trash-symbolic");
        remove.add_css_class("flat");
        remove.set_tooltip_text(Some("Remove this metric"));
        {
            let ui = self.clone_ref();
            remove.connect_clicked(move |_| {
                if index < ui.state.borrow().metrics.len() {
                    ui.state.borrow_mut().metrics.remove(index);
                }
                ui.schedule_rows();
                ui.fire();
            });
        }
        header.append(&up);
        header.append(&down);
        header.append(&remove);
        frame.append(&header);

        let vals_label = gtk::Label::builder()
            .xalign(0.0)
            .label("Values")
            .css_classes(["dim-label"])
            .build();
        frame.append(&vals_label);
        let vals = gtk::FlowBox::new();
        vals.set_selection_mode(gtk::SelectionMode::None);
        vals.set_homogeneous(true);
        vals.set_max_children_per_line(6);
        let allowed = dms_vals(&metric.device);
        for allowed_val in allowed {
            let selected = metric.vals.iter().any(|val| val == allowed_val);
            let check = gtk::CheckButton::with_label(allowed_val);
            check.set_active(selected);
            {
                let ui = self.clone_ref();
                let value = (*allowed_val).to_string();
                check.connect_toggled(move |button| {
                    let active = button.is_active();
                    let mut state = ui.state.borrow_mut();
                    let Some(metric) = state.metrics.get_mut(index) else {
                        return;
                    };
                    if active {
                        if !metric.vals.contains(&value) {
                            metric.vals.push(value.clone());
                        }
                    } else {
                        metric.vals.retain(|val| val != &value);
                        if metric.vals.is_empty() {
                            if let Some(first) = dms_vals(&metric.device).first() {
                                metric.vals.push((*first).to_string());
                            }
                        }
                    }
                    let restored = metric.vals.contains(&value);
                    drop(state);
                    if !active && restored {
                        button.set_active(true);
                        return;
                    }
                    ui.fire();
                });
            }
            vals.append(&check);
        }
        frame.append(&vals);

        let main = strings_dropdown(
            &metric.vals,
            metric.vals.first().map(String::as_str).unwrap_or_default(),
        );
        {
            let ui = self.clone_ref();
            main.connect_selected_notify(move |combo| {
                let selected = combo.selected() as usize;
                let mut state = ui.state.borrow_mut();
                let Some(metric) = state.metrics.get_mut(index) else {
                    return;
                };
                if selected >= metric.vals.len() {
                    return;
                }
                let value = metric.vals.remove(selected);
                metric.vals.insert(0, value);
                drop(state);
                ui.schedule_rows();
                ui.fire();
            });
        }
        frame.append(&form_row("Main value", &main));

        frame.upcast()
    }

    fn move_metric(&self, index: usize, delta: i32) {
        let mut state = self.state.borrow_mut();
        let len = state.metrics.len();
        let target = index as i32 + delta;
        if target < 0 || target as usize >= len {
            return;
        }
        state.metrics.swap(index, target as usize);
        drop(state);
        self.schedule_rows();
        self.fire();
    }
}

fn strings_dropdown(values: &[String], current: &str) -> gtk::DropDown {
    let refs: Vec<&str> = values.iter().map(String::as_str).collect();
    let combo = gtk::DropDown::from_strings(&refs);
    let selected = values
        .iter()
        .position(|value| value == current)
        .unwrap_or(0);
    combo.set_selected(selected as u32);
    combo.set_halign(gtk::Align::Start);
    combo
}

fn form_row(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let caption = gtk::Label::builder()
        .xalign(0.0)
        .width_chars(18)
        .label(label)
        .css_classes(["dim-label"])
        .build();
    row.append(&caption);
    row.append(widget);
    row.upcast()
}

fn enum_dropdown(values: &[&str], labels: &[&str], current: &str) -> gtk::DropDown {
    let mut label_list: Vec<String> = labels.iter().map(|label| (*label).to_string()).collect();
    let mut value_list: Vec<String> = values.iter().map(|value| (*value).to_string()).collect();
    if !value_list.iter().any(|value| value == current) {
        label_list.push(current.to_string());
        value_list.push(current.to_string());
    }
    let refs: Vec<&str> = label_list.iter().map(String::as_str).collect();
    let combo = gtk::DropDown::from_strings(&refs);
    let selected = value_list
        .iter()
        .position(|value| value == current)
        .unwrap_or(0);
    combo.set_selected(selected as u32);
    combo.set_halign(gtk::Align::Start);
    combo
}

pub(crate) fn build_sysmon_config(record: &Record) -> Editor {
    let state = Rc::new(RefCell::new(SysmonConfig::parse(&record.value)));
    Editor::SysmonConfig {
        ui: SysmonUi::new(state),
        original: record.value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: &str = r#"{
      metrics = [
        { device = "cpu"; graph = true; on = true; vals = [ "usage" "temp" "clock" ]; }
        { device = "memory"; graph = true; on = true; vals = [ "used" "swap" ]; }
        { device = "disk:nvme0n1"; graph = true; on = true; vals = [ "space" "read" "write" ]; }
        { device = "net"; graph = false; on = true; vals = [ "down" "up" ]; }
        { device = "gpu:0000:01:00.0"; graph = false; on = true; vals = [ "usage" "temp" "vram" "power" ]; }
      ];
      layout = "bars"; showHeader = false; barOrientation = "horizontal";
      barNames = true; barStyle = "tiles";
    }"#;

    #[test]
    fn parses_the_target_sysmon_config() {
        let cfg = SysmonConfig::parse(TARGET);
        assert_eq!(cfg.metrics.len(), 5);
        assert_eq!(cfg.metrics[0].vals, ["usage", "temp", "clock"]);
        assert_eq!(cfg.metrics[2].device, "disk:nvme0n1");
        assert_eq!(cfg.layout, "bars");
        assert!(cfg.bar_names);
        let nix = cfg.to_nix();
        assert_eq!(nix, cfg.to_nix());
        assert!(cfg.unchanged(TARGET));
        assert!(cfg.unchanged(&nix));
    }

    #[test]
    fn invalid_metric_values_are_normalized() {
        let cfg = SysmonConfig::parse(r#"{ metrics = [ { device = "cpu:0"; vals = []; } ]; }"#);
        assert!(cfg.metrics.is_empty());

        let cfg = SysmonConfig::parse(
            r#"{ metrics = [ { device = "cpu"; vals = [ "temp" "bogus" "temp" ]; } ]; }"#,
        );
        assert_eq!(cfg.metrics[0].vals, ["temp"]);
    }

    #[test]
    fn unknown_keys_and_mount_are_preserved() {
        let cfg = SysmonConfig::parse(
            r#"{
              tone = "primary"; title = "Box";
              metrics = [ { device = "disk:nvme0n1"; vals = [ "space" ]; mount = "/home"; } ];
            }"#,
        );
        let out = cfg.to_nix();
        assert!(out.contains(r#"tone = "primary""#));
        assert!(out.contains(r#"mount = "/home""#));
        assert!(cfg.unchanged(&out));
    }

    #[test]
    fn a_metric_without_vals_falls_back_to_its_first_value() {
        let cfg = SysmonConfig::parse(r#"{ metrics = [ { device = "memory"; } ]; }"#);
        assert_eq!(cfg.metrics[0].vals, ["used"]);
        assert!(cfg.metrics[0].on);
        assert!(!cfg.metrics[0].graph);
    }

    #[test]
    #[ignore = "needs a display; run with `cargo test -- --ignored`"]
    fn group_editor_edits_target() {
        if gtk::init().is_err() {
            eprintln!("skipping sysmon runtime check: no display");
            return;
        }
        let record = Record {
            segments: vec!["config".into()],
            value: TARGET.to_string(),
            ..Record::default()
        };
        let editor = build_sysmon_config(&record);
        let Editor::SysmonConfig { ui, original } = &editor else {
            panic!("sysmon editor expected");
        };
        assert!(ui.unchanged(original));
        {
            let mut state = ui.state.borrow_mut();
            state.bar_names = false;
            state.metrics[0].graph = false;
            let main = state.metrics[0].vals.remove(1);
            state.metrics[0].vals.insert(0, main);
        }
        let nix = editor.to_nix();
        assert!(nix.contains("barNames = false"));
        assert!(nix.contains("graph = false"));
        let reparsed = SysmonConfig::parse(&nix);
        assert_eq!(reparsed.metrics[0].vals, ["temp", "usage", "clock"]);
        assert!(!editor.unchanged(original));
    }
}
