use super::basic::build_raw;
use super::context::{BuildContext, ChangeHook};
use super::editor::Editor;
use super::fields::{
    attrset_field_nix, build_field_cell, capture_field_values, csv_list, dim_line,
    dim_line_with_help, field, field_has_content, popout_button_bar, restore_field_values,
    AttrsetField,
};
use super::names::{bind_display_label, keybind_field_help};
use super::nix::{attr_key, entry_text, parse_string_literal, quote, unwrap_modifiers};
use super::popout::PopoutSnapshot;
use crate::dsl::OptionSpec;
use crate::model::Record;
use crate::scan;
use crate::value;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

const DEFAULT_BIND_FIELDS: &[(&str, BindFieldKind)] = &[
    ("modifier", BindFieldKind::List),
    ("keys", BindFieldKind::List),
    ("dispatcher", BindFieldKind::Text),
    ("args", BindFieldKind::Text),
    ("description", BindFieldKind::Text),
    ("group", BindFieldKind::Text),
    ("subcat", BindFieldKind::Text),
];

const DEFAULT_BIND_PREVIEW: &[&str] = &["modifier", "keys"];
const DEFAULT_BIND_SORT: &[&str] = &["group", "subcat"];
const EXTRAS_FIELD: &str = "__extras";
const EXTRAS_LABEL: &str = "Other fields";

static NEXT_ROW_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BindFieldKind {
    Bool,
    Int,
    Float,
    Text,
    List,
}

impl BindFieldKind {
    fn parse(text: &str) -> Option<Self> {
        match text.trim().to_lowercase().as_str() {
            "bool" | "boolean" => Some(Self::Bool),
            "int" | "integer" => Some(Self::Int),
            "float" | "number" => Some(Self::Float),
            "list" | "string-list" | "strings" => Some(Self::List),
            "str" | "string" | "text" => Some(Self::Text),
            _ => None,
        }
    }

    fn default_source(self) -> &'static str {
        match self {
            Self::Bool => "false",
            Self::Int => "0",
            Self::Float => "0.0",
            Self::Text => "\"\"",
            Self::List => "[]",
        }
    }
}

#[derive(Clone, Debug)]
struct BindSchema {
    fields: Vec<(String, BindFieldKind)>,
    preview: Vec<String>,
    sort: Vec<String>,
    key_field: Option<String>,
}

impl BindSchema {
    fn from_spec(spec: &OptionSpec) -> Self {
        let fields: Vec<(String, BindFieldKind)> = match spec.bind_fields.as_ref() {
            Some(names) if !names.is_empty() => names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let kind = spec
                        .bind_field_types
                        .as_ref()
                        .and_then(|types| types.get(index))
                        .and_then(|kind| BindFieldKind::parse(kind))
                        .unwrap_or_else(|| default_kind(name));
                    (name.clone(), kind)
                })
                .collect(),
            _ => DEFAULT_BIND_FIELDS
                .iter()
                .map(|(name, kind)| ((*name).to_string(), *kind))
                .collect(),
        };
        let preview: Vec<String> = spec.bind_preview.clone().unwrap_or_else(|| {
            DEFAULT_BIND_PREVIEW
                .iter()
                .map(|name| name.to_string())
                .collect()
        });
        let sort: Vec<String> = spec.bind_sort.clone().unwrap_or_else(|| {
            DEFAULT_BIND_SORT
                .iter()
                .map(|name| name.to_string())
                .collect()
        });
        let preview = preview
            .into_iter()
            .filter(|name| fields.iter().any(|(field, _)| field == name))
            .collect();
        let sort = sort
            .into_iter()
            .filter(|name| fields.iter().any(|(field, _)| field == name))
            .collect();
        Self {
            fields,
            preview,
            sort,
            key_field: spec.bind_key_field.clone(),
        }
    }

    fn kind(&self, name: &str) -> Option<BindFieldKind> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, kind)| *kind)
    }
}

