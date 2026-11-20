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
}
