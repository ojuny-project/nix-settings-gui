//! The `mounts` control: a mount table, an add row, and a disk overview tree.

use super::context::{BuildContext, ChangeHook, TerminalRun};
use super::editor::Editor;
use super::nix::unwrap_modifiers;
use super::submodule::hook_notify;
use crate::mounts::{
    detect, fs, migrate, paths, scan_file_systems, serialize_file_systems, ExtraField, Identity,
    MountRow,
};
use crate::model::Record;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const COLUMNS: [&str; 4] = ["Mount location", "Drive name", "Drive UUID", "Options"];

#[derive(Clone)]
struct MountsUi {
    rows: Rc<RefCell<Vec<MountRow>>>,
    table: gtk::ListBox,
    names: Rc<RefCell<Vec<(String, String)>>>,
    notify: Rc<dyn Fn()>,
    terminal: Option<TerminalRun>,
}

impl MountsUi {
    fn rebuild(&self) {
        while let Some(child) = self.table.first_child() {
            self.table.remove(&child);
        }
        self.table.append(&header_row());
        let rows = self.rows.borrow().clone();
        for row in rows {
            let item = self.row_widget(row);
            self.table.append(&item);
        }
    }

    fn row_widget(&self, row: MountRow) -> gtk::ListBoxRow {
        let item = gtk::ListBoxRow::new();
        item.set_activatable(false);
        item.set_selectable(false);
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        line.set_margin_top(4);
        line.set_margin_bottom(4);

        let name = display_name(&row, &self.names.borrow());
        let uuid = row.uuid().unwrap_or("-").to_string();
        let options = row.options.join(" ");
        for text in [&row.mountpoint, &name, &uuid, &options] {
            let label = gtk::Label::new(Some(text));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_width_chars(18);
            line.append(&label);
        }

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        if self.can_migrate(&row) {
            let migrate_button = gtk::Button::with_label("Migrate");
            migrate_button.add_css_class("flat");
            let ui = self.clone();
            let target = row.clone();
            migrate_button.connect_clicked(move |_| ui.migrate(&target));
            actions.append(&migrate_button);
        }
        let edit = gtk::Button::with_label("Edit");
        edit.add_css_class("flat");
        {
            let ui = self.clone();
            let target = row.clone();
            edit.connect_clicked(move |_| ui.edit(target.clone()));
        }
        actions.append(&edit);
        let remove = gtk::Button::with_label("Remove");
        remove.add_css_class("flat");
        {
            let ui = self.clone();
            let target = row.mountpoint.clone();
            remove.connect_clicked(move |_| ui.remove(&target));
        }
        actions.append(&remove);
        line.append(&actions);
        item.set_child(Some(&line));
        item
    }

    fn add(&self, row: MountRow) {
        self.rows.borrow_mut().push(row);
        (self.notify)();
        self.rebuild();
    }

    fn edit_new(&self, row: MountRow) {
        let ui = self.clone();
        let existing: Vec<String> = self
            .rows
            .borrow()
            .iter()
            .map(|row| row.mountpoint.clone())
            .collect();
        edit_dialog(row, existing, move |updated| ui.add(updated));
    }

    fn edit(&self, row: MountRow) {
        let ui = self.clone();
        let original = row.mountpoint.clone();
        let existing: Vec<String> = self
            .rows
            .borrow()
            .iter()
            .filter(|row| row.mountpoint != original)
            .map(|row| row.mountpoint.clone())
            .collect();
        edit_dialog(row, existing, move |updated| {
            {
                let mut rows = ui.rows.borrow_mut();
                if let Some(slot) = rows.iter_mut().find(|row| row.mountpoint == original) {
                    *slot = updated;
                }
            }
            (ui.notify)();
            ui.rebuild();
        });
    }

    fn remove(&self, mountpoint: &str) {
        self.rows
            .borrow_mut()
            .retain(|row| row.mountpoint != mountpoint);
        (self.notify)();
        self.rebuild();
    }

    fn can_migrate(&self, row: &MountRow) -> bool {
        matches!(
            paths::check_target(&row.mountpoint),
            Ok(paths::TargetState::UserOwned)
        )
    }