fn default_kind(name: &str) -> BindFieldKind {
    DEFAULT_BIND_FIELDS
        .iter()
        .find(|(field, _)| *field == name)
        .map(|(_, kind)| *kind)
        .unwrap_or(BindFieldKind::Text)
}

#[derive(Clone)]
struct PreviewField {
    name: String,
    kind: BindFieldKind,
    entry: gtk::Entry,
    orig: String,
}

/// One bind of the `keybinds` widget: a display label and description on the
/// main line, the bind's configured fields in a hidden edit popup, and any
/// unknown fields in a raw editor.
#[derive(Clone)]
pub(crate) struct KeybindRow {
    id: u64,
    key: gtk::Entry,
    orig_key: String,
    key_locked: bool,
    preview: Rc<RefCell<Vec<PreviewField>>>,
    fields: Rc<RefCell<Vec<AttrsetField>>>,
    field_order: Vec<String>,
    pub(crate) row_box: gtk::Box,
    remove: gtk::Button,
    popup: gtk::Window,
    snapshot: Rc<RefCell<Option<PopoutSnapshot>>>,
    #[allow(dead_code)]
    pub(crate) search_text: String,
}

fn next_row_id() -> u64 {
    NEXT_ROW_ID.fetch_add(1, Ordering::Relaxed)
}

/// Install the one-time stylesheet that paints the keybind preview pill.
pub(crate) fn install_keybind_css() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let css = ".keybind-preview {
            background-color: alpha(@window_fg_color, 0.08);
            border-radius: 8px;
            padding: 2px 8px;
        }";
        let provider = gtk::CssProvider::new();
        provider.load_from_data(css);
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
}

/// The `modifier+keys` preview shown on a keybind row.
#[allow(dead_code)]
pub(crate) fn bind_preview(modifier: &str, keys: &str) -> String {
    let mut parts: Vec<String> = csv_list(modifier)
        .into_iter()
        .map(|m| m.to_uppercase())
        .collect();
    parts.extend(csv_list(keys).into_iter().map(|k| k.to_uppercase()));
    parts.join("+")
}

fn preview_label(preview: &[PreviewField]) -> String {
    let mut parts = Vec::new();
    for field in preview {
        let text = field.entry.text().to_string();
        match field.kind {
            BindFieldKind::List => {
                parts.extend(csv_list(&text).into_iter().map(|item| item.to_uppercase()))
            }
            _ => {
                if !text.trim().is_empty() {
                    parts.push(text.trim().to_string());
                }
            }
        }
    }
    parts.join("+")
}

fn preview_text(source: &str, kind: BindFieldKind) -> String {
    match kind {
        BindFieldKind::List => list_text(source),
        BindFieldKind::Text => entry_text(source),
        _ => source.trim().to_string(),
    }
}

fn preview_nix(text: &str, kind: BindFieldKind) -> String {
    match kind {
        BindFieldKind::List => csv_list_nix(text),
        BindFieldKind::Text => quote(text),
        _ => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                kind.default_source().to_string()
            } else {
                trimmed.to_string()
            }
        }
    }
}

fn preview_unchanged(field: &PreviewField) -> bool {
    let current = field.entry.text().to_string();
    match field.kind {
        BindFieldKind::List => csv_list(&current) == csv_list(&list_text(&field.orig)),
        BindFieldKind::Text => current == entry_text(&field.orig),
        _ => current.trim() == field.orig.trim(),
    }
}

/// A comma-separated list field rendered as a Nix list of strings.
pub(crate) fn csv_list_nix(text: &str) -> String {
    let items: Vec<String> = csv_list(text).iter().map(|t| quote(t)).collect();
    value::serialize_list(&items)
}

