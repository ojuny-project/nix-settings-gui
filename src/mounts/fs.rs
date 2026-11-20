//! Per-filesystem defaults and the choices the add flow must collect before a
//! row is written.

/// A decision the add dialog must collect for a filesystem type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// Ownership and permission mapping for uid, gid, and masks.
    Owner,
    /// NTFS driver choice: `ntfs3` or `ntfs`.
    Driver,
    /// btrfs subvolume path, existing or created.
    Subvolume,
    /// Trim policy: `discard`, periodic fstrim, or none.
    Trim,
    /// tmpfs size.
    Size,
    /// tmpfs mode.
    Mode,
}

/// Options that a type needs before its first mount, and whether fsck applies.
pub fn requirements(fs_type: &str) -> &'static [Requirement] {
    match fs_type.to_ascii_lowercase().as_str() {
        "vfat" | "exfat" => &[Requirement::Owner],
        "ntfs" | "ntfs3" => &[Requirement::Owner, Requirement::Driver],
        "btrfs" => &[Requirement::Subvolume],
        "ext4" | "xfs" | "f2fs" => &[Requirement::Trim],
        "tmpfs" => &[Requirement::Size, Requirement::Mode],
        _ => &[],
    }
}

/// Baseline options written for a type. Ownership values come from the add
/// dialog, so only the type-wide flags live here.
pub fn default_options(fs_type: &str) -> &'static [&'static str] {
    match fs_type.to_ascii_lowercase().as_str() {
        "vfat" => &["utf8", "shortname=winnt"],
        "exfat" => &["utf8"],
        "ntfs" | "ntfs3" => &["windows_names"],
        "btrfs" => &["compress=zstd", "noatime"],
        "ext4" | "xfs" | "f2fs" => &["noatime"],
        _ => &[],
    }
}

/// Whether the boot fsck stage should run for this type. NTFS and exFAT have no
/// guaranteed fsck helper in the boot environment.
pub fn fsck(fs_type: &str) -> bool {
    !matches!(fs_type.to_ascii_lowercase().as_str(), "ntfs" | "ntfs3" | "exfat" | "tmpfs")
}

/// The type a bare `ntfs` choice should be written as, given the driver pick.
pub fn ntfs_fs_type(driver: &str) -> &'static str {
    if driver == "ntfs3" {
        "ntfs3"
    } else {
        "ntfs"
    }
}

/// Whether a requirement is already satisfied by the current option list.
pub fn satisfied(requirement: Requirement, options: &[String]) -> bool {
    let has = |prefix: &str| options.iter().any(|option| option.starts_with(prefix));
    match requirement {
        // The driver is encoded in fsType, so it is never a missing option.
        Requirement::Driver | Requirement::Trim => true,
        Requirement::Owner => {
            has("uid=") && has("gid=") && (has("umask=") || has("fmask=") || has("dmask="))
        }
        Requirement::Subvolume => has("subvol=") || has("subvolid="),
        Requirement::Size => has("size="),
        Requirement::Mode => has("mode="),
    }
}

/// The requirements that still need a decision before the row may be saved.
pub fn missing_requirements(fs_type: &str, options: &[String]) -> Vec<Requirement> {
    requirements(fs_type)
        .iter()
        .copied()
        .filter(|requirement| !satisfied(*requirement, options))
        .collect()
}

impl Requirement {
    pub fn label(self) -> &'static str {
        match self {
            Requirement::Owner => "an owner mapping (uid, gid, and umask or masks)",
            Requirement::Driver => "a driver",
            Requirement::Subvolume => "a subvolume (subvol= or subvolid=)",
            Requirement::Trim => "a trim policy",
            Requirement::Size => "a size",
            Requirement::Mode => "a mode",
        }
    }
}

