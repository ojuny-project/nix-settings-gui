use super::basic::build_dropdown;
use super::context::BuildContext;
use super::dispatch::resolve_choices;
use super::editor::Editor;
use super::facter_detect::{self, HardwareDetection};
use super::nix::{normalize_path, parse_string_literal, unwrap_modifiers};
use crate::model::Record;
use gtk::prelude::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(crate) fn report_path(base: &str) -> PathBuf {
    let root = normalize_path(Path::new(base));
    let preferred = root.join("modules").join("facter.json");
    if preferred.is_file() {
        return preferred;
    }
    let mut found = Vec::new();
    find_reports(&root, &mut found);
    found.sort();
    found.into_iter().next().unwrap_or(preferred)
}

fn find_reports(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.')
            || matches!(name.as_ref(), "result" | "target")
            || name.starts_with("result-")
        {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            find_reports(&path, found);
        } else if name == "facter.json" {
            found.push(path);
        }
    }
}

pub(crate) fn build_facter(record: &Record, type_str: Option<&str>, ctx: &BuildContext) -> Editor {
    let mut choices = resolve_choices(&record.spec, type_str);
    if choices.is_empty() {
        choices.push("auto".to_string());
        choices.extend(facter_detect::profile_choices());
    }
    let value_text = unwrap_modifiers(&record.value);
    let editor = build_dropdown(&choices, &value_text);
    let Editor::Dropdown { combo, choices } = editor else {
        return editor;
    };

    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
    root.set_hexpand(true);
    let path = report_path(&ctx.base);
    let report = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| facter_detect::detect_report(&text));
    let present = report.is_some();
    let detection = report.unwrap_or_else(HardwareDetection::empty);

    if !present {
        let empty = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .label(format!(
                "No hardware report at {}. Run nix run .#regenerate-hardware -- --profile-only, or choose a profile manually.",
                path.display()
            ))
            .css_classes(["dim-label", "caption"])
            .build();
        root.append(&empty);
    }

    let grid = gtk::Grid::new();
    grid.set_column_spacing(12);
    grid.set_row_spacing(6);
    let profile_caption = gtk::Label::builder()
        .xalign(1.0)
        .label("Hardware profile")
        .css_classes(["dim-label", "caption"])
        .build();
    grid.attach(&profile_caption, 0, 0, 1, 1);
    grid.attach(&combo, 1, 0, 1, 1);
    let model_label = status_row(&grid, 1, "Detected model");
    let profile_label = status_row(&grid, 2, "Detected profile");
    let state_label = status_row(&grid, 3, "Coverage state");
    let gpu_label = status_row(&grid, 4, "Effective GPU");
    let source_label = status_row(&grid, 5, "GPU source");
    let managed_label = status_row(&grid, 6, "Profile manages GPU");
    root.append(&grid);

    let update: Rc<dyn Fn(&gtk::DropDown)> = {
        let choices = choices.clone();
        let detection = detection.clone();
        Rc::new(move |combo| {
            model_label.set_text(&detection.model);
            profile_label.set_text(&detection.profile);
            state_label.set_text(&detection.state);
            let selected = choices
                .get(combo.selected() as usize)
                .map(|choice| parse_string_literal(choice))
                .unwrap_or_else(|| "auto".to_string());
            let (_, gpu) = facter_detect::effective(&selected, &detection);
            gpu_label.set_text(&gpu.layout);
            source_label.set_text(&gpu.source);
            managed_label.set_text(if gpu.managed { "Yes" } else { "No" });
        })
    };
    update(&combo);
    {
        let update = update.clone();
        combo.connect_selected_notify(move |combo| update(combo));
    }

    Editor::Facter {
        root,
        combo,
        choices,
    }
}

fn status_row(grid: &gtk::Grid, row: i32, caption: &str) -> gtk::Label {
    let caption = gtk::Label::builder()
        .xalign(1.0)
        .label(caption)
        .css_classes(["dim-label", "caption"])
        .build();
    let value = gtk::Label::builder()
        .xalign(0.0)
        .selectable(true)
        .wrap(true)
        .build();
    grid.attach(&caption, 0, row, 1, 1);
    grid.attach(&value, 1, row, 1, 1);
    value
}

#[cfg(test)]
mod tests {
    use super::report_path;
    use std::path::PathBuf;

    #[test]
    fn resolves_the_report_from_the_flake_root() {
        assert_eq!(
            report_path("/repo"),
            PathBuf::from("/repo/modules/facter.json")
        );
    }

    #[test]
    fn finds_an_existing_report_in_a_nested_module_tree() {
        let root = std::env::temp_dir().join(format!(
            "nix-settings-gui-facter-{}",
            std::process::id()
        ));
        let nested = root.join("modules/07-system");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("facter.json"), "{}").unwrap();
        assert_eq!(report_path(root.to_str().unwrap()), nested.join("facter.json"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