/// Keep a horizontal separator between consecutive keybind rows: drop any
/// existing separators and re-insert them between the current row widgets.
pub(crate) fn sync_keybind_separators(container: &gtk::Box) {
    let mut children: Vec<gtk::Widget> = Vec::new();
    let mut child = container.first_child();
    while let Some(widget) = child {
        children.push(widget.clone());
        child = widget.next_sibling();
    }
    let mut separators: Vec<gtk::Widget> = Vec::new();
    let mut rows: Vec<gtk::Widget> = Vec::new();
    for widget in children {
        if widget.downcast_ref::<gtk::Separator>().is_some() {
            separators.push(widget);
        } else {
            rows.push(widget);
        }
    }
    for separator in separators {
        container.remove(&separator);
    }
    for pair in rows.windows(2) {
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_margin_top(6);
        separator.set_margin_bottom(6);
        container.insert_child_after(&separator, Some(&pair[0]));
    }
}

fn make_typed_field(
    name: &str,
    raw: &str,
    kind: BindFieldKind,
    ctx: &BuildContext,
) -> AttrsetField {
    let trimmed = raw.trim();
    let present = !trimmed.is_empty() && trimmed != "null";
    let source = if present {
        trimmed.to_string()
    } else {
        kind.default_source().to_string()
    };
    AttrsetField {
        name: name.to_string(),
        editor: build_field_cell(&source, ctx),
        orig_value: if present { source } else { String::new() },
        orig_present: present,
    }
}

fn extras_text(scan_row: &scan::TableRowScan, schema: &BindSchema) -> String {
    let mut text = String::new();
    for (name, value) in &scan_row.fields {
        if schema.fields.iter().any(|(field, _)| field == name) {
            continue;
        }
        text.push_str(&format!("{} = {}; ", attr_key(name), value.trim()));
    }
    text.trim().to_string()
}