    fn migrate(&self, row: &MountRow) {
        let Some(terminal) = self.terminal.clone() else {
            return;
        };
        let Some(device) = row.device.clone() else {
            return;
        };
        let script = migrate::script(&row.mountpoint, &device, &row.options);
        let _ = terminal(&[format!("sh -c {}", migrate::shell_quote(&script))]);
    }
}

pub(crate) fn build_mounts(record: &Record, ctx: &BuildContext) -> Editor {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_hexpand(true);

    let hint = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .label(
            "Internal mounts only. Devices are written as UUIDs. Core filesystems stay in the generated hardware module. External media is configured below the mount table, not here.",
        )
        .css_classes(["dim-label", "caption"])
        .build();
    root.append(&hint);

    let scanned = scan_file_systems(&unwrap_modifiers(&record.value));
    let rows = Rc::new(RefCell::new(scanned.clone()));
    let orig = Rc::new(RefCell::new(scanned));
    let on_change: ChangeHook = Rc::new(RefCell::new(None));
    let notify = hook_notify(&on_change);
    let names = Rc::new(RefCell::new(probe_names()));
    let table = gtk::ListBox::new();
    table.set_selection_mode(gtk::SelectionMode::None);
    let ui = MountsUi {
        rows: rows.clone(),
        table: table.clone(),
        names,
        notify,
        terminal: ctx.terminal_run.clone(),
    };
    ui.rebuild();

    let status = gtk::Label::new(None::<&str>);
    status.set_xalign(0.0);
    status.add_css_class("error");

    let partitions = Rc::new(RefCell::new(probe_partitions()));
    let labels = partition_labels(&partitions.borrow());
    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let combo = gtk::DropDown::from_strings(&refs);
    combo.set_hexpand(true);
    let mountpoint = gtk::Entry::new();
    mountpoint.set_hexpand(true);
    mountpoint.set_placeholder_text(Some("/mnt/<drive>"));
    {
        let partitions = partitions.clone();
        let mountpoint = mountpoint.clone();
        combo.connect_selected_notify(move |combo| {
            if let Some(candidate) = partitions.borrow().get(combo.selected() as usize) {
                mountpoint.set_text(&format!("/mnt/{}", candidate.name));
            }
        });
    }

    let add = gtk::Button::with_label("Add mount");
    {
        let ui = ui.clone();
        let partitions = partitions.clone();
        let combo = combo.clone();
        let mountpoint = mountpoint.clone();
        let status = status.clone();
        add.connect_clicked(move |_| {
            status.set_text("");
            let index = combo.selected() as usize;
            let Some(candidate) = partitions.borrow().get(index).cloned() else {
                status.set_text("Select a partition first.");
                return;
            };
            let mut row = row_from_candidate(&candidate);
            let target = mountpoint.text().trim().to_string();
            if !target.is_empty() {
                row.mountpoint = target;
            }
            if let Err(message) = validate_new(&row, &ui.rows.borrow()) {
                status.set_text(&message);
                return;
            }
            if fs::requirements(&candidate.fstype).is_empty() {
                ui.add(row);
            } else {
                ui.edit_new(row);
            }
        });
    }

    let automount = gtk::Button::with_label("Automount all");
    {
        let ui = ui.clone();
        let partitions = partitions.clone();
        let status = status.clone();
        automount.connect_clicked(move |_| {
            status.set_text("");
            let candidates = partitions.borrow().clone();
            for candidate in candidates {
                let row = row_from_candidate(&candidate);
                let valid = validate_new(&row, &ui.rows.borrow()).is_ok();
                if valid {
                    ui.add(row);
                }
            }
        });
    }

    let add_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    add_box.append(&combo);
    add_box.append(&mountpoint);
    add_box.append(&add);
    add_box.append(&automount);
    root.append(&add_box);

    let add_tmpfs = gtk::Button::with_label("Add tmpfs");
    {
        let ui = ui.clone();
        add_tmpfs.connect_clicked(move |_| {
            ui.edit_new(MountRow {
                fs_type: Some("tmpfs".to_string()),
                options_present: true,
                ..MountRow::default()
            });
        });
    }
    let add_bind = gtk::Button::with_label("Add bind mount");
    {
        let ui = ui.clone();
        add_bind.connect_clicked(move |_| {
            ui.edit_new(MountRow {
                fs_type: Some("none".to_string()),
                options: vec!["bind".to_string()],
                options_present: true,
                ..MountRow::default()
            });
        });
    }
    let synthetic_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    synthetic_box.append(&add_tmpfs);
    synthetic_box.append(&add_bind);
    root.append(&synthetic_box);

    root.append(&table);

    let overview_label = gtk::Label::new(Some("Unpartitioned drives"));
    overview_label.set_xalign(0.0);
    overview_label.add_css_class("dim-label");
    root.append(&overview_label);

    let tree = gtk::ListBox::new();
    tree.set_selection_mode(gtk::SelectionMode::Single);
    let entries: Rc<RefCell<Vec<OverviewSelection>>> = Rc::new(RefCell::new(Vec::new()));
    build_overview(&tree, &entries);
    root.append(&tree);

    let modify = gtk::Button::with_label("Modify");
    modify.set_halign(gtk::Align::Start);
    {
        let tree = tree.clone();
        let entries = entries.clone();
        let terminal = ctx.terminal_run.clone();
        let status = status.clone();
        modify.connect_clicked(move |_| {
            status.set_text("");
            let Some(row) = tree.selected_row() else {
                status.set_text("Select a disk or partition.");
                return;
            };
            let index = row.index() as usize;
            let Some(entry) = entries.borrow().get(index).cloned() else {
                return;
            };
            match entry {
                OverviewSelection::Disk { path, can_modify } => {
                    if !can_modify {
                        status.set_text("This disk has mounted partitions or no free space.");
                        return;
                    }
                    let Some(terminal) = terminal.clone() else {
                        status.set_text("Configure a terminal command first.");
                        return;
                    };
                    let _ = terminal(&[format!("cfdisk {path}")]);
                }
                OverviewSelection::Partition { path } => {
                    if detect::has_filesystem(&path) {
                        status.set_text("This partition already has a filesystem.");
                        return;
                    }
                    mkfs_dialog(&path, terminal.clone(), status.clone());
                }
            }
        });
    }
    root.append(&modify);
    root.append(&status);

    Editor::Mounts {
        root,
        rows,
        orig,
        on_change,
    }
}

