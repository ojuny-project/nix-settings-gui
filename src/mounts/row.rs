//! One `fileSystems` row and the literal scanner/serializer around it.

use crate::scan::scan_value_direct;
use crate::value;
use crate::widgets::nix::{attr_key, parse_string_literal, quote, truthy};

/// One mountpoint entry from a `fileSystems`-shaped attribute set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MountRow {
    pub mountpoint: String,
    pub device: Option<String>,
    pub fs_type: Option<String>,
    pub options: Vec<String>,
    pub options_present: bool,
    pub needed_for_boot: bool,
    /// Bindings the widget does not model, kept verbatim so a save never drops
    /// a hand-written field.
    pub extras: Vec<ExtraField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraField {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Identity {
    Device,
    Tmpfs,
    Bind,
    Synthetic,
}

impl MountRow {
    pub fn identity(&self) -> Identity {
        match self.fs_type.as_deref() {
            Some("tmpfs") => Identity::Tmpfs,
            Some("none") if self.options.iter().any(|option| option == "bind") => Identity::Bind,
            _ if self.device.is_some() => Identity::Device,
            _ => Identity::Synthetic,
        }
    }
}

/// Read every direct mountpoint binding of a `fileSystems` literal.
pub fn scan_file_systems(text: &str) -> Vec<MountRow> {
    scan_value_direct(text.trim())
        .into_iter()
        .map(|record| {
            let mut row = MountRow {
                mountpoint: record.segments.join("."),
                ..MountRow::default()
            };
            for field in scan_value_direct(&record.value) {
                let name = field.segments.join(".");
                match name.as_str() {
                    "device" => row.device = Some(parse_string_literal(&field.value)),
                    "fsType" => row.fs_type = Some(parse_string_literal(&field.value)),
                    "options" => {
                        row.options_present = true;
                        row.options = value::list_items(&field.value)
                            .unwrap_or_default()
                            .iter()
                            .map(|item| parse_string_literal(item))
                            .collect();
                    }
                    "neededForBoot" => row.needed_for_boot = truthy(&field.value),
                    _ => row.extras.push(ExtraField {
                        name,
                        value: field.value.trim().to_string(),
                    }),
                }
            }
            row
        })
        .collect()
}

/// Write the rows back as one attribute set literal.
pub fn serialize_file_systems(rows: &[MountRow]) -> String {
    let entries: Vec<String> = rows.iter().map(serialize_row).collect();
    format!("{{ {} }}", entries.join(" "))
}

fn serialize_row(row: &MountRow) -> String {
    let mut fields: Vec<String> = Vec::new();
    if let Some(device) = row.device.as_deref().filter(|value| !value.is_empty()) {
        fields.push(format!("device = {};", quote(device)));
    }
    if let Some(fs_type) = row.fs_type.as_deref().filter(|value| !value.is_empty()) {
        fields.push(format!("fsType = {};", quote(fs_type)));
    }
    if row.options_present {
        let options: Vec<String> = row
            .options
            .iter()
            .filter(|option| !option.is_empty())
            .map(|option| quote(option))
            .collect();
        if options.is_empty() {
            fields.push("options = [ ];".to_string());
        } else {
            fields.push(format!("options = [ {} ];", options.join(" ")));
        }
    }
    if row.needed_for_boot {
        fields.push("neededForBoot = true;".to_string());
    }
    for extra in &row.extras {
        fields.push(format!("{} = {};", attr_key(&extra.name), extra.value));
    }
    format!("{} = {{ {} }};", attr_key(&row.mountpoint), fields.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "/mnt/data" = {
        device = "/dev/disk/by-uuid/0f0e0d0c";
        fsType = "ext4";
        options = [ "noatime" "nofail" ];
        neededForBoot = false;
      };
    }"#;

    #[test]
    fn scans_a_mountpoint_row() {
        let rows = scan_file_systems(SAMPLE);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].mountpoint, "/mnt/data");
        assert_eq!(
            rows[0].device.as_deref(),
            Some("/dev/disk/by-uuid/0f0e0d0c")
        );
        assert_eq!(rows[0].fs_type.as_deref(), Some("ext4"));
        assert_eq!(rows[0].options, vec!["noatime", "nofail"]);
        assert!(rows[0].options_present);
        assert!(!rows[0].needed_for_boot);
        assert!(rows[0].extras.is_empty());
    }

    #[test]
    fn keeps_unknown_fields() {
        let rows = scan_file_systems(
            r#"{ "/mnt/x" = { device = "/dev/sda1"; fsType = "ext4"; label = "data"; }; }"#,
        );
        assert_eq!(rows[0].extras.len(), 1);
        assert_eq!(rows[0].extras[0].name, "label");
        assert_eq!(rows[0].extras[0].value, "\"data\"");
    }

    #[test]
    fn round_trips_rows_and_extras() {
        let rows = scan_file_systems(SAMPLE);
        let serialized = serialize_file_systems(&rows);
        let rescanned = scan_file_systems(&serialized);
        assert_eq!(rows, rescanned);
    }

    #[test]
    fn writes_an_empty_options_list() {
        let rows = scan_file_systems(
            r#"{ "/mnt/x" = { device = "/dev/sda1"; fsType = "ext4"; options = [ ]; }; }"#,
        );
        let serialized = serialize_file_systems(&rows);
        assert!(serialized.contains("options = [ ];"), "{serialized}");
    }

    #[test]
    fn identifies_tmpfs_and_bind_rows() {
        let tmpfs = scan_file_systems(r#"{ "/mnt/ram" = { fsType = "tmpfs"; options = [ "size=1G" ]; }; }"#);
        assert_eq!(tmpfs[0].identity(), Identity::Tmpfs);
        let bind = scan_file_systems(
            r#"{ "/mnt/src" = { device = "/srv/src"; fsType = "none"; options = [ "bind" ]; }; }"#,
        );
        assert_eq!(bind[0].identity(), Identity::Bind);
    }

    #[test]
    fn scans_an_empty_set() {
        assert!(scan_file_systems("{ }").is_empty());
    }
}
