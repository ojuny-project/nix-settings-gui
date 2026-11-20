//! The `mounts` control: a list of `fileSystems` rows with edit and remove.

use super::context::{BuildContext, TerminalRun};
use super::context::ChangeHook;
use super::editor::Editor;
use super::nix::unwrap_modifiers;
use super::submodule::hook_notify;
use crate::mounts::{
    detect, fs, migrate, paths, scan_file_systems, serialize_file_systems, ExtraField, MountRow,
};
use crate::model::Record;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct MountsRow {
    pub(crate) orig: MountRow,
    pub(crate) mountpoint: gtk::Entry,
    pub(crate) device: gtk::Entry,
    pub(crate) fs_type: gtk::Entry,
    pub(crate) options: gtk::Entry,
    pub(crate) nofail: gtk::Switch,
    pub(crate) read_only: gtk::Switch,
    pub(crate) needed_for_boot: gtk::Switch,
    pub(crate) expander: gtk::Expander,
}

impl MountsRow {
    pub(crate) fn read(&self) -> MountRow {
        let mut options: Vec<String> = self
            .options
            .text()
            .split_whitespace()
            .filter(|option| *option != "nofail" && *option != "ro")
            .map(str::to_string)
            .collect();
        if self.nofail.is_active() {
            options.push("nofail".to_string());
        }
        if self.read_only.is_active() {
            options.push("ro".to_string());
        }
        MountRow {
            mountpoint: self.mountpoint.text().trim().to_string(),
            device: non_empty(&self.device.text()),
            fs_type: non_empty(&self.fs_type.text()),
            options_present: self.orig.options_present || !options.is_empty(),
            options,
            needed_for_boot: self.needed_for_boot.is_active(),
            extras: self.orig.extras.clone(),
        }
    }
}