#[derive(Clone)]
enum OverviewSelection {
    Disk { path: String, can_modify: bool },
    Partition { path: String },
}

fn build_overview(
    list: &gtk::ListBox,
    entries: &Rc<RefCell<Vec<OverviewSelection>>>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let Some(devices) = detect::probe() else {
        return;
    };
    for disk in detect::overview(&devices) {
        let has_unformatted = disk.partitions.iter().any(|part| {
            part.fstype.is_none()
                && !part.mounted
                && part.parttype.as_deref() != Some(detect::ESP_PARTTYPE)
        });
        let can_modify = disk.free > 0 && !disk.mounted_child;
        entries.borrow_mut().push(OverviewSelection::Disk {
            path: disk.path.clone(),
            can_modify,
        });
        let text = format!(
            "{}  {}  {}  {} free",
            disk.name,
            detect::format_size(disk.size),
            disk.model.as_deref().unwrap_or(""),
            detect::format_size(Some(disk.free))
        )
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
        list.append(&overview_row(&text, 0, can_modify || has_unformatted));
        for part in disk.partitions.iter().filter(|part| {
            part.fstype.is_none()
                && !part.mounted
                && part.parttype.as_deref() != Some(detect::ESP_PARTTYPE)
        }) {
            entries
                .borrow_mut()
                .push(OverviewSelection::Partition {
                    path: part.path.clone(),
                });
            let text = format!(
                "{}  {}  no filesystem",
                part.name,
                detect::format_size(part.size)
            );
            list.append(&overview_row(&text, 1, true));
        }
    }
}

