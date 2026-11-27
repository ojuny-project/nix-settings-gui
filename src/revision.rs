//! The source revision this binary was built from.
//!
//! The flake sets `NSG_SOURCE_REVISION` when it builds the package, so a user
//! can compare the running revision with the revision that contains a fix.
//! Local development builds fall back to the package version and the compiled
//! target.

pub fn running() -> String {
    if let Some(revision) = option_env!("NSG_SOURCE_REVISION") {
        let revision = revision.trim();
        if !revision.is_empty() {
            return revision.to_string();
        }
    }
    format!(
        "{} ({})",
        env!("CARGO_PKG_VERSION"),
        option_env!("TARGET").unwrap_or("unknown target")
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn running_revision_is_never_empty() {
        assert!(!super::running().trim().is_empty());
    }
}
