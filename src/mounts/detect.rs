//! Block-device probing and the internal/external classifier.
//!
//! The classifier reads the transport from `lsblk` and treats USB, MMC, the
//! removable flag and a PCIe tunnel behind a Thunderbolt controller as
//! external. The Thunderbolt check is a path match against the controller BDF,
//! which the kernel already publishes under
//! `/sys/bus/thunderbolt/devices/domain*/device`.

use serde::Deserialize;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct BlockDevices {
    #[serde(default)]
    pub blockdevices: Vec<Device>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub tran: Option<String>,
    #[serde(default)]
    pub rm: bool,
    #[serde(default)]
    pub hotplug: bool,
    #[serde(default)]
    pub fstype: Option<String>,
    #[serde(default)]
    pub parttype: Option<String>,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub pkname: Option<String>,
    #[serde(default)]
    pub children: Vec<Device>,
}

/// Parse the JSON printed by `lsblk --json`.
pub fn parse_lsblk(text: &str) -> Option<BlockDevices> {
    serde_json::from_str(text).ok()
}

/// A disk is external when it is on USB or MMC, reports itself removable, or
/// its sysfs path sits behind a Thunderbolt controller.
pub fn is_external(device: &Device, sysfs_path: &str, thunderbolt: &[String]) -> bool {
    let transport = device.tran.as_deref().unwrap_or("").to_ascii_lowercase();
    transport == "usb"
        || transport == "mmc"
        || device.rm
        || thunderbolt.iter().any(|bdf| sysfs_path.contains(bdf))
}

/// The controller BDF is the PCI component directly before `domainN`.
pub fn bdf_from_controller_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        if !part.starts_with("domain") || index == 0 {
            continue;
        }
        let candidate = parts[index - 1];
        if is_bdf(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn is_bdf(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 12 {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        let valid = match index {
            4 | 7 => *byte == b':',
            10 => *byte == b'.',
            _ => byte.is_ascii_hexdigit(),
        };
        if !valid {
            return false;
        }
    }
    true
}

/// Read the controller BDFs from the running kernel. Missing bus means empty.
pub fn thunderbolt_bdfs() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir("/sys/bus/thunderbolt/devices") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("domain") {
            continue;
        }
        let device = entry.path().join("device");
        let Ok(canonical) = std::fs::canonicalize(&device) else {
            continue;
        };
        if let Some(bdf) = bdf_from_controller_path(&canonical.to_string_lossy()) {
            if !out.contains(&bdf) {
                out.push(bdf);
            }
        }
    }
    out
}

pub fn sysfs_block_path(name: &str) -> String {
    format!("/sys/block/{name}")
}

/// The EFI system partition GUID, excluded from the mount candidates.
pub const ESP_PARTTYPE: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";

/// One partition the internal mount form can offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountCandidate {
    pub name: String,
    pub path: String,
    pub uuid: Option<String>,
    pub fstype: String,
    pub size: Option<String>,
    pub model: Option<String>,
}

/// Partitions on internal disks that carry a mountable filesystem.
pub fn internal_candidates(devices: &BlockDevices) -> Vec<MountCandidate> {
    let tb = thunderbolt_bdfs();
    let mut out = Vec::new();
    for disk in &devices.blockdevices {
        if disk.kind != "disk" {
            continue;
        }
        let sysfs = canonical_sysfs_block_path(&disk.name);
        if is_external(disk, &sysfs, &tb) {
            continue;
        }
        for part in &disk.children {
            if part.kind != "part" {
                continue;
            }
            let Some(fstype) = part.fstype.as_deref() else {
                continue;
            };
            if is_unmountable_type(fstype) {
                continue;
            }
            if part.parttype.as_deref() == Some(ESP_PARTTYPE) {
                continue;
            }
            out.push(MountCandidate {
                name: part.name.clone(),
                path: if part.path.is_empty() {
                    format!("/dev/{}", part.name)
                } else {
                    part.path.clone()
                },
                uuid: part.uuid.clone(),
                fstype: fstype.to_string(),
                size: part.size.clone(),
                model: disk.model.clone(),
            });
        }
    }
    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

fn is_unmountable_type(fstype: &str) -> bool {
    matches!(
        fstype,
        "swap"
            | "crypto_LUKS"
            | "LVM2_member"
            | "linux_raid_member"
            | "zfs_member"
            | "BitLocker"
    )
}

/// One disk or partition offered to the Modify actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskCandidate {
    pub name: String,
    pub path: String,
    pub size: Option<String>,
    pub model: Option<String>,
}