fn overview_row(text: &str, indent: i32, selectable: bool) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_selectable(selectable);
    row.set_activatable(false);
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_margin_start(indent * 18);
    if !selectable {
        label.add_css_class("dim-label");
    }
    row.set_child(Some(&label));
    row
}

fn mkfs_dialog(path: &str, terminal: Option<TerminalRun>, status: gtk::Label) {
    let window = gtk::Window::builder()
        .title("Apply a filesystem")
        .modal(true)
        .resizable(false)
        .build();
    let content = dialog_box();
    let title = gtk::Label::new(Some(&format!("Format {path}")));
    title.set_xalign(0.0);
    content.append(&title);
    let note = gtk::Label::new(Some(
        "This erases the partition. A partition that already has a filesystem is refused.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    content.append(&note);
    let types = ["ext4", "btrfs", "xfs", "f2fs", "vfat", "exfat", "ntfs"];
    let combo = gtk::DropDown::from_strings(&types);
    content.append(&combo);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let format = gtk::Button::with_label("Format");
    format.add_css_class("destructive-action");
    buttons.append(&cancel);
    buttons.append(&format);
    content.append(&buttons);
    window.set_child(Some(&content));

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    {
        let window = window.clone();
        let path = path.to_string();
        format.connect_clicked(move |_| {
            if detect::has_filesystem(&path) {
                status.set_text("This partition already has a filesystem.");
                window.close();
                return;
            }
            let Some(terminal) = terminal.clone() else {
                status.set_text("Configure a terminal command first.");
                return;
            };
            let index = combo.selected() as usize;
            let Some(fs_type) = types.get(index).copied() else {
                return;
            };
            if let Some(command) = fs::mkfs_command(fs_type, &path) {
                let _ = terminal(&[command]);
            }
            window.close();
        });
    }
    window.present();
}

fn edit_dialog<F: Fn(MountRow) + 'static>(
    initial: MountRow,
    existing_mountpoints: Vec<String>,
    on_save: F,
) {
    let is_tmpfs = initial.fs_type.as_deref() == Some("tmpfs");
    let is_bind = initial.identity() == Identity::Bind;
    let requirements = fs::requirements(initial.fs_type.as_deref().unwrap_or(""));

    let window = gtk::Window::builder()
        .title(if initial.mountpoint.is_empty() {
            "Add mount"
        } else {
            "Edit mount"
        })
        .modal(true)
        .resizable(false)
        .build();
    let content = dialog_box();
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(6)
        .build();
    content.append(&grid);
    let mut row_index = 0;

    let mountpoint = gtk::Entry::new();
    mountpoint.set_text(&initial.mountpoint);
    mountpoint.set_hexpand(true);
    field(&grid, "Mount location", &mountpoint, row_index);
    row_index += 1;

    let source = gtk::Entry::new();
    source.set_text(initial.device.as_deref().unwrap_or(""));
    source.set_hexpand(true);
    if is_bind {
        field(&grid, "Source path", &source, row_index);
        row_index += 1;
    } else if !is_tmpfs {
        let device = gtk::Label::new(Some(initial.device.as_deref().unwrap_or("")));
        device.set_xalign(0.0);
        field(&grid, "Device", &device, row_index);
        row_index += 1;
    }

    let fs_label = gtk::Label::new(Some(initial.fs_type.as_deref().unwrap_or("")));
    fs_label.set_xalign(0.0);
    field(&grid, "Filesystem", &fs_label, row_index);
    row_index += 1;

    let choices = ["Choose...", "Read and write", "Read only", "None"];
    let owner = gtk::DropDown::from_strings(&choices);
    let group = gtk::DropDown::from_strings(&choices);
    let others = gtk::DropDown::from_strings(&choices);
    if let Some(mask) = initial
        .options
        .iter()
        .find_map(|option| option.strip_prefix("umask="))
        .and_then(|value| u32::from_str_radix(value, 8).ok())
    {
        let mode = 0o777 & !mask;
        owner.set_selected(permission_index((mode >> 6) & 7));
        group.set_selected(permission_index((mode >> 3) & 7));
        others.set_selected(permission_index(mode & 7));
    }
    if requirements.contains(&fs::Requirement::Owner) {
        field(&grid, "Owner", &owner, row_index);
        row_index += 1;
        field(&grid, "Group", &group, row_index);
        row_index += 1;
        field(&grid, "Everyone", &others, row_index);
        row_index += 1;
    }

    let driver = gtk::DropDown::from_strings(&["Choose...", "ntfs3", "ntfs-3g"]);
    if requirements.contains(&fs::Requirement::Driver) {
        driver.set_selected(match initial.fs_type.as_deref() {
            Some("ntfs3") => 1,
            Some("ntfs") => 2,
            _ => 0,
        });
        field(&grid, "Driver", &driver, row_index);
        row_index += 1;
    }

    let subvolume = gtk::Entry::new();
    subvolume.set_hexpand(true);
    if requirements.contains(&fs::Requirement::Subvolume) {
        subvolume.set_text(
            initial
                .options
                .iter()
                .find_map(|option| option.strip_prefix("subvol="))
                .unwrap_or("@data"),
        );
        field(&grid, "Subvolume", &subvolume, row_index);
        row_index += 1;
    }

    let size = gtk::Entry::new();
    size.set_hexpand(true);
    let mode = gtk::Entry::new();
    mode.set_hexpand(true);
    if is_tmpfs {
        size.set_text(
            initial
                .options
                .iter()
                .find_map(|option| option.strip_prefix("size="))
                .unwrap_or("1G"),
        );
        mode.set_text(
            initial
                .options
                .iter()
                .find_map(|option| option.strip_prefix("mode="))
                .unwrap_or("1777"),
        );
        field(&grid, "Size", &size, row_index);
        row_index += 1;
        field(&grid, "Mode", &mode, row_index);
        row_index += 1;
    }

    let nofail = gtk::Switch::new();
    nofail.set_active(
        initial.options.iter().any(|option| option == "nofail")
            || initial.mountpoint.is_empty(),
    );
    let read_only = gtk::Switch::new();
    read_only.set_active(initial.options.iter().any(|option| option == "ro"));
    let fsck = gtk::Switch::new();
    fsck.set_active(
        !initial
            .extras
            .iter()
            .any(|extra| extra.name == "fsck" && extra.value == "false"),
    );
    let automount = gtk::Switch::new();
    automount.set_active(
        initial
            .options
            .iter()
            .any(|option| option == "x-systemd.automount"),
    );
    for (label, switch) in [
        ("Nofail", &nofail),
        ("Read only", &read_only),
        ("fsck at boot", &fsck),
        ("Automount on access", &automount),
    ] {
        switch.set_halign(gtk::Align::Start);
        field(&grid, label, switch, row_index);
        row_index += 1;
    }

    let error = gtk::Label::new(None::<&str>);
    error.set_xalign(0.0);
    error.set_wrap(true);
    error.add_css_class("error");
    content.append(&error);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let save = gtk::Button::with_label("Save");
    save.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&save);
    content.append(&buttons);
    window.set_child(Some(&content));

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    {
        let window = window.clone();
        let initial = initial.clone();
        let existing = existing_mountpoints.clone();
        save.connect_clicked(move |_| {
            let target = mountpoint.text().trim().to_string();
            if target.is_empty() {
                error.set_text("A mount location is required.");
                return;
            }
            if let Err(message) = paths::check_target(&target) {
                error.set_text(&message);
                return;
            }
            if existing.iter().any(|known| known == &target) {
                error.set_text("That mount location is already used.");
                return;
            }
            let mut options: Vec<String> = initial
                .options
                .iter()
                .filter(|option| !is_managed_option(option))
                .cloned()
                .collect();
            if nofail.is_active() {
                options.push("nofail".to_string());
            }
            if read_only.is_active() {
                options.push("ro".to_string());
            }
            if automount.is_active() {
                options.push("x-systemd.automount".to_string());
            }
            let fs_type = if requirements.contains(&fs::Requirement::Driver) {
                match driver.selected() {
                    1 => Some(fs::ntfs_fs_type("ntfs3").to_string()),
                    2 => Some(fs::ntfs_fs_type("ntfs-3g").to_string()),
                    _ => {
                        error.set_text("Choose a driver.");
                        return;
                    }
                }
            } else {
                initial.fs_type.clone()
            };
            if requirements.contains(&fs::Requirement::Owner) {
                if owner.selected() == 0 || group.selected() == 0 || others.selected() == 0 {
                    error.set_text("Choose the owner, group and everyone permissions.");
                    return;
                }
                let bits = |index: u32| match index {
                    1 => 6,
                    2 => 4,
                    _ => 0,
                };
                let mode =
                    (bits(owner.selected()) << 6) | (bits(group.selected()) << 3) | bits(others.selected());
                let umask = 0o777 & !mode;
                // SAFETY: geteuid and getegid have no preconditions.
                options.push(format!("uid={}", unsafe { libc::geteuid() }));
                options.push(format!("gid={}", unsafe { libc::getegid() }));
                options.push(format!("umask={umask:04o}"));
            }
            if requirements.contains(&fs::Requirement::Subvolume) {
                let value = subvolume.text().trim().to_string();
                if value.is_empty() {
                    error.set_text("A subvolume is required.");
                    return;
                }
                options.push(format!("subvol={value}"));
            }
            if is_tmpfs {
                let size_value = size.text().trim().to_string();
                let mode_value = mode.text().trim().to_string();
                if size_value.is_empty() || mode_value.is_empty() {
                    error.set_text("Size and mode are required.");
                    return;
                }
                options.push(format!("size={size_value}"));
                options.push(format!("mode={mode_value}"));
            }
            let device = if is_bind {
                let value = source.text().trim().to_string();
                if value.is_empty() {
                    error.set_text("A source path is required.");
                    return;
                }
                Some(value)
            } else if is_tmpfs {
                None
            } else {
                initial.device.clone()
            };
            let mut extras = initial.extras.clone();
            extras.retain(|extra| extra.name != "fsck");
            if !fsck.is_active() {
                extras.push(ExtraField {
                    name: "fsck".to_string(),
                    value: "false".to_string(),
                });
            }
            let updated = MountRow {
                mountpoint: target,
                device,
                fs_type: if is_bind {
                    Some("none".to_string())
                } else {
                    fs_type
                },
                options_present: true,
                options,
                needed_for_boot: false,
                extras,
            };
            let missing =
                fs::missing_requirements(updated.fs_type.as_deref().unwrap_or(""), &updated.options);
            if !missing.is_empty() {
                error.set_text(&format!(
                    "Still required: {}.",
                    missing
                        .iter()
                        .map(|requirement| requirement.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                return;
            }
            on_save(updated);
            window.close();
        });
    }
    window.present();
}

fn dialog_box() -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content
}

fn field(grid: &gtk::Grid, label: &str, widget: &impl IsA<gtk::Widget>, row: i32) {
    let title = gtk::Label::new(Some(label));
    title.set_xalign(0.0);
    grid.attach(&title, 0, row, 1, 1);
    widget.set_hexpand(true);
    grid.attach(widget, 1, row, 1, 1);
}

fn permission_index(bits: u32) -> u32 {
    if bits & 6 == 6 {
        1
    } else if bits & 4 == 4 {
        2
    } else {
        3
    }
}

fn is_managed_option(option: &str) -> bool {
    if option == "x-systemd.automount" {
        return true;
    }
    let name = option.split('=').next().unwrap_or("");
    matches!(
        name,
        "nofail" | "ro" | "uid" | "gid" | "umask" | "fmask" | "dmask" | "subvol" | "subvolid" | "size" | "mode"
    )
}

fn header_row() -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    row.set_selectable(false);
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for text in COLUMNS {
        let label = gtk::Label::new(Some(text));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.add_css_class("dim-label");
        line.append(&label);
    }
    let actions = gtk::Label::new(Some(""));
    actions.set_width_chars(12);
    line.append(&actions);
    row.set_child(Some(&line));
    row
}

fn display_name(row: &MountRow, names: &[(String, String)]) -> String {
    match row.identity() {
        Identity::Tmpfs => "tmpfs".to_string(),
        Identity::Bind => "bind".to_string(),
        _ => match row.uuid() {
            Some(uuid) => names
                .iter()
                .find(|(known, _)| known == uuid)
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| uuid.to_string()),
            None => row
                .device
                .clone()
                .unwrap_or_else(|| "-".to_string()),
        },
    }
}

