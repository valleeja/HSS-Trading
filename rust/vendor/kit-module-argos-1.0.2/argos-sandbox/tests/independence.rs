//! The sandbox and its policy crate must never depend on a private Argos crate
//! (keys, ledgers, votes, detection, app), so they can be handed to the
//! projects on their own. Allow-list: the only Argos crates they may use are
//! each other and the MIT module kit, by path, and nothing comes from git.

use std::path::Path;

/// The only Argos crates the sandbox and its policy crate may depend on.
const ALLOWED: [&str; 2] = ["argos-module-policy", "argos-module-sdk"];

fn manifest(crate_dir: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_dir)
        .join("Cargo.toml");
    std::fs::read_to_string(path).expect("Cargo.toml")
}

/// Names declared in every dependency section of a manifest.
fn dependency_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_deps = line.ends_with("dependencies]");
            continue;
        }
        if in_deps {
            if let Some((name, _)) = line.split_once('=') {
                names.push(name.trim().to_owned());
            }
        }
    }
    names
}

#[test]
fn no_private_crate_in_the_sandbox_or_its_policy() {
    for crate_dir in ["argos-sandbox", "argos-module-policy"] {
        let text = manifest(crate_dir);
        for name in dependency_names(&text) {
            assert!(
                !name.starts_with("argos") || ALLOWED.contains(&name.as_str()),
                "{crate_dir} depends on the Argos crate {name}"
            );
        }
        for line in text.lines().filter(|l| l.contains("path =")) {
            assert!(
                ALLOWED.iter().any(|a| line.trim_start().starts_with(a)),
                "{crate_dir}: {line}"
            );
        }
        assert!(!text.contains("git ="), "{crate_dir}: no git dependency");
    }
    // The kit itself depends on nothing at all.
    let sdk = manifest("argos-module-sdk");
    let dependencies = sdk
        .split("\n[")
        .find(|section| section.starts_with("dependencies]"));
    assert_eq!(
        dependencies, None,
        "argos-module-sdk must have no dependency"
    );
}

#[test]
fn the_licences_are_declared() {
    let dir = |crate_dir: &str| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(crate_dir)
    };
    for crate_dir in ["argos-sandbox", "argos-module-policy"] {
        assert!(manifest(crate_dir).contains(r#"license = "PolyForm-Noncommercial-1.0.0""#));
        // The official text, unchanged, with the required notice, and the commercial terms.
        let text = std::fs::read_to_string(dir(crate_dir).join("LICENSE.md")).expect("LICENSE.md");
        assert!(text.starts_with("# PolyForm Noncommercial License 1.0.0"));
        assert!(text.contains("Required Notice: Copyright (c) 2026 Jee (Argos)"));
        let commercial =
            std::fs::read_to_string(dir(crate_dir).join("COMMERCIAL.md")).expect("COMMERCIAL.md");
        assert!(commercial.contains("maintenance keys"));
    }
    assert!(manifest("argos-module-sdk").contains(r#"license = "MIT OR Apache-2.0""#));
}