/// One bind of the `keybinds` widget: a display label and description on the
/// main line, the preview pill, and an edit popup holding every configured
/// field plus an editor for unknown fields.
fn make_keybind_row(
    scan_row: &scan::TableRowScan,
    schema: &BindSchema,
    list: bool,
    ctx: &BuildContext,
) -> KeybindRow {
    install_keybind_css();
    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    row_box.set_hexpand(true);
    let main = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title_stack = gtk::Box::new(gtk::Orientation::Vertical, 2);
    title_stack.set_hexpand(true);

    let label_override = scan_row.spec.label.clone();
    let label_source = if list {
        schema
            .key_field
            .as_ref()
            .and_then(|name| field(&scan_row.fields, name))
            .map(entry_text)
            .unwrap_or_default()
    } else {
        scan_row.key.clone()
    };
    let mut search_text = format!(
        "{} {}",
        label_source,
        bind_display_label(&label_source, label_override.as_deref())
    );
    for (_, value) in &scan_row.fields {
        search_text.push(' ');
        search_text.push_str(value);
    }
    let title = gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .label(bind_display_label(&label_source, label_override.as_deref()))
        .build();
    title_stack.append(&title);

    let description = scan_row
        .fields
        .iter()
        .find(|(name, _)| name == "description")
        .map(|(_, value)| entry_text(value))
        .unwrap_or_default();
    let desc_caption = gtk::Label::builder()
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .label(&description)
        .css_classes(["dim-label", "caption"])
        .build();
    title_stack.append(&desc_caption);
    main.append(&title_stack);

    let mut preview_values = Vec::new();
    for name in &schema.preview {
        let kind = schema.kind(name).unwrap_or(BindFieldKind::Text);
        let raw = field(&scan_row.fields, name).unwrap_or("");
        let entry = gtk::Entry::new();
        entry.set_width_chars(if kind == BindFieldKind::List { 16 } else { 12 });
        entry.set_tooltip_text(Some(if kind == BindFieldKind::List {
            "Values, comma separated"
        } else {
            "Preview field"
        }));
        entry.set_text(&preview_text(raw, kind));
        preview_values.push(PreviewField {
            name: name.clone(),
            kind,
            entry,
            orig: raw.to_string(),
        });
    }
    let preview = Rc::new(RefCell::new(preview_values));
    let preview_widget = gtk::Label::builder()
        .label(preview_label(&preview.borrow()))
        .css_classes(["keybind-preview", "caption"])
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build();
    for field in preview.borrow().iter() {
        let update = preview_widget.clone();
        let values = preview.clone();
        field.entry.clone().connect_changed(move |_| {
            update.set_label(&preview_label(&values.borrow()));
        });
    }
    main.append(&preview_widget);

    let mut field_values = Vec::new();
    for (name, kind) in &schema.fields {
        if schema.preview.iter().any(|preview| preview == name) {
            continue;
        }
        let raw = field(&scan_row.fields, name).unwrap_or("");
        field_values.push(make_typed_field(name, raw, *kind, ctx));
    }
    let extras = extras_text(scan_row, schema);
    if !extras.is_empty() {
        let (view, _root) = build_raw(&extras, &OptionSpec::default());
        field_values.push(AttrsetField {
            name: EXTRAS_FIELD.to_string(),
            editor: Editor::Raw { view, root: _root },
            orig_value: extras,
            orig_present: true,
        });
    }
    let fields = Rc::new(RefCell::new(field_values));

    let key = gtk::Entry::new();
    key.set_hexpand(true);
    key.set_text(&scan_row.key);
    key.set_visible(!list);

    let popup = gtk::Window::builder()
        .title(bind_display_label(&label_source, label_override.as_deref()))
        .modal(true)
        .default_width(440)
        .hide_on_close(true)
        .build();
    let popup_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    popup_box.set_margin_top(12);
    popup_box.set_margin_bottom(12);
    popup_box.set_margin_start(12);
    popup_box.set_margin_end(12);

    let popup_desc = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .label(&description)
        .css_classes(["dim-label", "caption"])
        .build();
    popup_box.append(&popup_desc);
    if !list {
        popup_box.append(&dim_line("name", &key));
    }
    for f in fields.borrow().iter() {
        let label = if f.name == EXTRAS_FIELD {
            EXTRAS_LABEL
        } else {
            f.name.as_str()
        };
        let help = if f.name == EXTRAS_FIELD {
            Some("Unrecognized fields are preserved verbatim.")
        } else {
            keybind_field_help(&f.name)
        };
        popup_box.append(&dim_line_with_help(label, help, &f.editor.root()));
    }
    let (bar, cancel, apply) = popout_button_bar();
    popup_box.append(&bar);
    popup.set_child(Some(&popup_box));

    let edit = gtk::Button::with_label("Edit");
    main.append(&edit);
    let remove = gtk::Button::from_icon_name("user-trash-symbolic");
    remove.add_css_class("flat");
    remove.set_tooltip_text(Some("Remove bind"));
    main.append(&remove);
    row_box.append(&main);

    let row = KeybindRow {
        id: next_row_id(),
        key,
        orig_key: scan_row.key.clone(),
        key_locked: list,
        preview,
        fields,
        field_order: schema.fields.iter().map(|(name, _)| name.clone()).collect(),
        row_box,
        remove,
        popup,
        snapshot: Rc::new(RefCell::new(None)),
        search_text,
    };

    {
        let popup = row.popup.clone();
        row.key.connect_changed(move |entry| {
            let label = bind_display_label(&entry.text(), label_override.as_deref());
            title.set_label(&label);
            popup.set_title(Some(&label));
        });
    }
    {
        if let Some(desc) = row.fields.borrow().iter().find(|f| f.name == "description") {
            if let Editor::Text(entry) = &desc.editor {
                let entry_cb = entry.clone();
                entry.connect_changed(move |_| {
                    desc_caption.set_label(&entry_cb.text());
                    popup_desc.set_label(&entry_cb.text());
                });
            }
        }
    }
    {
        let popup = row.popup.clone();
        let snapshot = row.snapshot.clone();
        let fields = row.fields.clone();
        let key = row.key.clone();
        let edit_probe = edit.clone();
        edit.connect_clicked(move |_| {
            if popup.transient_for().is_none() {
                if let Some(parent) = edit_probe.root().and_downcast::<gtk::Window>() {
                    popup.set_transient_for(Some(&parent));
                }
            }
            let values = capture_field_values(&fields.borrow());
            *snapshot.borrow_mut() = Some(PopoutSnapshot {
                name: key.text().to_string(),
                fields: values,
                invalid_provider: None,
            });
            popup.set_default_size(440, -1);
            popup.present();
        });
    }
    {
        let popup = row.popup.clone();
        let snapshot = row.snapshot.clone();
        apply.connect_clicked(move |_| {
            snapshot.borrow_mut().take();
            popup.set_visible(false);
        });
    }
    {
        let popup = row.popup.clone();
        cancel.connect_clicked(move |_| popup.close());
    }
    {
        let snapshot = row.snapshot.clone();
        let fields = row.fields.clone();
        let key = row.key.clone();
        let key_locked = row.key_locked;
        row.popup.connect_close_request(move |_| {
            let taken = snapshot.borrow_mut().take();
            if let Some(popout) = taken {
                if !key_locked {
                    key.set_text(&popout.name);
                }
                restore_field_values(&fields.borrow(), &popout.fields);
            }
            glib::Propagation::Proceed
        });
    }
    {
        let popup = row.popup.clone();
        let controller = gtk::EventControllerKey::new();
        controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                popup.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        row.popup.add_controller(controller);
    }

    row
}