fn probe_names() -> Vec<(String, String)> {
    detect::probe()
        .map(|devices| detect::partition_names(&devices))
        .unwrap_or_default()
}

fn probe_partitions() -> Vec<detect::MountCandidate> {
    detect::probe()
        .map(|devices| detect::internal_candidates(&devices))
        .unwrap_or_default()
}

fn partition_labels(candidates: &[detect::MountCandidate]) -> Vec<String> {
    candidates
        .iter()
        .map(|candidate| {
            format!(
                "{}  {}  {}  {}",
                candidate.name,
                detect::format_size(candidate.size),
                candidate.fstype,
                candidate.model.as_deref().unwrap_or("")
            )
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
        })
        .collect()
}

fn row_from_candidate(candidate: &detect::MountCandidate) -> MountRow {
    let mut options: Vec<String> = fs::default_options(&candidate.fstype)
        .iter()
        .map(|option| option.to_string())
        .collect();
    options.push("nofail".to_string());
    let extras = if fs::fsck(&candidate.fstype) {
        Vec::new()
    } else {
        vec![ExtraField {
            name: "fsck".to_string(),
            value: "false".to_string(),
        }]
    };
    MountRow {
        mountpoint: format!("/mnt/{}", candidate.name),
        device: Some(
            candidate
                .uuid
                .as_ref()
                .map(|uuid| format!("/dev/disk/by-uuid/{uuid}"))
                .unwrap_or_else(|| candidate.path.clone()),
        ),
        fs_type: Some(candidate.fstype.clone()),
        options_present: true,
        options,
        needed_for_boot: false,
        extras,
    }
}

