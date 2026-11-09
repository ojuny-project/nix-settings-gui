use super::basic::build_list;
use super::basic::build_raw;
use super::context::ChangeHook;
use super::editor::Editor;
use super::nix::{attr_key, entry_text, parse_string_literal, quote, truthy};
use crate::dsl::OptionSpec;
use crate::model::Record;
use crate::scan;
use crate::value;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ContainerFields {
    pub name: String,
    pub image: String,
    pub ports: Vec<String>,
    pub volumes: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub auto_start: bool,
    pub extras: String,
}

#[derive(Clone)]
pub(crate) struct EnvRow {
    key: gtk::Entry,
    value: gtk::Entry,
    row: gtk::Box,
}

#[derive(Clone)]
pub(crate) struct DockerRow {
    pub(crate) key: gtk::Entry,
    pub(crate) image: gtk::Entry,
    pub(crate) ports: Editor,
    pub(crate) volumes: Editor,
    pub(crate) environment: Rc<RefCell<Vec<EnvRow>>>,
    pub(crate) auto_start: gtk::Switch,
    pub(crate) extras: gtk::TextView,
    pub(crate) remove: gtk::Button,
    pub(crate) expander: gtk::Expander,
    pub(crate) orig: ContainerFields,
}

impl DockerRow {
    fn read_fields(&self) -> ContainerFields {
        ContainerFields {
            name: self.key.text().trim().to_string(),
            image: self.image.text().to_string(),
            ports: list_values(&self.ports),
            volumes: list_values(&self.volumes),
            environment: self
                .environment
                .borrow()
                .iter()
                .map(|row| {
                    (
                        row.key.text().trim().to_string(),
                        row.value.text().to_string(),
                    )
                })
                .filter(|(key, _)| !key.is_empty())
                .collect(),
            auto_start: self.auto_start.is_active(),
            extras: text_of(&self.extras),
        }
    }
}

impl ContainerFields {
    fn new(name: String) -> Self {
        Self {
            name,
            auto_start: true,
            ..Self::default()
        }
    }
}

fn list_values(editor: &Editor) -> Vec<String> {
    let Editor::List { entries, .. } = editor else {
        return Vec::new();
    };
    entries
        .borrow()
        .iter()
        .map(|entry| parse_string_literal(&entry.text()))
        .filter(|item| !item.is_empty())
        .collect()
}

fn text_of(view: &gtk::TextView) -> String {
    let buffer = view.buffer();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string()
}

fn string_list(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|item| quote(item)).collect();
    format!("[ {} ]", quoted.join(" "))
}

fn dotted_key(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| attr_key(segment))
        .collect::<Vec<_>>()
        .join(".")
}

pub(crate) fn scan_containers(value_text: &str) -> Vec<ContainerFields> {
    scan::scan_value_direct(value_text)
        .into_iter()
        .filter(|record| record.segments.len() == 1)
        .map(|record| scan_container(&record.segments[0], &record.value))
        .collect()
}

fn scan_container(name: &str, value_text: &str) -> ContainerFields {
    let mut fields = ContainerFields {
        name: name.to_string(),
        auto_start: true,
        ..ContainerFields::default()
    };
    if value::classify(value_text) != value::ValueKind::AttrSet {
        fields.extras = value_text.trim().to_string();
        return fields;
    }
    for record in scan::scan_value_direct(value_text) {
        match record.segments.join(".").as_str() {
            "image" => fields.image = entry_text(&record.value),
            "ports" => fields.ports = parse_string_list(&record.value),
            "volumes" => fields.volumes = parse_string_list(&record.value),
            "environment" => fields.environment = scan_environment(&record.value),
            "autoStart" => fields.auto_start = truthy(&record.value),
            _ => fields.extras.push_str(&format!(
                "{} = {}; ",
                dotted_key(&record.segments),
                record.value.trim()
            )),
        }
    }
    fields.extras = fields.extras.trim().to_string();
    fields
}

fn scan_environment(value_text: &str) -> Vec<(String, String)> {
    if value::classify(value_text) != value::ValueKind::AttrSet {
        return Vec::new();
    }
    scan::scan_value_direct(value_text)
        .into_iter()
        .map(|record| {
            (
                dotted_key(&record.segments),
                parse_string_literal(&record.value),
            )
        })
        .collect()
}

fn parse_string_list(value_text: &str) -> Vec<String> {
    value::list_items(value_text)
        .unwrap_or_default()
        .into_iter()
        .map(|item| parse_string_literal(&item))
        .filter(|item| !item.is_empty())
        .collect()
}

/// Render the whole editor back to a container attribute set.
pub(crate) fn serialize_containers(containers: &[ContainerFields]) -> String {
    let mut entries = Vec::new();
    for container in containers {
        let mut fields = vec![format!("image = {};", quote(&container.image))];
        fields.push(format!("ports = {};", string_list(&container.ports)));
        fields.push(format!("volumes = {};", string_list(&container.volumes)));
        let environment: Vec<String> = container
            .environment
            .iter()
            .map(|(key, value)| format!("{} = {};", attr_key(key), quote(value)))
            .collect();
        fields.push(format!("environment = {{ {} }};", environment.join(" ")));
        fields.push(format!("autoStart = {};", container.auto_start));
        let extras = container.extras.trim();
        if !extras.is_empty() {
            fields.push(extras.to_string());
        }
        entries.push(format!(
            "{} = {{ {} }};",
            attr_key(container.name.trim()),
            fields.join(" ")
        ));
    }
    format!("{{ {} }}", entries.join(" "))
}

