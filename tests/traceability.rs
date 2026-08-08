//! Checks the requirement-attribution map π mechanically, against the
//! specification and the test suite as they exist on disk.
//!
//! PROTO-001 requires π to be total in both directions: every requirement atom
//! reaches at least one TEST, and every TEST attributes at least one atom. A
//! traceability claim that is only asserted in prose is the "asserted
//! convergence" anti-pattern; this file is the mechanism that decides it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn spec() -> String {
    fs::read_to_string(root().join("specs/SPEC-001-circus-agent-harness.md"))
        .expect("the specification is readable")
}

fn suite() -> String {
    fs::read_to_string(root().join("tests/spec.rs")).expect("the suite is readable")
}

/// Atoms declared in bold, e.g. `**REQ-004.c**`, plus the temporal atoms of
/// NFR-002, which are declared inside its formula block.
fn declared_atoms(spec: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = spec;
    while let Some(i) = rest.find("**") {
        rest = &rest[i + 2..];
        let Some(j) = rest.find("**") else { break };
        let inner = &rest[..j];
        rest = &rest[j + 2..];
        if is_atom(inner) {
            out.insert(inner.to_owned());
        }
    }
    // NFR-002.a and NFR-002.b are named atoms of a temporal property.
    for line in spec.lines() {
        let t = line.trim();
        if let Some(name) = t.split_whitespace().next()
            && is_atom(name)
            && t.contains('=')
        {
            out.insert(name.to_owned());
        }
    }
    out
}

fn is_atom(s: &str) -> bool {
    let Some((head, tail)) = s.split_once('.') else {
        return false;
    };
    (head.starts_with("REQ-") || head.starts_with("NFR-"))
        && head.len() == 7
        && head[4..].chars().all(|c| c.is_ascii_digit())
        && tail.len() == 1
        && tail.chars().all(|c| c.is_ascii_lowercase())
}

/// Atoms cited by a TEST entry, written `[[…#REQ-004]].c`.
fn atoms_cited_by_tests(spec: &str) -> BTreeSet<String> {
    let tests = spec
        .split("\n## Tests")
        .nth(1)
        .expect("a Tests section")
        .split("\n## ")
        .next()
        .unwrap();
    let mut out = BTreeSet::new();
    for (i, _) in tests
        .match_indices("#REQ-")
        .chain(tests.match_indices("#NFR-"))
    {
        let tail = &tests[i + 1..];
        let Some(close) = tail.find("]]") else {
            continue;
        };
        let id = &tail[..close];
        let after = &tail[close + 2..];
        if let Some(letter) = after.strip_prefix('.').and_then(|s| s.chars().next())
            && letter.is_ascii_lowercase()
        {
            out.insert(format!("{id}.{letter}"));
        }
    }
    out
}

fn spec_test_ids(spec: &str) -> BTreeSet<String> {
    spec.lines()
        .filter_map(|l| l.strip_prefix("### TEST-"))
        .filter_map(|l| l.split(':').next())
        .map(|n| format!("TEST-{n}"))
        .collect()
}

fn implemented_test_ids(suite: &str) -> BTreeSet<String> {
    suite
        .lines()
        .filter_map(|l| l.trim().strip_prefix("fn test_"))
        .filter_map(|l| l.get(..3))
        .filter(|n| n.chars().all(|c| c.is_ascii_digit()))
        .map(|n| format!("TEST-{n}"))
        .collect()
}

#[test]
fn every_requirement_atom_is_validated_by_a_test() {
    let spec = spec();
    let declared = declared_atoms(&spec);
    let cited = atoms_cited_by_tests(&spec);
    assert!(
        declared.len() >= 30,
        "only {} atoms found; the parser is wrong",
        declared.len()
    );

    let uncovered: Vec<_> = declared.difference(&cited).cloned().collect();
    assert!(
        uncovered.is_empty(),
        "π is not total: these atoms reach no TEST: {uncovered:?}"
    );
}

#[test]
fn every_cited_atom_is_actually_declared() {
    let spec = spec();
    let declared = declared_atoms(&spec);
    let cited = atoms_cited_by_tests(&spec);
    let dangling: Vec<_> = cited.difference(&declared).cloned().collect();
    assert!(
        dangling.is_empty(),
        "these TESTs validate atoms that no requirement declares: {dangling:?}"
    );
}

