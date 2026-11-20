//! Gate: a module declared by the library must also be declared by the binary
//! when the binary's own source can reach it. Adding `mounts` to `lib.rs` but
//! not to `main.rs` broke the Nix build once.

use std::path::Path;

fn modules(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line
                .strip_prefix("pub mod ")
                .or_else(|| line.strip_prefix("mod "))?;
            Some(rest.trim_end_matches(';').trim().to_string())
        })
        .collect()
}

#[test]
fn every_library_module_is_declared_in_the_binary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let library = std::fs::read_to_string(root.join("src/lib.rs")).expect("read lib.rs");
    let binary = std::fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let binary_modules = modules(&binary);

    for name in modules(&library) {
        if name == "headless" {
            continue;
        }
        assert!(
            binary_modules.contains(&name),
            "src/lib.rs declares {name} but src/main.rs does not"
        );
    }
}