/// Internal disks that are not the boot, root or Nix disk.
pub fn internal_disks(devices: &BlockDevices) -> Vec<DiskCandidate> {
    let tb = thunderbolt_bdfs();
    let protected = protected_disk_names();
    let mut out: Vec<DiskCandidate> = devices
        .blockdevices
        .iter()
        .filter(|disk| disk.kind == "disk")
        .filter(|disk| !protected.contains(&disk.name))
        .filter(|disk| !is_external(disk, &canonical_sysfs_block_path(&disk.name), &tb))
        .map(|disk| DiskCandidate {
            name: disk.name.clone(),
            path: disk.path.clone(),
            size: disk.size.clone(),
            model: disk.model.clone(),
        })
        .collect();
    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

/// Internal partitions with no filesystem, excluding the ESP.
pub fn unformatted_partitions(devices: &BlockDevices) -> Vec<DiskCandidate> {
    let tb = thunderbolt_bdfs();
    let protected = protected_disk_names();
    let mut out = Vec::new();
    for disk in &devices.blockdevices {
        if disk.kind != "disk" || protected.contains(&disk.name) {
            continue;
        }
        if is_external(disk, &canonical_sysfs_block_path(&disk.name), &tb) {
            continue;
        }
        for part in &disk.children {
            if part.kind != "part" || part.fstype.is_some() {
                continue;
            }
            if part.parttype.as_deref() == Some(ESP_PARTTYPE) {
                continue;
            }
            out.push(DiskCandidate {
                name: part.name.clone(),
                path: part.path.clone(),
                size: part.size.clone(),
                model: disk.model.clone(),
            });
        }
    }
    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

/// Disks that carry a protected mount, so cfdisk must not touch them.
pub fn protected_disk_names() -> Vec<String> {
    ["/", "/boot", "/nix", "/efi"]
        .iter()
        .filter_map(|target| {
            let output = std::process::Command::new("findmnt")
                .args(["-no", "SOURCE", target])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let source = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if source.is_empty() {
                return None;
            }
            let output = std::process::Command::new("lsblk")
                .args(["-no", "PKNAME", &source])
                .output()
                .ok()?;
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Some(if name.is_empty() { source } else { name })
        })
        .collect()
}

pub fn canonical_sysfs_block_path(name: &str) -> String {
    std::fs::canonicalize(sysfs_block_path(name))
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| sysfs_block_path(name))
}

/// Classify one disk using the live kernel state.
pub fn disk_is_external(device: &Device) -> bool {
    let path = canonical_sysfs_block_path(&device.name);
    is_external(device, &path, &thunderbolt_bdfs())
}

/// Run `lsblk` and parse the result. A missing or failing tool returns none.
pub fn probe() -> Option<BlockDevices> {
    let output = std::process::Command::new("lsblk")
        .args([
            "--json",
            "--output",
            "NAME,PATH,TYPE,TRAN,RM,HOTPLUG,FSTYPE,PARTTYPE,UUID,SIZE,MODEL,PKNAME",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_lsblk(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "blockdevices": [
        {"name":"sdb","path":"/dev/sdb","type":"disk","tran":"usb","rm":false,
         "hotplug":true,"fstype":null,"size":"465,8G","model":"Tech","pkname":null,
         "children":[
           {"name":"sdb1","path":"/dev/sdb1","type":"part","tran":"usb","rm":false,
            "hotplug":true,"fstype":"exfat","uuid":"4E21-0000","pkname":"sdb"}
         ]},
        {"name":"nvme0n1","path":"/dev/nvme0n1","type":"disk","tran":"nvme","rm":false,
         "hotplug":false,"fstype":null,"size":"238,5G","model":"INTEL","pkname":null}
      ]
    }"#;

    #[test]
    fn parses_lsblk_json_and_children() {
        let parsed = parse_lsblk(SAMPLE).expect("json");
        assert_eq!(parsed.blockdevices.len(), 2);
        assert_eq!(parsed.blockdevices[0].tran.as_deref(), Some("usb"));
        assert_eq!(parsed.blockdevices[0].children.len(), 1);
        assert_eq!(
            parsed.blockdevices[0].children[0].fstype.as_deref(),
            Some("exfat")
        );
        assert!(!parsed.blockdevices[0].rm);
    }

    #[test]
    fn usb_disk_with_rm_false_is_external() {
        let parsed = parse_lsblk(SAMPLE).expect("json");
        let disk = &parsed.blockdevices[0];
        assert!(is_external(disk, "/sys/devices/.../usb1/1-4/block/sdb", &[]));
    }

    #[test]
    fn internal_nvme_is_not_external() {
        let parsed = parse_lsblk(SAMPLE).expect("json");
        let disk = &parsed.blockdevices[1];
        assert!(!is_external(
            disk,
            "/sys/devices/pci0000:00/0000:00:1c.4/block/nvme0n1",
            &[]
        ));
    }

    #[test]
    fn a_disk_behind_the_controller_bdf_is_external() {
        let parsed = parse_lsblk(SAMPLE).expect("json");
        let disk = &parsed.blockdevices[1];
        let path = "/sys/devices/pci0000:00/0000:07:00.0/0000:08:00.0/nvme/nvme1/nvme1n1";
        assert!(is_external(disk, path, &["0000:07:00.0".to_string()]));
    }

    #[test]
    fn extracts_the_controller_bdf() {
        let path = "/sys/devices/pci0000:00/0000:00:1d.0/0000:05:00.0/0000:06:00.0/0000:07:00.0/domain0/device";
        assert_eq!(bdf_from_controller_path(path).as_deref(), Some("0000:07:00.0"));
        assert_eq!(bdf_from_controller_path("/sys/devices/virtual/block/loop0"), None);
    }

    #[test]
    fn rejects_a_malformed_bdf() {
        assert!(!is_bdf("0000:07:00"));
        assert!(!is_bdf("zzzz:07:00.0"));
        assert!(is_bdf("0000:07:00.0"));
    }

    #[test]
    fn offers_only_mountable_internal_partitions() {
        let sample = r#"{
          "blockdevices": [
            {"name":"sdc","path":"/dev/sdc","type":"disk","tran":"sata","rm":false,
             "hotplug":false,"fstype":null,"size":"240G","model":"WD Green","pkname":null,
             "children":[
               {"name":"sdc1","path":"/dev/sdc1","type":"part","tran":"sata","rm":false,
                "hotplug":false,"fstype":"ext4","uuid":"1111-2222","pkname":"sdc"},
               {"name":"sdc2","path":"/dev/sdc2","type":"part","tran":"sata","rm":false,
                "hotplug":false,"fstype":"swap","uuid":"3333-4444","pkname":"sdc"},
               {"name":"sdc3","path":"/dev/sdc3","type":"part","tran":"sata","rm":false,
                "hotplug":false,"fstype":"vfat","uuid":"5555-6666",
                "parttype":"c12a7328-f81f-11d2-ba4b-00a0c93ec93b","pkname":"sdc"}
             ]},
            {"name":"sdb","path":"/dev/sdb","type":"disk","tran":"usb","rm":false,
             "hotplug":true,"fstype":null,"size":"240G","model":"USB","pkname":null,
             "children":[
               {"name":"sdb1","path":"/dev/sdb1","type":"part","tran":"usb","rm":false,
                "hotplug":true,"fstype":"exfat","uuid":"7777-8888","pkname":"sdb"}
             ]}
          ]
        }"#;
        let parsed = parse_lsblk(sample).expect("json");
        let candidates = internal_candidates(&parsed);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "sdc1");
        assert_eq!(candidates[0].fstype, "ext4");
    }
}
