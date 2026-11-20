//! Failure-proof migration of a non-empty, user-owned directory onto a new
//! filesystem.
//!
//! The script copies, verifies, moves the original aside, mounts the new
//! filesystem, and leaves the old copy in place. Deleting the old copy is a
//! separate step after a successful boot.

/// Quote one shell word with single quotes.
pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// Keep only options that are valid for a live `mount`. fstab-only options
/// such as `nofail` and the systemd timeout keys are dropped.
pub fn live_options(options: &[String]) -> Vec<String> {
    options
        .iter()
        .filter(|option| {
            if option.starts_with("x-systemd.") {
                return false;
            }
            let name = option.split('=').next().unwrap_or("");
            matches!(
                name,
                "subvol"
                    | "subvolid"
                    | "compress"
                    | "ro"
                    | "noatime"
                    | "uid"
                    | "gid"
                    | "umask"
                    | "fmask"
                    | "dmask"
                    | "discard"
                    | "ssd"
                    | "space_cache"
                    | "windows_names"
                    | "utf8"
                    | "shortname"
            )
        })
        .cloned()
        .collect()
}

/// Build the migration script. It runs as root, so every internal step is
/// already elevated and no nested `sudo` is used.
pub fn script(target: &str, device: &str, options: &[String]) -> String {
    let target = shell_quote(target);
    let device = shell_quote(device);
    let live = live_options(options);
    let mount_command = if live.is_empty() {
        "mount".to_string()
    } else {
        format!("mount -o {}", shell_quote(&live.join(",")))
    };
    format!(
        r#"set -euo pipefail
target={target}
device={device}
temp=/mnt/.nsg-migrate
if [ -e "$target.old" ]; then
  echo "$target.old already exists. Move it away before migrating."
  exit 1
fi
mkdir -p "$temp"
{mount_command} "$device" "$temp"
rsync -aHAX --numeric-ids --no-devices --no-specials "$target/" "$temp/"
changes=$(rsync -aHAX --numeric-ids --no-devices --no-specials --checksum --dry-run --itemize-changes "$target/" "$temp/" | grep -v '^\.d' || true)
if [ -n "$changes" ]; then
  printf '%s\n' "$changes"
  echo "The copy is not identical. The original was not touched."
  umount "$temp"
  exit 1
fi
umount "$temp"
mv "$target" "$target.old"
mkdir "$target"
{mount_command} "$device" "$target"
echo "Migration complete."
echo "Save and rebuild. Delete $target.old only after a successful boot."
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_single_quotes() {
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn drops_fstab_only_options() {
        let options = [
            "subvol=@data".to_string(),
            "nofail".to_string(),
            "x-systemd.automount".to_string(),
            "noatime".to_string(),
            "uid=1000".to_string(),
        ];
        assert_eq!(
            live_options(&options),
            vec![
                "subvol=@data".to_string(),
                "noatime".to_string(),
                "uid=1000".to_string()
            ]
        );
    }

    #[test]
    fn script_keeps_the_original_until_validation_passes() {
        let text = script(
            "/home/den/Documents",
            "/dev/disk/by-uuid/abc",
            &["noatime".to_string()],
        );
        assert!(text.contains("target='/home/den/Documents'"), "{text}");
        assert!(text.contains("rsync -aHAX"), "{text}");
        assert!(text.contains("--checksum"), "{text}");
        assert!(text.contains("exit 1"), "{text}");
        assert!(text.contains("mv \"$target\" \"$target.old\""), "{text}");
        assert!(text.contains("after a successful boot"), "{text}");
    }

    #[test]
    fn script_omits_mount_options_when_none_apply() {
        let text = script("/mnt/data", "/dev/sdb1", &["nofail".to_string()]);
        assert!(text.contains("mount \"$device\" \"$temp\""), "{text}");
    }
}