pub(crate) fn docker_nix(editor: &Editor) -> String {
    let Editor::Docker { rows, .. } = editor else {
        return "{ }".to_string();
    };
    let containers: Vec<ContainerFields> =
        rows.borrow().iter().map(DockerRow::read_fields).collect();
    serialize_containers(&containers)
}

pub(crate) fn docker_unchanged(rows: &Rc<RefCell<Vec<DockerRow>>>, orig_count: usize) -> bool {
    let rows = rows.borrow();
    rows.len() == orig_count && rows.iter().all(|row| row.read_fields() == row.orig)
}

/// Build the container list editor. Every container is an expander with its
/// image, port, volume, environment, and auto-start fields.
pub(crate) fn build_docker(record: &Record) -> Editor {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_hexpand(true);

    let hint = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .label("Manage the declarative containers. Each container writes image, ports, volumes, environment, and autoStart.")
        .css_classes(["dim-label", "caption"])
        .build();
    root.append(&hint);

    let rows_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    rows_box.set_hexpand(true);
    root.append(&rows_box);

    let rows: Rc<RefCell<Vec<DockerRow>>> = Rc::new(RefCell::new(Vec::new()));
    let on_change: ChangeHook = Rc::new(RefCell::new(None));
    let notify = super::submodule::hook_notify(&on_change);

    let scans = scan_containers(&record.value);
    let orig_count = scans.len();
    for scan in scans {
        let row = make_docker_row(scan, &rows_box, &rows, &notify);
        rows_box.append(&row.expander);
        rows.borrow_mut().push(row);
    }

    let add = gtk::Button::with_label("Add container");
    add.set_halign(gtk::Align::Start);
    {
        let rows_box = rows_box.clone();
        let rows = rows.clone();
        let notify = notify.clone();
        add.connect_clicked(move |_| {
            let name = unique_container_name(&rows.borrow());
            let row = make_docker_row(ContainerFields::new(name), &rows_box, &rows, &notify);
            rows_box.append(&row.expander);
            rows.borrow_mut().push(row);
            notify();
        });
    }
    root.append(&add);

    Editor::Docker {
        root,
        rows,
        on_change,
        orig_count,
    }
}