#[test]
fn every_specified_test_is_implemented() {
    let specified = spec_test_ids(&spec());
    let implemented = implemented_test_ids(&suite());
    assert!(
        !specified.is_empty(),
        "no TEST entries found in the specification"
    );

    let missing: Vec<_> = specified.difference(&implemented).cloned().collect();
    assert!(
        missing.is_empty(),
        "specified but not implemented: {missing:?}"
    );
}

#[test]
fn every_implemented_test_is_specified() {
    let specified = spec_test_ids(&spec());
    let implemented = implemented_test_ids(&suite());
    let extra: Vec<_> = implemented.difference(&specified).cloned().collect();
    assert!(extra.is_empty(), "implemented but not specified: {extra:?}");
}

#[test]
fn every_test_entry_attributes_its_target() {
    let spec = spec();
    let tests = spec
        .split("\n## Tests")
        .nth(1)
        .unwrap()
        .split("\n## ")
        .next()
        .unwrap();
    for block in tests.split("\n### ").skip(1) {
        let title = block.lines().next().unwrap_or("");
        assert!(
            block.contains("Validates:"),
            "TEST entry `{title}` has no `Validates:` field"
        );
    }
}

#[test]
fn every_contract_declares_its_parts() {
    let spec = spec();
    let contracts = spec
        .split("\n## Contracts")
        .nth(1)
        .expect("a Contracts section")
        .split("\n## ")
        .next()
        .unwrap();
    for block in contracts.split("\n### ").skip(1) {
        let title = block.lines().next().unwrap_or("").to_owned();
        for part in [
            "Pre-conditions",
            "Post-conditions",
            "Error model",
            "Implements",
            "Verified by",
        ] {
            assert!(
                block.contains(part),
                "contract `{title}` has no `{part}` section"
            );
        }
    }
}

#[test]
fn every_control_in_the_orientation_digest_resolves() {
    // The Controls digest is the artefact re-read before a consequential
    // action, so a stale entry there is worse than none.
    let spec = spec();
    let orientation = spec
        .split("Controls:")
        .nth(1)
        .expect("an Orientation Controls digest")
        .split("\nOpen:")
        .next()
        .unwrap();
    let mut count = 0;
    for line in orientation
        .lines()
        .filter(|l| l.trim_start().starts_with("- "))
    {
        count += 1;
        assert!(
            line.contains("[["),
            "control line without a wikilink: {line}"
        );
    }
    assert!(
        count >= 5,
        "the Controls digest looks truncated: {count} entries"
    );
}

#[test]
fn the_state_root_layout_matches_the_decision_record() {
    // ADR-006 fixes the layout the implementation derives paths from.
    let spec = spec();
    let adr = spec
        .split("### ADR-006")
        .nth(1)
        .expect("ADR-006")
        .split("\n### ")
        .next()
        .unwrap();
    for expected in [
        "record.json",
        "transcript.log",
        "sentinel",
        ".lock",
        "git-common-dir",
    ] {
        assert!(
            adr.contains(expected),
            "ADR-006 no longer names `{expected}`"
        );
    }
    let paths = fs::read_to_string(root().join("src/core/paths.rs")).unwrap();
    for expected in ["record.json", "transcript.log", "sentinel", ".lock"] {
        assert!(
            paths.contains(expected),
            "the implementation no longer derives `{expected}`"
        );
    }
}

#[test]
fn no_specification_id_is_referenced_in_plain_text() {
    // PROTO-001: every cross-reference is a wikilink. A plain-text reference
    // is invisible debt that `zetl check --dead-links` cannot see.
    let spec = spec();
    let mut offenders = Vec::new();
    let mut in_code = false;
    for (n, line) in spec.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        for prefix in ["REQ-", "NFR-", "CON-", "ADR-", "TEST-", "OBS-"] {
            let mut rest: &str = line;
            while let Some(i) = rest.find(prefix) {
                let before = &rest[..i];
                let is_linked = before.ends_with("[[")
                    || before.ends_with('#')
                    || before.ends_with("**")
                    || before.ends_with('`');
                let is_heading = line.starts_with("### ") || line.starts_with("#### ");
                if !is_linked && !is_heading {
                    offenders.push(format!("{}: {}", n + 1, line.trim()));
                    break;
                }
                rest = &rest[i + prefix.len()..];
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "plain-text artefact references (should be wikilinks):\n{}",
        offenders.join("\n")
    );
}

fn _unused(_: &Path) {}