/// The comma-separated text shown in a modifier/keys entry.
pub(crate) fn list_text(source: &str) -> String {
    value::list_items(source)
        .map(|items| {
            items
                .iter()
                .map(|item| parse_string_literal(item))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// Whether a comma-separated entry still matches the Nix list source it was
/// built from. The entry holds the display form, so the original must go
/// through `list_text` before the two can be compared.
#[allow(dead_code)]
pub(crate) fn csv_matches_list_source(text: &str, original: &str) -> bool {
    csv_list(text) == csv_list(&list_text(original))
}

fn row_fields_nix(row: &KeybindRow) -> Vec<String> {
    let mut fields = Vec::new();
    for name in &row.field_order {
        if let Some(preview) = row
            .preview
            .borrow()
            .iter()
            .find(|field| &field.name == name)
        {
            fields.push(format!(
                "{} = {};",
                attr_key(name),
                preview_nix(&preview.entry.text(), preview.kind)
            ));
            continue;
        }
        if let Some(field) = row.fields.borrow().iter().find(|field| &field.name == name) {
            if let Some(line) = attrset_field_nix(field) {
                fields.push(line);
            }
        }
    }
    if let Some(extras) = row
        .fields
        .borrow()
        .iter()
        .find(|field| field.name == EXTRAS_FIELD)
    {
        if let Editor::Raw { view, .. } = &extras.editor {
            let buffer = view.buffer();
            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();
            if !text.trim().is_empty() {
                fields.push(text.trim().to_string());
            }
        }
    }
    fields
}

pub(crate) fn keybinds_nix(rows: &[KeybindRow], list: bool) -> String {
    let parts: Vec<String> = rows
        .iter()
        .map(|row| {
            let fields = row_fields_nix(row);
            if list {
                format!("{{ {} }}", fields.join(" "))
            } else {
                format!(
                    "{} = {{ {} }};",
                    attr_key(&row.key.text()),
                    fields.join(" ")
                )
            }
        })
        .collect();
    if list {
        value::serialize_list(&parts)
    } else {
        format!("{{ {} }}", parts.join(" "))
    }
}

fn row_unchanged(row: &KeybindRow) -> bool {
    if row.orig_key == "\0" {
        return false;
    }
    if !row.key_locked && row.key.text() != row.orig_key {
        return false;
    }
    if !row.preview.borrow().iter().all(preview_unchanged) {
        return false;
    }
    row.fields.borrow().iter().all(|field| {
        field.editor.unchanged(&field.orig_value)
            || (!field.orig_present && !field_has_content(&field.editor))
    })
}

pub(crate) fn keybind_rows_unchanged(rows: &[KeybindRow]) -> bool {
    rows.iter().all(row_unchanged)
}

pub(crate) fn connect_keybind_row(row: &KeybindRow, update: &Rc<dyn Fn()>) {
    {
        let update = update.clone();
        row.key.connect_changed(move |_| update());
    }
    for field in row.preview.borrow().iter() {
        let update = update.clone();
        field.entry.connect_changed(move |_| update());
    }
    for field in row.fields.borrow().iter() {
        field.editor.connect_changed(update);
    }
}

fn scaffold_fields(schema: &BindSchema) -> Vec<(String, String)> {
    schema
        .fields
        .iter()
        .map(|(name, _kind)| (name.clone(), String::new()))
        .collect()
}

fn unique_key(rows: &[KeybindRow]) -> String {
    let existing: Vec<String> = rows.iter().map(|row| row.key.text().to_string()).collect();
    let mut index = 1;
    loop {
        let candidate = if index == 1 {
            "bind".to_string()
        } else {
            format!("bind{index}")
        };
        if !existing.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

pub(crate) fn build_keybinds(record: &Record) -> Editor {
    let ctx = BuildContext::default();
    let schema = BindSchema::from_spec(&record.spec);
    let value_text = unwrap_modifiers(&record.value);
    let value_text = value_text.as_str();
    let kind = value::classify(value_text);
    if !matches!(kind, value::ValueKind::AttrSet | value::ValueKind::List) {
        let (view, root) = build_raw(value_text, &record.spec);
        return Editor::Raw { view, root };
    }
    let list = kind == value::ValueKind::List;

    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
    root.set_hexpand(true);
    let container = gtk::Box::new(gtk::Orientation::Vertical, 6);
    container.set_hexpand(true);
    let rows: Rc<RefCell<Vec<KeybindRow>>> = Rc::new(RefCell::new(Vec::new()));
    let on_change: ChangeHook = Rc::new(RefCell::new(None));

    let mut scanned = scan::scan_table(value_text, list);
    if !schema.sort.is_empty() {
        scanned.sort_by(|a, b| {
            let key = |row: &scan::TableRowScan| {
                let mut parts: Vec<String> = schema
                    .sort
                    .iter()
                    .map(|name| field(&row.fields, name).unwrap_or("").to_string())
                    .collect();
                parts.push(row.key.clone());
                parts
            };
            key(a).cmp(&key(b))
        });
    }

    for scan_row in scanned {
        let row = make_keybind_row(&scan_row, &schema, list, &ctx);
        wire_remove(&container, &rows, &on_change, &row);
        container.append(&row.row_box);
        sync_keybind_separators(&container);
        rows.borrow_mut().push(row);
    }

    let add = gtk::Button::with_label("Add bind");
    add.set_halign(gtk::Align::Start);
    {
        let container = container.clone();
        let rows = rows.clone();
        let on_change = on_change.clone();
        let schema = schema.clone();
        add.connect_clicked(move |_| {
            let key = if list {
                String::new()
            } else {
                unique_key(&rows.borrow())
            };
            let scan_row = scan::TableRowScan {
                key,
                fields: scaffold_fields(&schema),
                spec: OptionSpec::default(),
            };
            let mut row = make_keybind_row(&scan_row, &schema, list, &ctx);
            row.orig_key = "\0".to_string();
            wire_remove(&container, &rows, &on_change, &row);
            container.append(&row.row_box);
            sync_keybind_separators(&container);
            rows.borrow_mut().push(row);
            if let Some(cb) = on_change.borrow().as_ref() {
                cb();
            }
        });
    }

    root.append(&container);
    root.append(&add);
    Editor::Keybinds {
        root,
        rows,
        on_change,
        list,
    }
}

fn wire_remove(
    container: &gtk::Box,
    rows: &Rc<RefCell<Vec<KeybindRow>>>,
    on_change: &ChangeHook,
    row: &KeybindRow,
) {
    let container = container.clone();
    let rows = rows.clone();
    let on_change = on_change.clone();
    let row_box = row.row_box.clone();
    let popup = row.popup.clone();
    let id = row.id;
    row.remove.connect_clicked(move |_| {
        popup.destroy();
        container.remove(&row_box);
        sync_keybind_separators(&container);
        rows.borrow_mut().retain(|row| row.id != id);
        if let Some(cb) = on_change.borrow().as_ref() {
            cb();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_comma_separated_lists() {
        assert_eq!(csv_list("SUPER, SHIFT"), ["SUPER", "SHIFT"]);
        assert_eq!(csv_list(" , , "), [] as [&str; 0]);
        assert_eq!(csv_list(""), [] as [&str; 0]);
    }

    #[test]
    fn previews_modifier_plus_keys() {
        assert_eq!(
            bind_preview("SUPER, SHIFT", "Return, a"),
            "SUPER+SHIFT+RETURN+A"
        );
        assert_eq!(bind_preview("", ""), "");
    }

    #[test]
    fn reads_list_text_from_nix() {
        assert_eq!(list_text("[ \"SUPER\" \"SHIFT\" ]"), "SUPER, SHIFT");
        assert_eq!(list_text("[]"), "");
        assert_eq!(list_text("null"), "");
    }

    #[test]
    fn compares_entries_to_list_sources() {
        assert!(csv_matches_list_source(
            "SUPER, SHIFT",
            "[ \"SUPER\" \"SHIFT\" ]"
        ));
        assert!(csv_matches_list_source("", "[]"));
        assert!(!csv_matches_list_source("SUPER", "[ \"SUPER\" \"SHIFT\" ]"));
        assert!(!csv_matches_list_source("SUPER", "[]"));
    }

    #[test]
    fn serialises_csv_lists_as_nix_lists() {
        assert_eq!(csv_list_nix("SUPER, SHIFT"), "[ \"SUPER\" \"SHIFT\" ]");
        assert_eq!(csv_list_nix(""), "[]");
    }
    #[test]
    fn keybind_labels_derive_from_names_and_overrides() {
        assert_eq!(bind_display_label("", None), "New bind:");
        assert_eq!(bind_display_label("focusDown", None), "Focus Down:");
        assert_eq!(
            bind_display_label("mute", Some("Mute Audio")),
            "Mute Audio:"
        );
        assert_eq!(
            bind_display_label("mute", Some("Mute Audio:")),
            "Mute Audio:"
        );
        assert_eq!(bind_display_label("mute", Some("")), "Mute:");
    }

    #[test]
    fn default_schema_matches_the_historical_shape() {
        let schema = BindSchema::from_spec(&OptionSpec::default());
        let names: Vec<&str> = schema
            .fields
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "modifier",
                "keys",
                "dispatcher",
                "args",
                "description",
                "group",
                "subcat"
            ]
        );
        assert_eq!(schema.preview, ["modifier", "keys"]);
        assert_eq!(schema.sort, ["group", "subcat"]);
    }

    #[test]
    fn custom_schema_uses_declared_types() {
        let spec = OptionSpec {
            bind_fields: Some(vec!["key".into(), "command".into(), "repeat".into()]),
            bind_field_types: Some(vec!["list".into(), "str".into(), "bool".into()]),
            bind_preview: Some(vec!["key".into()]),
            bind_sort: Some(Vec::new()),
            ..OptionSpec::default()
        };
        let schema = BindSchema::from_spec(&spec);
        assert_eq!(schema.kind("key"), Some(BindFieldKind::List));
        assert_eq!(schema.kind("command"), Some(BindFieldKind::Text));
        assert_eq!(schema.kind("repeat"), Some(BindFieldKind::Bool));
        assert!(schema.sort.is_empty());
    }

    #[test]
    fn keeps_unknown_fields_out_of_the_known_set() {
        let schema = BindSchema::from_spec(&OptionSpec::default());
        let scan_row = scan::TableRowScan {
            key: "reload".into(),
            fields: vec![
                ("modifier".into(), "[ \"SUPER\" ]".into()),
                ("keys".into(), "[ \"r\" ]".into()),
                ("action".into(), "\"reload\"".into()),
            ],
            spec: OptionSpec::default(),
        };
        assert_eq!(extras_text(&scan_row, &schema), "action = \"reload\";");
    }
}
