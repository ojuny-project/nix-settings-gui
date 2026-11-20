//! Mountpoint guards shared by the widget and its tests.

use std::os::unix::fs::MetadataExt;

/// Paths a user mount may never cover.
pub const PROTECTED: &[&str] = &["/", "/nix", "/boot", "/efi", "/bin", "/lib", "/var", "/etc"];

/// Collapse a path lexically. Returns `None` for anything that is not absolute.
pub fn normalize(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in trimmed.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    Some(format!("/{}", parts.join("/")))
}

/// Resolve symlinks when the path exists, otherwise fall back to the lexical
/// form. A missing mountpoint still has to pass the guard.
pub fn resolve(path: &str) -> Option<String> {
    let normalized = normalize(path)?;
    match std::fs::canonicalize(&normalized) {
        Ok(canonical) => normalize(&canonical.to_string_lossy()),
        Err(_) => Some(normalized),
    }
}

pub fn is_protected(path: &str) -> bool {
    let Some(normalized) = normalize(path) else {
        return true;
    };
    protected_normalized(&normalized)
}

/// Resolve first, so `/mnt/link` pointing at `/nix` is rejected as well.
pub fn is_protected_resolved(path: &str) -> bool {
    let Some(resolved) = resolve(path) else {
        return true;
    };
    protected_normalized(&resolved)
}

fn protected_normalized(normalized: &str) -> bool {
    if normalized == "/" {
        return true;
    }
    PROTECTED
        .iter()
        .filter(|protected| **protected != "/")
        .any(|protected| {
            normalized == *protected || normalized.starts_with(&format!("{protected}/"))
        })
}

/// Why a target may or may not be used for a new mount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetState {
    Missing,
    Empty,
    UserOwned,
    Mounted,
}

/// Check a mountpoint against the protected list and the migration rule.
///
/// A missing or empty directory is fine. A non-empty directory is allowed only
/// when the current user owns it, so a migration can run without elevation.
/// A root-owned non-empty directory is rejected, which also covers system
/// directories that are not on the protected list.
pub fn check_target(path: &str) -> Result<TargetState, String> {
    let Some(normalized) = normalize(path) else {
        return Err(format!("{path} is not an absolute path"));
    };
    if is_protected_resolved(&normalized) {
        return Err(format!(
            "{normalized} is a protected path and cannot be mounted over"
        ));
    }
    let metadata = match std::fs::metadata(&normalized) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(TargetState::Missing),
    };
    if !metadata.is_dir() {
        return Err(format!("{normalized} is not a directory"));
    }
    if is_mountpoint(&normalized) {
        return Ok(TargetState::Mounted);
    }
    let mut entries = std::fs::read_dir(&normalized)
        .map_err(|error| format!("cannot read {normalized}: {error}"))?;
    if entries.next().is_none() {
        return Ok(TargetState::Empty);
    }
    if metadata.uid() == unsafe { libc::geteuid() } {
        Ok(TargetState::UserOwned)
    } else {
        Err(format!(
            "{normalized} is not empty and is not owned by the current user; a mount there would need elevated privileges to migrate"
        ))
    }
}

fn is_mountpoint(path: &str) -> bool {
    let Ok(mountinfo) = std::fs::read_to_string("/proc/self/mountinfo") else {
        return false;
    };
    mountinfo.lines().any(|line| {
        line.split_whitespace()
            .nth(4)
            .map(|mount| mount == path)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_trailing_slashes_and_dots() {
        assert_eq!(normalize("/mnt/data/").as_deref(), Some("/mnt/data"));
        assert_eq!(normalize("/mnt/./data").as_deref(), Some("/mnt/data"));
        assert_eq!(normalize("/mnt/a/../data").as_deref(), Some("/mnt/data"));
        assert_eq!(normalize("/").as_deref(), Some("/"));
        assert_eq!(normalize("relative/path"), None);
        assert_eq!(normalize(""), None);
    }

    #[test]
    fn rejects_protected_paths_on_boundaries() {
        assert!(is_protected("/"));
        assert!(is_protected("/nix"));
        assert!(is_protected("/nix/store"));
        assert!(is_protected("/boot/efi"));
        assert!(is_protected("/efi"));
        assert!(is_protected("/etc"));
        assert!(is_protected("/var/lib"));
        assert!(is_protected("/bin"));
        assert!(is_protected("/lib"));
        assert!(!is_protected("/nixos-data"));
        assert!(!is_protected("/bootleg"));
        assert!(!is_protected("/mnt/data"));
        assert!(!is_protected("/home/den/Documents"));
    }

    #[test]
    fn a_protected_prefix_without_a_separator_is_allowed() {
        assert!(!is_protected("/etcetera"));
        assert!(!is_protected("/variety"));
    }

    #[test]
    fn resolves_a_symlink_into_a_protected_path() {
        let base = std::env::temp_dir().join(format!("nsg-mounts-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let link = base.join("link");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/nix/store", &link).expect("symlink");
        let target = link.to_string_lossy().to_string();
        assert!(is_protected_resolved(&target), "{target}");
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_dir(&base);
    }

    #[test]
    fn target_rules_follow_ownership() {
        let base = std::env::temp_dir().join(format!("nsg-target-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("temp dir");
        let path = base.to_string_lossy().to_string();
        assert_eq!(check_target(&path), Ok(TargetState::Empty));
        let file = base.join("data");
        std::fs::write(&file, b"x").expect("write");
        assert_eq!(check_target(&path), Ok(TargetState::UserOwned));
        std::fs::remove_file(&file).expect("remove file");
        std::fs::remove_dir(&base).expect("remove dir");
        assert_eq!(
            check_target("/mnt/nsg-definitely-missing"),
            Ok(TargetState::Missing)
        );
        assert!(check_target("/var").is_err());
        assert!(check_target("/nix/store").is_err());
    }
}