fn validate_new(row: &MountRow, rows: &[MountRow]) -> Result<(), String> {
    if row.mountpoint.is_empty() {
        return Err("A mount location is required.".to_string());
    }
    if let Err(message) = paths::check_target(&row.mountpoint) {
        return Err(message);
    }
    if rows.iter().any(|known| {
        known.mountpoint == row.mountpoint
            || (known.device.is_some() && known.device == row.device)
    }) {
        return Err("That drive is already mounted or the location is used.".to_string());
    }
    Ok(())
}

pub(crate) fn mounts_nix(editor: &Editor) -> String {
    let Editor::Mounts { rows, .. } = editor else {
        return "{ }".to_string();
    };
    serialize_file_systems(&rows.borrow())
}

pub(crate) fn mounts_unchanged(
    rows: &Rc<RefCell<Vec<MountRow>>>,
    orig: &Rc<RefCell<Vec<MountRow>>>,
) -> bool {
    *rows.borrow() == *orig.borrow()
}

pub(crate) fn mounts_validation_error(rows: &Rc<RefCell<Vec<MountRow>>>) -> Option<String> {
    let rows = rows.borrow();
    let mut seen: Vec<String> = Vec::new();
    for row in rows.iter() {
        if row.mountpoint.is_empty() {
            return Some("Every mount needs a mount location.".to_string());
        }
        let Some(normalized) = paths::normalize(&row.mountpoint) else {
            return Some(format!("{} is not an absolute path.", row.mountpoint));
        };
        if let Err(message) = paths::check_target(&normalized) {
            return Some(message);
        }
        if seen.contains(&normalized) {
            return Some(format!("{normalized} is used by more than one mount."));
        }
        seen.push(normalized);
        let missing = fs::missing_requirements(
            row.fs_type.as_deref().unwrap_or(""),
            &row.options,
        );
        if !missing.is_empty() {
            return Some(format!(
                "{} needs {}.",
                row.mountpoint,
                missing
                    .iter()
                    .map(|requirement| requirement.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    None
}
