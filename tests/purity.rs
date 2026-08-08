//! Enforces the Dependency Rule of the Purity Boundary Map in
//! `SPEC-001-circus-agent-harness`: dependencies point inward, and `core` MUST
//! NOT import from `shell`.
//!
//! A comment saying so is not enforcement. This is.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d)
            .expect("readable source directory")
            .flatten()
        {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out
}

fn core_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core")
}

#[test]
fn core_does_not_import_shell() {
    let files = rust_files(&core_dir());
    assert!(!files.is_empty(), "no core sources found");

    let mut offenders = Vec::new();
    for f in &files {
        let body = fs::read_to_string(f).unwrap();
        for (n, line) in body.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let mentions_shell = code.contains("crate::shell")
                || code.contains("super::shell")
                || code.contains("circus::shell");
            if mentions_shell {
                offenders.push(format!("{}:{}: {}", f.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the pure core imports from the effectful shell:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn core_performs_no_process_or_filesystem_effects() {
    // The one sanctioned exception is `UnixSeconds::now`, which reads the
    // wall clock and is confined to a single constructor so every other
    // function in the core takes the instant as an argument.
    const BANNED: &[&str] = &[
        "std::process::Command",
        "Command::new",
        "std::fs::",
        "fs::read",
        "fs::write",
        "fs::File",
        "TcpStream",
        "std::env::",
    ];

    let mut offenders = Vec::new();
    for f in rust_files(&core_dir()) {
        let body = fs::read_to_string(&f).unwrap();
        // Test modules exercise the core against real shells and files on
        // purpose; the boundary constrains the library, not its tests.
        let library = body.split("#[cfg(test)]").next().unwrap_or("");
        for (n, line) in library.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for banned in BANNED {
                if code.contains(banned) {
                    offenders.push(format!("{}:{}: {}", f.display(), n + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the pure core performs an effect:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn every_core_module_is_reachable_from_the_map() {
    // The Purity Boundary Map names the core's members. A module that appears
    // in the source but not in the map is undocumented surface.
    let spec = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("specs/SPEC-001-circus-agent-harness.md"),
    )
    .expect("the specification is readable");
    let map = spec
        .split("## Purity Boundary Map")
        .nth(1)
        .expect("the specification has a Purity Boundary Map")
        .split("\n## ")
        .next()
        .unwrap();

    // `mod`, `time`, and `shquote` are supporting modules named in the map's
    // prose rather than as bullet entries; the rest must each appear.
    let named = ["paths", "prompt", "verifier", "decision", "record"];
    for m in named {
        assert!(
            map.contains(m),
            "core module `{m}` is missing from the Purity Boundary Map"
        );
    }
}
