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
    pub size: Option<u64>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub pkname: Option<String>,
    #[serde(default)]
    pub mountpoints: Vec<Option<String>>,
    #[serde(default)]
    pub children: Vec<Device>,
}

impl Device {
    pub fn is_mounted(&self) -> bool {
        self.mountpoints
            .iter()
            .flatten()
            .any(|mountpoint| !mountpoint.is_empty())
    }
}

/// Parse the JSON printed by `lsblk --json --bytes`.
pub fn parse_lsblk(text: &str) -> Option<BlockDevices> {
    serde_json::from_str(text).ok()
}

pub fn format_size(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "?".to_string();
    };
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
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

pub fn canonical_sysfs_block_path(name: &str) -> String {
    std::fs::canonicalize(sysfs_block_path(name))
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| sysfs_block_path(name))
}

/// Classify one disk using the live kernel state.
#[allow(dead_code)]
pub fn disk_is_external(device: &Device) -> bool {
    let path = canonical_sysfs_block_path(&device.name);
    is_external(device, &path, &thunderbolt_bdfs())
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
    pub size: Option<u64>,
    pub model: Option<String>,
}

/// Partitions on internal disks that carry a mountable filesystem and are not
/// already mounted.
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
            if part.kind != "part" || part.is_mounted() {
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
                size: part.size,
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
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskCandidate {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
    pub model: Option<String>,
    /// Unallocated bytes on a disk. `None` for partitions.
    pub free: Option<u64>,
}

fn disk_used_bytes(disk: &Device) -> u64 {
    disk.children.iter().filter_map(|child| child.size).sum()
}

fn disk_free_bytes(disk: &Device) -> u64 {
    disk.size.unwrap_or(0).saturating_sub(disk_used_bytes(disk))
}

fn has_mounted_child(disk: &Device) -> bool {
    disk.is_mounted()
        || disk
            .children
            .iter()
            .any(|child| child.is_mounted())
}

fn is_internal_disk(disk: &Device, protected: &[String], thunderbolt: &[String]) -> bool {
    disk.kind == "disk"
        && !protected.contains(&disk.name)
        && !is_external(
            disk,
            &canonical_sysfs_block_path(&disk.name),
            thunderbolt,
        )
}

/// Internal disks that are not the boot, root or Nix disk and have no mounted
/// partition. They are the only disks cfdisk may touch.
#[allow(dead_code)]
pub fn internal_disks(devices: &BlockDevices) -> Vec<DiskCandidate> {
    let tb = thunderbolt_bdfs();
    let protected = protected_disk_names();
    let mut out: Vec<DiskCandidate> = devices
        .blockdevices
        .iter()
        .filter(|disk| is_internal_disk(disk, &protected, &tb))
        .filter(|disk| !has_mounted_child(disk))
        .map(|disk| DiskCandidate {
            name: disk.name.clone(),
            path: disk.path.clone(),
            size: disk.size,
            model: disk.model.clone(),
            free: None,
        })
        .collect();
    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

/// Internal disks with unallocated space and no mounted partition.
#[allow(dead_code)]
pub fn free_space_disks(devices: &BlockDevices) -> Vec<DiskCandidate> {
    let tb = thunderbolt_bdfs();
    let protected = protected_disk_names();
    let mut out: Vec<DiskCandidate> = devices
        .blockdevices
        .iter()
        .filter(|disk| is_internal_disk(disk, &protected, &tb))
        .filter(|disk| !has_mounted_child(disk))
        .filter_map(|disk| {
            let free = disk_free_bytes(disk);
            (free > 0).then(|| DiskCandidate {
                name: disk.name.clone(),
                path: disk.path.clone(),
                size: disk.size,
                model: disk.model.clone(),
                free: Some(free),
            })
        })
        .collect();
    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

/// Internal partitions with no filesystem, excluding the ESP.
#[allow(dead_code)]
pub fn unformatted_partitions(devices: &BlockDevices) -> Vec<DiskCandidate> {
    let tb = thunderbolt_bdfs();
    let protected = protected_disk_names();
    let mut out = Vec::new();
    for disk in &devices.blockdevices {
        if !is_internal_disk(disk, &protected, &tb) {
            continue;
        }
        for part in &disk.children {
            if part.kind != "part" || part.fstype.is_some() || part.is_mounted() {
                continue;
            }
            if part.parttype.as_deref() == Some(ESP_PARTTYPE) {
                continue;
            }
            out.push(DiskCandidate {
                name: part.name.clone(),
                path: part.path.clone(),
                size: part.size,
                model: disk.model.clone(),
                free: None,
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

/// Run `lsblk` and parse the result. A missing or failing tool returns none.
pub fn probe() -> Option<BlockDevices> {
    let output = std::process::Command::new("lsblk")
        .args([
            "--json",
            "--bytes",
            "--output",
            "NAME,PATH,TYPE,TRAN,RM,HOTPLUG,FSTYPE,PARTTYPE,UUID,SIZE,MODEL,PKNAME,MOUNTPOINTS",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_lsblk(&String::from_utf8_lossy(&output.stdout))
}

/// One partition in the Modify tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverviewPartition {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
    pub fstype: Option<String>,
    pub parttype: Option<String>,
    pub mounted: bool,
}

/// One disk and its partitions in the Modify tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverviewDisk {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
    pub model: Option<String>,
    pub free: u64,
    pub mounted_child: bool,
    pub partitions: Vec<OverviewPartition>,
}

/// Disks and partitions for the tree view. A disk is included when it has
/// unallocated space or an unformatted partition.
pub fn overview(devices: &BlockDevices) -> Vec<OverviewDisk> {
    let tb = thunderbolt_bdfs();
    let protected = protected_disk_names();
    let mut out = Vec::new();
    for disk in &devices.blockdevices {
        if !is_internal_disk(disk, &protected, &tb) {
            continue;
        }
        let partitions: Vec<OverviewPartition> = disk
            .children
            .iter()
            .filter(|part| part.kind == "part")
            .map(|part| OverviewPartition {
                name: part.name.clone(),
                path: part.path.clone(),
                size: part.size,
                fstype: part.fstype.clone(),
                parttype: part.parttype.clone(),
                mounted: part.is_mounted(),
            })
            .collect();
        let has_unformatted = partitions.iter().any(|part| {
            part.fstype.is_none()
                && !part.mounted
                && part.parttype.as_deref() != Some(ESP_PARTTYPE)
        });
        let free = disk_free_bytes(disk);
        if free == 0 && !has_unformatted {
            continue;
        }
        out.push(OverviewDisk {
            name: disk.name.clone(),
            path: disk.path.clone(),
            size: disk.size,
            model: disk.model.clone(),
            free,
            mounted_child: has_mounted_child(disk),
            partitions,
        });
    }
    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

/// UUID to `/dev` name for every partition, including mounted ones. Used by
/// the table's Drive name column.
pub fn partition_names(devices: &BlockDevices) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for disk in &devices.blockdevices {
        for part in &disk.children {
            if part.kind == "part" {
                if let Some(uuid) = &part.uuid {
                    out.push((uuid.clone(), part.name.clone()));
                }
            }
        }
    }
    out
}

/// Whether a partition already has a filesystem. A failed probe returns true so
/// the Modify action refuses rather than overwriting.
pub fn has_filesystem(path: &str) -> bool {
    match std::process::Command::new("lsblk")
        .args(["-no", "FSTYPE", path])
        .output()
    {
        Ok(output) => !String::from_utf8_lossy(&output.stdout).trim().is_empty(),
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "blockdevices": [
        {"name":"sdb","path":"/dev/sdb","type":"disk","tran":"usb","rm":false,
         "hotplug":true,"fstype":null,"size":500107862016,"model":"Tech","pkname":null,
         "mountpoints":[],
         "children":[
           {"name":"sdb1","path":"/dev/sdb1","type":"part","tran":"usb","rm":false,
            "hotplug":true,"fstype":"exfat","uuid":"4E21-0000","size":100000000000,
            "pkname":"sdb","mountpoints":[]}
         ]},
        {"name":"nvme0n1","path":"/dev/nvme0n1","type":"disk","tran":"nvme","rm":false,
         "hotplug":false,"fstype":null,"size":256060514304,"model":"INTEL","pkname":null,
         "mountpoints":[],
         "children":[
           {"name":"nvme0n1p1","path":"/dev/nvme0n1p1","type":"part","tran":"nvme",
            "rm":false,"hotplug":false,"fstype":"vfat","uuid":"C542-8434",
            "size":1073741824,"pkname":"nvme0n1","mountpoints":["/boot"]},
           {"name":"nvme0n1p2","path":"/dev/nvme0n1p2","type":"part","tran":"nvme",
            "rm":false,"hotplug":false,"fstype":"ext4","uuid":"a710",
            "size":240000000000,"pkname":"nvme0n1","mountpoints":["/"]}
         ]},
        {"name":"sdc","path":"/dev/sdc","type":"disk","tran":"sata","rm":false,
         "hotplug":false,"fstype":null,"size":240057409536,"model":"WD Green",
         "pkname":null,"mountpoints":[],
         "children":[
           {"name":"sdc1","path":"/dev/sdc1","type":"part","tran":"sata","rm":false,
            "hotplug":false,"fstype":"ext4","uuid":"1111-2222","size":100000000000,
            "pkname":"sdc","mountpoints":[]},
           {"name":"sdc2","path":"/dev/sdc2","type":"part","tran":"sata","rm":false,
            "hotplug":false,"fstype":"swap","uuid":"3333-4444","size":8589934592,
            "pkname":"sdc","mountpoints":["[SWAP]"]}
         ]},
        {"name":"sdd","path":"/dev/sdd","type":"disk","tran":"sata","rm":false,
         "hotplug":false,"fstype":null,"size":500000000000,"model":"Spare",
         "pkname":null,"mountpoints":[],
         "children":[
           {"name":"sdd1","path":"/dev/sdd1","type":"part","tran":"sata","rm":false,
            "hotplug":false,"fstype":"ext4","uuid":"9999-0000","size":100000000000,
            "pkname":"sdd","mountpoints":[]}
         ]}
      ]
    }"#;

    #[test]
    fn parses_lsblk_json_and_children() {
        let parsed = parse_lsblk(SAMPLE).expect("json");
        assert_eq!(parsed.blockdevices.len(), 4);
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
    fn offers_only_unmounted_mountable_internal_partitions() {
        let parsed = parse_lsblk(SAMPLE).expect("json");
        let candidates = internal_candidates(&parsed);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].name, "sdc1");
        assert_eq!(candidates[1].name, "sdd1");
        assert_eq!(candidates[0].fstype, "ext4");
    }

    #[test]
    fn free_space_disks_skip_mounted_and_protected_disks() {
        let parsed = parse_lsblk(SAMPLE).expect("json");
        let free = free_space_disks(&parsed);
        assert_eq!(free.len(), 1);
        assert_eq!(free[0].name, "sdd");
        assert!(free[0].free.unwrap() > 0);
    }

    #[test]
    fn unformatted_partitions_exclude_mounted_and_swap() {
        let parsed = parse_lsblk(SAMPLE).expect("json");
        let parts = unformatted_partitions(&parsed);
        assert!(parts.is_empty());
    }

    #[test]
    fn formats_bytes() {
        assert_eq!(format_size(Some(0)), "0 B");
        assert_eq!(format_size(Some(1024)), "1.0 K");
        assert_eq!(format_size(Some(256060514304)), "238.5 G");
        assert_eq!(format_size(None), "?");
    }
}