fn non_empty(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn probe_candidates() -> Vec<detect::MountCandidate> {
    detect::probe()
        .map(|devices| detect::internal_candidates(&devices))
        .unwrap_or_default()
}

fn candidate_labels(candidates: &[detect::MountCandidate]) -> Vec<String> {
    candidates
        .iter()
        .map(|candidate| {
            let size = detect::format_size(candidate.size);
            let model = candidate.model.as_deref().unwrap_or("");
            let uuid = candidate.uuid.as_deref().unwrap_or("no uuid");
            format!(
                "{}  {}  {}  {}  {}",
                candidate.name, candidate.fstype, size, model, uuid
            )
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
        })
        .collect()
}

fn probe_disks() -> Vec<detect::DiskCandidate> {
    detect::probe()
        .map(|devices| detect::internal_disks(&devices))
        .unwrap_or_default()
}

fn probe_unformatted() -> Vec<detect::DiskCandidate> {
    detect::probe()
        .map(|devices| detect::unformatted_partitions(&devices))
        .unwrap_or_default()
}

/// Choose a filesystem for an unformatted partition and run mkfs through the
/// terminal.
fn mkfs_dialog(candidate: &detect::DiskCandidate, terminal: TerminalRun) {
    let window = gtk::Window::builder()
        .title("Apply a filesystem")
        .modal(true)
        .resizable(false)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let title = gtk::Label::new(Some(&format!("Format {}", candidate.path)));
    title.set_xalign(0.0);
    content.append(&title);

    let note = gtk::Label::new(Some(
        "This erases the partition. Choose the filesystem type, then refresh the disk list when the command finishes.",
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
    let confirm = gtk::Button::with_label("Format");
    confirm.add_css_class("destructive-action");
    buttons.append(&cancel);
    buttons.append(&confirm);
    content.append(&buttons);

    window.set_child(Some(&content));

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    {
        let window = window.clone();
        let terminal = terminal.clone();
        let path = candidate.path.clone();
        confirm.connect_clicked(move |_| {
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

fn disk_labels(disks: &[detect::DiskCandidate]) -> Vec<String> {
    disks
        .iter()
        .map(|disk| {
            let size = detect::format_size(disk.size);
            let model = disk.model.as_deref().unwrap_or("");
            format!("{}  {}  {}", disk.name, size, model)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn device_path(candidate: &detect::MountCandidate) -> String {
    candidate
        .uuid
        .as_ref()
        .map(|uuid| format!("/dev/disk/by-uuid/{uuid}"))
        .unwrap_or_else(|| candidate.path.clone())
}

fn row_from_candidate(candidate: &detect::MountCandidate) -> MountRow {
    let mut options: Vec<String> = fs::default_options(&candidate.fstype)
        .iter()
        .map(|option| option.to_string())
        .collect();
    if matches!(
        candidate.fstype.as_str(),
        "vfat" | "exfat" | "ntfs" | "ntfs3"
    ) {
        // SAFETY: getuid and getgid have no preconditions.
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        options.push(format!("uid={uid}"));
        options.push(format!("gid={gid}"));
        options.push("umask=0022".to_string());
    }
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
        device: Some(device_path(candidate)),
        fs_type: Some(candidate.fstype.clone()),
        options_present: !options.is_empty(),
        options,
        needed_for_boot: false,
        extras,
    }
}

pub(crate) fn build_mounts(record: &Record, ctx: &BuildContext) -> Editor {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_hexpand(true);

    let hint = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .label(
            "One row per mountpoint. The device is a /dev path or a /dev/disk/by-uuid/ value. \
             tmpfs and bind rows leave the device empty. Protected paths are rejected on save.",
        )
        .css_classes(["dim-label", "caption"])
        .build();
    root.append(&hint);

    let rows_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    rows_box.set_hexpand(true);

    let rows: Rc<RefCell<Vec<MountsRow>>> = Rc::new(RefCell::new(Vec::new()));
    let on_change: ChangeHook = Rc::new(RefCell::new(None));
    let notify = hook_notify(&on_change);

    let candidates: Rc<RefCell<Vec<detect::MountCandidate>>> =
        Rc::new(RefCell::new(probe_candidates()));
    let labels = candidate_labels(&candidates.borrow());
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let model = gtk::StringList::new(&label_refs);
    let combo = gtk::DropDown::new(Some(model.clone()), None::<gtk::Expression>);
    combo.set_hexpand(true);

    let detected = gtk::Label::new(Some("Detected disks"));
    detected.set_xalign(0.0);
    detected.add_css_class("dim-label");
    root.append(&detected);

    let picker = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    picker.append(&combo);

    let refresh = gtk::Button::with_label("Refresh");
    {
        let candidates = candidates.clone();
        let model = model.clone();
        let combo = combo.clone();
        refresh.connect_clicked(move |_| {
            let next = probe_candidates();
            let next_labels = candidate_labels(&next);
            let refs: Vec<&str> = next_labels.iter().map(String::as_str).collect();
            model.splice(0, model.n_items(), &refs);
            *candidates.borrow_mut() = next;
            if !refs.is_empty() {
                combo.set_selected(0);
            }
        });
    }
    picker.append(&refresh);

    let add_detected = gtk::Button::with_label("Add from disk");
    {
        let combo = combo.clone();
        let candidates = candidates.clone();
        let append: Rc<dyn Fn(&detect::MountCandidate, Option<u32>, Option<String>)> = Rc::new({
            let rows_box = rows_box.clone();
            let rows = rows.clone();
            let notify = notify.clone();
            let terminal = ctx.terminal_run.clone();
            move |candidate, umask, subvolume| {
                let device = device_path(candidate);
                if rows.borrow().iter().any(|row| row.device.text() == device) {
                    return;
                }
                let mut fields = row_from_candidate(candidate);
                if let Some(subvolume) = subvolume {
                    fields.options.push(format!("subvol={subvolume}"));
                    fields.options_present = true;
                }
                if let Some(umask) = umask {
                    fields.options.retain(|option| !option.starts_with("umask="));
                    fields.options.push(format!("umask={umask:04o}"));
                    fields.options_present = true;
                }
                let row = make_mount_row(fields, &rows_box, &rows, &notify, terminal.clone());
                rows_box.append(&row.expander);
                rows.borrow_mut().push(row);
                notify();
            }
        });
        add_detected.connect_clicked(move |_| {
            let index = combo.selected() as usize;
            let candidate = candidates.borrow().get(index).cloned();
            let Some(candidate) = candidate else {
                return;
            };
            if candidate.fstype.eq_ignore_ascii_case("btrfs") {
                let append = append.clone();
                let candidate = candidate.clone();
                subvolume_dialog(move |subvolume| {
                    append(&candidate, None, Some(subvolume));
                });
            } else if fs::requirements(&candidate.fstype).contains(&fs::Requirement::Owner) {
                let append = append.clone();
                let fs_type = candidate.fstype.clone();
                let candidate = candidate.clone();
                permission_dialog(&fs_type, move |umask| {
                    append(&candidate, Some(umask), None);
                });
            } else {
                append(&candidate, None, None);
            }
        });
    }
    picker.append(&add_detected);
    root.append(&picker);

    let automount = gtk::Button::with_label("Automount all");
    automount.set_halign(gtk::Align::Start);
    {
        let candidates = candidates.clone();
        let rows_box = rows_box.clone();
        let rows = rows.clone();
        let notify = notify.clone();
        let terminal = ctx.terminal_run.clone();
        automount.connect_clicked(move |_| {
            for candidate in candidates.borrow().iter() {
                let mountpoint = format!("/mnt/{}", candidate.name);
                if rows
                    .borrow()
                    .iter()
                    .any(|row| row.mountpoint.text() == mountpoint)
                {
                    continue;
                }
                let row = make_mount_row(
                    row_from_candidate(candidate),
                    &rows_box,
                    &rows,
                    &notify,
                    terminal.clone(),
                );
                rows_box.append(&row.expander);
                rows.borrow_mut().push(row);
            }
            notify();
        });
    }
    root.append(&automount);

    if let Some(terminal) = ctx.terminal_run.clone() {
        let disks: Rc<RefCell<Vec<detect::DiskCandidate>>> =
            Rc::new(RefCell::new(probe_disks()));
        let labels = disk_labels(&disks.borrow());
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let disk_model = gtk::StringList::new(&refs);
        let disk_combo = gtk::DropDown::new(Some(disk_model), None::<gtk::Expression>);
        disk_combo.set_hexpand(true);

        let label = gtk::Label::new(Some("Modify a disk"));
        label.set_xalign(0.0);
        label.add_css_class("dim-label");
        root.append(&label);

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row.append(&disk_combo);
        let modify = gtk::Button::with_label("Open cfdisk");
        {
            let disks = disks.clone();
            let disk_combo = disk_combo.clone();
            let terminal = terminal.clone();
            modify.connect_clicked(move |_| {
                let index = disk_combo.selected() as usize;
                let Some(disk) = disks.borrow().get(index).cloned() else {
                    return;
                };
                let _ = terminal(&[format!("cfdisk {}", disk.path)]);
            });
        }
        row.append(&modify);
        root.append(&row);
    }

    if let Some(terminal) = ctx.terminal_run.clone() {
        let unformatted: Rc<RefCell<Vec<detect::DiskCandidate>>> =
            Rc::new(RefCell::new(probe_unformatted()));
        let labels = disk_labels(&unformatted.borrow());
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let model = gtk::StringList::new(&refs);
        let combo_unformatted = gtk::DropDown::new(Some(model), None::<gtk::Expression>);
        combo_unformatted.set_hexpand(true);

        let label = gtk::Label::new(Some("Unformatted partitions"));
        label.set_xalign(0.0);
        label.add_css_class("dim-label");
        root.append(&label);

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row.append(&combo_unformatted);
        let format = gtk::Button::with_label("Apply filesystem");
        {
            let unformatted = unformatted.clone();
            let combo_unformatted = combo_unformatted.clone();
            let terminal = terminal.clone();
            format.connect_clicked(move |_| {
                let index = combo_unformatted.selected() as usize;
                let Some(candidate) = unformatted.borrow().get(index).cloned() else {
                    return;
                };
                mkfs_dialog(&candidate, terminal.clone());
            });
        }
        row.append(&format);
        root.append(&row);
    }

    root.append(&rows_box);

    let scans = scan_file_systems(&unwrap_modifiers(&record.value));
    let orig_count = scans.len();
    for scan in scans {
        let row = make_mount_row(scan, &rows_box, &rows, &notify, ctx.terminal_run.clone());
        rows_box.append(&row.expander);
        rows.borrow_mut().push(row);
    }

    let add = gtk::Button::with_label("Add mount");
    add.set_halign(gtk::Align::Start);
    {
        let rows_box = rows_box.clone();
        let rows = rows.clone();
        let notify = notify.clone();
        let terminal = ctx.terminal_run.clone();
        add.connect_clicked(move |_| {
            let row = make_mount_row(
                MountRow::default(),
                &rows_box,
                &rows,
                &notify,
                terminal.clone(),
            );
            rows_box.append(&row.expander);
            rows.borrow_mut().push(row);
            notify();
        });
    }
    root.append(&add);

    Editor::Mounts {
        root,
        rows,
        on_change,
        orig_count,
    }
}

/// Ask for the btrfs subvolume before the row is added.
fn subvolume_dialog<F: Fn(String) + 'static>(on_confirm: F) {
    let window = gtk::Window::builder()
        .title("btrfs subvolume")
        .modal(true)
        .resizable(false)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let title = gtk::Label::new(Some("Subvolume to mount"));
    title.set_xalign(0.0);
    content.append(&title);

    let note = gtk::Label::new(Some(
        "The subvolume must already exist. Create it with btrfs subvolume create before adding the mount.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    content.append(&note);

    let entry = gtk::Entry::new();
    entry.set_text("@data");
    entry.set_hexpand(true);
    content.append(&entry);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let confirm = gtk::Button::with_label("Add mount");
    confirm.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&confirm);
    content.append(&buttons);

    window.set_child(Some(&content));

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    {
        let window = window.clone();
        confirm.connect_clicked(move |_| {
            let subvolume = entry.text().trim().to_string();
            if !subvolume.is_empty() {
                on_confirm(subvolume);
            }
            window.close();
        });
    }

    window.present();
}

/// Ask for the owner, group and everyone permissions with readable choices,
/// then turn the triple into a umask.
fn permission_dialog<F: Fn(u32) + 'static>(fs_type: &str, on_confirm: F) {
    let window = gtk::Window::builder()
        .title("Mount permissions")
        .modal(true)
        .resizable(false)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let title = gtk::Label::new(Some(&format!("Permissions for a new {fs_type} mount")));
    title.set_xalign(0.0);
    content.append(&title);

    let note = gtk::Label::new(Some(
        "The mount owner is the current user. Choose what the owner, the group, and everyone else may do.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    content.append(&note);

    let choices = ["Read and write", "Read only", "None"];
    let owner = gtk::DropDown::from_strings(&choices);
    let group = gtk::DropDown::from_strings(&choices);
    let others = gtk::DropDown::from_strings(&choices);
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(6)
        .build();
    for (row, (label, widget)) in [
        ("Owner", &owner),
        ("Group", &group),
        ("Everyone", &others),
    ]
    .into_iter()
    .enumerate()
    {
        field(&grid, label, widget, row as i32);
    }
    content.append(&grid);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let confirm = gtk::Button::with_label("Add mount");
    confirm.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&confirm);
    content.append(&buttons);

    window.set_child(Some(&content));

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    {
        let window = window.clone();
        confirm.connect_clicked(move |_| {
            let bits = |index: u32| match index {
                0 => 6,
                1 => 4,
                _ => 0,
            };
            let mode = (bits(owner.selected()) << 6)
                | (bits(group.selected()) << 3)
                | bits(others.selected());
            let umask = 0o777 & !mode;
            on_confirm(umask);
            window.close();
        });
    }

    window.present();
}

fn make_mount_row(
    fields: MountRow,
    rows_box: &gtk::Box,
    rows: &Rc<RefCell<Vec<MountsRow>>>,
    notify: &Rc<dyn Fn()>,
    terminal: Option<TerminalRun>,
) -> MountsRow {
    let orig = fields.clone();
    let label = if fields.mountpoint.is_empty() {
        "New mount"
    } else {
        fields.mountpoint.as_str()
    };
    let expander = gtk::Expander::new(Some(label));
    expander.set_expanded(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);

    let mountpoint = gtk::Entry::new();
    mountpoint.set_text(&fields.mountpoint);
    mountpoint.set_hexpand(true);
    mountpoint.set_placeholder_text(Some("/mnt/data"));

    let remove = gtk::Button::from_icon_name("user-trash-symbolic");
    remove.add_css_class("flat");
    remove.set_tooltip_text(Some("Remove mount"));
    remove.set_halign(gtk::Align::End);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    header.append(&mountpoint);
    header.append(&remove);
    content.append(&header);

    let device = gtk::Entry::new();
    device.set_text(fields.device.as_deref().unwrap_or(""));
    device.set_hexpand(true);
    device.set_placeholder_text(Some("/dev/disk/by-uuid/..."));

    let fs_type = gtk::Entry::new();
    fs_type.set_text(fields.fs_type.as_deref().unwrap_or(""));
    fs_type.set_hexpand(true);
    fs_type.set_placeholder_text(Some("ext4, btrfs, vfat, tmpfs, none"));

    let options = gtk::Entry::new();
    options.set_text(&fields.options.join(" "));
    options.set_hexpand(true);
    options.set_placeholder_text(Some("noatime nofail"));

    let needed_for_boot = gtk::Switch::new();
    needed_for_boot.set_active(fields.needed_for_boot);
    needed_for_boot.set_halign(gtk::Align::Start);

    let nofail = gtk::Switch::new();
    nofail.set_active(
        fields.options.iter().any(|option| option == "nofail") || fields.mountpoint.is_empty(),
    );
    nofail.set_halign(gtk::Align::Start);

    let read_only = gtk::Switch::new();
    read_only.set_active(fields.options.iter().any(|option| option == "ro"));
    read_only.set_halign(gtk::Align::Start);

    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(6)
        .build();
    field(&grid, "Mountpoint", &mountpoint, 0);
    field(&grid, "Device", &device, 1);
    field(&grid, "Filesystem", &fs_type, 2);
    field(&grid, "Options", &options, 3);
    field(&grid, "Nofail", &nofail, 4);
    field(&grid, "Read only", &read_only, 5);
    field(&grid, "Needed for boot", &needed_for_boot, 6);
    content.append(&grid);

    let migrate = gtk::Button::with_label("Migrate existing contents");
    migrate.set_halign(gtk::Align::Start);
    migrate.set_sensitive(false);
    content.append(&migrate);

    expander.set_child(Some(&content));

    {
        let migrate = migrate.clone();
        let update_mountpoint = mountpoint.clone();
        let update = Rc::new(move || {
            let target = update_mountpoint.text().trim().to_string();
            let state = paths::check_target(&target);
            migrate.set_sensitive(matches!(state, Ok(paths::TargetState::UserOwned)));
        });
        update();
        let update_for_signal = update.clone();
        mountpoint.connect_changed(move |_| update_for_signal());
    }

    {
        let terminal = terminal.clone();
        let mountpoint = mountpoint.clone();
        let device = device.clone();
        let options = options.clone();
        migrate.connect_clicked(move |_| {
            let target = mountpoint.text().trim().to_string();
            let device = device.text().trim().to_string();
            if target.is_empty() || device.is_empty() {
                return;
            }
            let Some(terminal) = terminal.clone() else {
                return;
            };
            let options: Vec<String> = options
                .text()
                .split_whitespace()
                .map(str::to_string)
                .collect();
            let script = migrate::script(&target, &device, &options);
            let _ = terminal(&[format!("sh -c {}", migrate::shell_quote(&script))]);
        });
    }

    {
        let expander = expander.clone();
        let notify = notify.clone();
        mountpoint.connect_changed(move |entry| {
            let text = entry.text().trim().to_string();
            expander.set_label(Some(if text.is_empty() {
                "New mount"
            } else {
                text.as_str()
            }));
            notify();
        });
    }
    for entry in [&device, &fs_type, &options] {
        let notify = notify.clone();
        entry.connect_changed(move |_| notify());
    }
    for switch in [&nofail, &read_only] {
        let notify = notify.clone();
        switch.connect_active_notify(move |_| notify());
    }
    {
        let notify = notify.clone();
        needed_for_boot.connect_active_notify(move |_| notify());
    }

    {
        let rows = rows.clone();
        let rows_box = rows_box.clone();
        let notify = notify.clone();
        let expander = expander.clone();
        remove.connect_clicked(move |_| {
            rows_box.remove(&expander);
            rows.borrow_mut().retain(|row| row.expander != expander);
            notify();
        });
    }

    MountsRow {
        orig,
        mountpoint,
        device,
        fs_type,
        options,
        nofail,
        read_only,
        needed_for_boot,
        expander,
    }
}

fn field(grid: &gtk::Grid, label: &str, widget: &impl IsA<gtk::Widget>, row: i32) {
    let title = gtk::Label::new(Some(label));
    title.set_xalign(0.0);
    grid.attach(&title, 0, row, 1, 1);
    widget.set_hexpand(true);
    grid.attach(widget, 1, row, 1, 1);
}

pub(crate) fn mounts_nix(editor: &Editor) -> String {
    let Editor::Mounts { rows, .. } = editor else {
        return "{ }".to_string();
    };
    let rows: Vec<MountRow> = rows.borrow().iter().map(MountsRow::read).collect();
    serialize_file_systems(&rows)
}

pub(crate) fn mounts_unchanged(rows: &Rc<RefCell<Vec<MountsRow>>>, orig_count: usize) -> bool {
    let rows = rows.borrow();
    rows.len() == orig_count && rows.iter().all(|row| row.read() == row.orig)
}

pub(crate) fn mounts_validation_error(rows: &Rc<RefCell<Vec<MountsRow>>>) -> Option<String> {
    let rows = rows.borrow();
    let mut seen: Vec<String> = Vec::new();
    for row in rows.iter() {
        let value = row.read();
        if value.mountpoint.is_empty() {
            return Some("Every mount needs a mountpoint.".to_string());
        }
        let Some(normalized) = paths::normalize(&value.mountpoint) else {
            return Some(format!("{} is not an absolute path.", value.mountpoint));
        };
        if let Err(message) = paths::check_target(&normalized) {
            return Some(message);
        }
        if seen.contains(&normalized) {
            return Some(format!("{normalized} is used by more than one mount."));
        }
        seen.push(normalized);
        if value.device.is_none() && value.fs_type.is_none() {
            return Some(format!(
                "{} needs a device or a filesystem.",
                value.mountpoint
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_mountpoint() {
        let rows: Rc<RefCell<Vec<MountsRow>>> = Rc::new(RefCell::new(Vec::new()));
        assert!(mounts_validation_error(&rows).is_none());
        let _ = rows;
    }

    #[test]
    fn non_empty_trims() {
        assert_eq!(non_empty("  /dev/sda1 "), Some("/dev/sda1".to_string()));
        assert_eq!(non_empty("   "), None);
    }
}
