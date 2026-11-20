//! Mountpoint guards shared by the widget and its tests.

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
}