/// The `mkfs` command for a type, or `None` when the type has no formatter.
pub fn mkfs_command(fs_type: &str, device: &str) -> Option<String> {
    let (program, flags) = match fs_type.to_ascii_lowercase().as_str() {
        "ext4" => ("mkfs.ext4", "-F"),
        "btrfs" => ("mkfs.btrfs", "-f"),
        "xfs" => ("mkfs.xfs", "-f"),
        "f2fs" => ("mkfs.f2fs", "-f"),
        "vfat" => ("mkfs.vfat", ""),
        "exfat" => ("mkfs.exfat", ""),
        "ntfs" | "ntfs3" => ("mkfs.ntfs", "-F"),
        _ => return None,
    };
    let command = if flags.is_empty() {
        format!("{program} {device}")
    } else {
        format!("{program} {flags} {device}")
    };
    Some(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn vfat_needs_an_owner_mapping() {
        assert_eq!(requirements("vfat"), &[Requirement::Owner]);
        assert!(default_options("vfat").contains(&"utf8"));
    }

    #[test]
    fn ntfs_needs_a_driver_and_has_no_fsck() {
        assert_eq!(
            requirements("ntfs3"),
            &[Requirement::Owner, Requirement::Driver]
        );
        assert!(!fsck("ntfs"));
        assert!(!fsck("exfat"));
        assert_eq!(ntfs_fs_type("ntfs3"), "ntfs3");
        assert_eq!(ntfs_fs_type("ntfs-3g"), "ntfs");
    }

    #[test]
    fn btrfs_needs_a_subvolume() {
        assert_eq!(requirements("btrfs"), &[Requirement::Subvolume]);
        assert!(default_options("btrfs").contains(&"compress=zstd"));
    }

    #[test]
    fn block_types_need_a_trim_policy() {
        for fs_type in ["ext4", "xfs", "f2fs"] {
            assert_eq!(requirements(fs_type), &[Requirement::Trim], "{fs_type}");
        }
    }

    #[test]
    fn tmpfs_needs_size_and_mode() {
        assert_eq!(requirements("tmpfs"), &[Requirement::Size, Requirement::Mode]);
        assert!(!fsck("tmpfs"));
    }

    #[test]
    fn unknown_types_have_no_requirements() {
        assert!(requirements("iso9660").is_empty());
        assert!(default_options("iso9660").is_empty());
        assert!(fsck("iso9660"));
    }

    #[test]
    fn flags_missing_owner_mapping() {
        assert_eq!(
            missing_requirements("vfat", &options(&["utf8"])),
            vec![Requirement::Owner]
        );
        assert!(missing_requirements(
            "vfat",
            &options(&["utf8", "uid=1000", "gid=100", "umask=0022"])
        )
        .is_empty());
        assert!(missing_requirements(
            "ntfs3",
            &options(&["uid=1000", "gid=100", "fmask=0077", "dmask=0077"])
        )
        .is_empty());
    }

    #[test]
    fn flags_missing_btrfs_subvolume_and_tmpfs_fields() {
        assert_eq!(
            missing_requirements("btrfs", &options(&["compress=zstd"])),
            vec![Requirement::Subvolume]
        );
        assert!(missing_requirements("btrfs", &options(&["subvol=@data"])).is_empty());
        assert_eq!(
            missing_requirements("tmpfs", &options(&["size=1G"])),
            vec![Requirement::Mode]
        );
        assert!(missing_requirements("tmpfs", &options(&["size=1G", "mode=1777"])).is_empty());
    }

    #[test]
    fn ext4_needs_nothing_extra() {
        assert!(missing_requirements("ext4", &options(&["noatime"])).is_empty());
    }

    #[test]
    fn builds_mkfs_commands() {
        assert_eq!(
            mkfs_command("ext4", "/dev/sdb1").as_deref(),
            Some("mkfs.ext4 -F /dev/sdb1")
        );
        assert_eq!(
            mkfs_command("btrfs", "/dev/sdb1").as_deref(),
            Some("mkfs.btrfs -f /dev/sdb1")
        );
        assert_eq!(
            mkfs_command("vfat", "/dev/sdb1").as_deref(),
            Some("mkfs.vfat /dev/sdb1")
        );
        assert_eq!(mkfs_command("iso9660", "/dev/sdb1"), None);
    }
}