fn unique_container_name(rows: &[DockerRow]) -> String {
    let names: Vec<String> = rows
        .iter()
        .map(|row| row.key.text().trim().to_string())
        .collect();
    let mut index = 1;
    loop {
        let candidate = if index == 1 {
            "container".to_string()
        } else {
            format!("container{index}")
        };
        if !names.iter().any(|name| name == &candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn make_docker_row(
    fields: ContainerFields,
    rows_box: &gtk::Box,
    rows: &Rc<RefCell<Vec<DockerRow>>>,
    notify: &Rc<dyn Fn()>,
) -> DockerRow {
    let orig = fields.clone();
    let expander = gtk::Expander::new(Some(&fields.name));
    expander.set_expanded(true);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);

    let key = gtk::Entry::new();
    key.set_text(&fields.name);
    key.set_hexpand(true);
    let remove = gtk::Button::from_icon_name("user-trash-symbolic");
    remove.add_css_class("flat");
    remove.set_tooltip_text(Some("Remove container"));
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    header.append(&key);
    header.append(&remove);
    content.append(&header);

    {
        let expander = expander.clone();
        let notify = notify.clone();
        key.connect_changed(move |entry| {
            expander.set_label(Some(&entry.text()));
            notify();
        });
    }

    let image = gtk::Entry::new();
    image.set_text(&fields.image);
    image.set_hexpand(true);
    content.append(&field_block("Image", &image));

    let ports = build_list(&string_list(&fields.ports), false, None);
    ports.connect_changed(notify);
    content.append(&field_block("Ports", &ports.root()));

    let volumes = build_list(&string_list(&fields.volumes), false, None);
    volumes.connect_changed(notify);
    content.append(&field_block("Volumes", &volumes.root()));

    let environment: Rc<RefCell<Vec<EnvRow>>> = Rc::new(RefCell::new(Vec::new()));
    let environment_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    for (name, value) in &fields.environment {
        let row = env_row(name, value, &environment_box, &environment, notify);
        environment_box.append(&row.row);
        environment.borrow_mut().push(row);
    }
    let add_environment = gtk::Button::with_label("Add environment variable");
    add_environment.set_halign(gtk::Align::Start);
    {
        let environment_box = environment_box.clone();
        let environment = environment.clone();
        let notify = notify.clone();
        add_environment.connect_clicked(move |_| {
            let row = env_row("", "", &environment_box, &environment, &notify);
            environment_box.append(&row.row);
            environment.borrow_mut().push(row);
            notify();
        });
    }
    environment_box.append(&add_environment);
    content.append(&field_block("Environment", &environment_box));

    let auto_start = gtk::Switch::new();
    auto_start.set_active(fields.auto_start);
    auto_start.set_halign(gtk::Align::Start);
    content.append(&field_block("Start at boot", &auto_start));

    let (extras, extras_root) = build_raw(
        &fields.extras,
        &OptionSpec {
            rows: Some(3),
            ..OptionSpec::default()
        },
    );
    content.append(&field_block("Extra options", &extras_root));

    expander.set_child(Some(&content));

    let row = DockerRow {
        key,
        image,
        ports,
        volumes,
        environment,
        auto_start,
        extras,
        remove,
        expander,
        orig,
    };
    connect_docker_row(&row, notify);

    {
        let rows = rows.clone();
        let rows_box = rows_box.clone();
        let notify = notify.clone();
        let key = row.key.clone();
        let expander = row.expander.clone();
        row.remove.connect_clicked(move |_| {
            rows_box.remove(&expander);
            rows.borrow_mut().retain(|row| row.key != key);
            notify();
        });
    }

    row
}

fn env_row(
    name: &str,
    initial_value: &str,
    environment_box: &gtk::Box,
    environment: &Rc<RefCell<Vec<EnvRow>>>,
    notify: &Rc<dyn Fn()>,
) -> EnvRow {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let key = gtk::Entry::new();
    key.set_text(name);
    key.set_width_chars(20);
    key.set_placeholder_text(Some("NAME"));
    let value = gtk::Entry::new();
    value.set_text(initial_value);
    value.set_hexpand(true);
    value.set_placeholder_text(Some("value"));
    let remove = gtk::Button::with_label("Remove");
    row.append(&key);
    row.append(&value);
    row.append(&remove);
    for entry in [&key, &value] {
        let notify = notify.clone();
        entry.connect_changed(move |_| notify());
    }
    {
        let environment_box = environment_box.clone();
        let environment = environment.clone();
        let notify = notify.clone();
        let row_widget = row.clone();
        let key = key.clone();
        remove.connect_clicked(move |_| {
            environment_box.remove(&row_widget);
            environment.borrow_mut().retain(|row| row.key != key);
            notify();
        });
    }
    EnvRow { key, value, row }
}

fn connect_docker_row(row: &DockerRow, notify: &Rc<dyn Fn()>) {
    for entry in [&row.image] {
        let notify = notify.clone();
        entry.connect_changed(move |_| notify());
    }
    {
        let notify = notify.clone();
        row.auto_start.connect_active_notify(move |_| notify());
    }
    {
        let notify = notify.clone();
        row.extras.buffer().connect_changed(move |_| notify());
    }
}

fn field_block(label: &str, child: &impl IsA<gtk::Widget>) -> gtk::Box {
    let block = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let caption = gtk::Label::builder()
        .label(label)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    block.append(&caption);
    block.append(child);
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        web = {
            image = "nginx:latest";
            ports = [ "127.0.0.1:8080:80" ];
            volumes = [ "html:/usr/share/nginx/html" ];
            environment = { TZ = "Europe/Amsterdam"; };
            autoStart = true;
        };
        db = {
            image = "postgres:16";
            ports = [ ];
            volumes = [ "pgdata:/var/lib/postgresql/data" ];
            environment = { POSTGRES_DB = "app"; };
            autoStart = false;
        };
    }"#;

    #[test]
    fn scans_container_fields() {
        let containers = scan_containers(SAMPLE);
        assert_eq!(containers.len(), 2);
        assert_eq!(containers[0].name, "web");
        assert_eq!(containers[0].image, "nginx:latest");
        assert_eq!(containers[0].ports, vec!["127.0.0.1:8080:80"]);
        assert_eq!(containers[0].volumes, vec!["html:/usr/share/nginx/html"]);
        assert_eq!(
            containers[0].environment,
            vec![("TZ".to_string(), "Europe/Amsterdam".to_string())]
        );
        assert!(containers[0].auto_start);
        assert!(!containers[1].auto_start);
    }

    #[test]
    fn serializes_and_rescans_the_same_containers() {
        let containers = scan_containers(SAMPLE);
        let nix = serialize_containers(&containers);
        assert!(nix.contains("web = {"));
        assert!(nix.contains("image = \"nginx:latest\";"));
        assert!(nix.contains("ports = [ \"127.0.0.1:8080:80\" ];"));
        assert!(nix.contains("environment = { TZ = \"Europe/Amsterdam\"; };"));
        assert!(nix.contains("autoStart = false;"));
        let round_trip = scan_containers(&nix);
        assert_eq!(round_trip, containers);
    }

    #[test]
    fn keeps_unknown_container_fields() {
        let containers = scan_containers("{ web = { image = \"nginx\"; user = \"1000:1000\"; }; }");
        assert!(containers[0].extras.contains("user = \"1000:1000\";"));
        let nix = serialize_containers(&containers);
        assert!(nix.contains("user = \"1000:1000\";"));
    }
}
