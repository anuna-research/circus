//! `circus accept --auto-merge` and `circus spawn --auto-merge`/
//! `--force-merge`: fold a merge into the same run as acceptance (or, for
//! `--force-merge`, into the same run as the driver finishing at all), so
//! "merge on completion" needs no second command.
//!
//! Not part of `SPEC-001-circus-agent-harness`'s formal REQ/TEST numbering —
//! `tests/traceability.rs` only reads `tests/spec.rs`, deliberately, so this
//! file's tests are exempt from that mechanical π-totality check rather than
//! silently breaking it. These flags are convenience layered on top of
//! `accept` and `merge`, both already fully specified and tested; these
//! tests are about the composition, not a new contract.

mod common;

use std::fs;

use common::{Fx, INTEGRATION};

/// The positive case: an attempt whose verifier passes merges immediately,
/// and the one run record printed carries both outcomes.
#[test]
fn auto_merge_merges_immediately_on_accept() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    let wt = fx.worktree_of(&id);
    fs::write(wt.join("work.txt"), "work\n").unwrap();
    fx.git_in(&wt, &["add", "-A"]).ok();
    fx.git_in(&wt, &["commit", "-q", "-m", "work"]).ok();

    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(0).to_str().unwrap(),
        "--evidence",
        "theory:q1",
        "--auto-merge",
    ]);
    r.ok();

    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_ne!(before, after, "auto_merge did not move the integration ref");

    let rec = r.record();
    assert_eq!(rec["decision"], "accepted");
    assert_eq!(rec["state"], "merged");
    assert_eq!(rec["merge"]["result"], "merged");
    assert!(rec["merge"]["commit"].is_string());

    // Exactly one JSON document on stdout — `r.record()` above already
    // proves this (it fails on trailing data), stated here as the property
    // under test rather than an incidental side effect of another assertion.
}

/// A rejected attempt is never merged, auto_merge or not — the flag changes
/// when a merge is asked for, never whether one is allowed.
#[test]
fn auto_merge_does_not_merge_a_rejected_attempt() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(1).to_str().unwrap(),
        "--evidence",
        "theory:q1",
        "--auto-merge",
    ]);

    assert_ne!(r.code, 0, "a rejected verifier must not report success");
    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_eq!(
        before, after,
        "a rejected attempt moved the integration ref"
    );

    let rec = r.record();
    assert_eq!(rec["decision"], "rejected");
    assert_eq!(rec["state"], "rejected");
    assert!(
        rec["merge"].is_null(),
        "a rejected attempt recorded a merge"
    );
}

/// A conflicting merge is still reported as a conflict under `--auto-merge`,
/// not silently swallowed — same outcome `circus merge` alone would give,
/// just reached without a second command.
#[test]
fn auto_merge_reports_a_conflict_and_leaves_the_ref_alone() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");

    let wt = fx.worktree_of(&id);
    fs::write(wt.join("shared.txt"), "from the worker\n").unwrap();
    fx.git_in(&wt, &["add", "-A"]).ok();
    fx.git_in(&wt, &["commit", "-q", "-m", "worker"]).ok();

    fx.git(&["checkout", "-q", INTEGRATION]).ok();
    fs::write(fx.repo.join("shared.txt"), "from the trunk\n").unwrap();
    fx.git(&["add", "-A"]).ok();
    fx.git(&["commit", "-q", "-m", "trunk"]).ok();

    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(0).to_str().unwrap(),
        "--evidence",
        "theory:q1",
        "--auto-merge",
    ]);

    assert_ne!(r.code, 0, "a conflict must not report success");
    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_eq!(
        before, after,
        "a conflicting auto_merge moved the integration ref"
    );

    let rec = r.record();
    assert_eq!(
        rec["decision"], "accepted",
        "the accept half still succeeded"
    );
    assert_eq!(
        rec["state"], "accepted",
        "a conflicted merge must not read as merged"
    );
    assert_eq!(rec["merge"]["result"], "conflict");
    assert_eq!(
        fx.git_in(&wt, &["status", "--porcelain"]).stdout.trim(),
        "",
        "the attempt worktree was left conflicted"
    );
}

/// Without the flag, nothing changes from before `--auto-merge` existed:
/// acceptance alone never merges.
#[test]
fn accept_without_auto_merge_does_not_merge() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    let wt = fx.worktree_of(&id);
    fs::write(wt.join("work.txt"), "work\n").unwrap();
    fx.git_in(&wt, &["add", "-A"]).ok();
    fx.git_in(&wt, &["commit", "-q", "-m", "work"]).ok();

    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(0).to_str().unwrap(),
        "--evidence",
        "theory:q1",
    ]);
    r.ok();

    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_eq!(before, after);
    let rec = r.record();
    assert_eq!(rec["state"], "accepted");
    assert!(rec["merge"].is_null());
}

// ── `circus spawn --auto-merge` / `--force-merge` ──────────────────────────
//
// `spawn` doesn't know the attempt id until it runs, unlike
// `prepare_and_complete`'s scripts above (which already have one from a
// separate `prepare` call) — so these driver scripts read `$CIRCUS_SENTINEL`
// at run time instead of a path baked in ahead of it, per drivers/README.md's
// documented contract.

/// A driver that commits a real change and reports success.
fn ok_driver(fx: &Fx, name: &str) -> std::path::PathBuf {
    fx.script(
        name,
        "echo out; echo err >&2\n\
         echo work > work.txt\n\
         git add -A\n\
         git commit -q -m work\n\
         echo 0 > \"$CIRCUS_SENTINEL\"\n\
         sleep 30\n",
    )
}

#[test]
fn spawn_auto_merge_merges_when_the_verifier_passes() {
    let fx = Fx::new();
    let task_file = fx.raw_file("task.md", "do the work\n");
    let driver = ok_driver(&fx, "drv-ok");
    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&[
        "spawn",
        "--task",
        "model",
        "--integration",
        INTEGRATION,
        "--task-file",
        task_file.to_str().unwrap(),
        "--verifier",
        "true",
        "--evidence",
        "theory:q1",
        "--auto-merge",
        "--",
        driver.to_str().unwrap(),
    ]);
    r.ok();

    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_ne!(
        before, after,
        "spawn --auto-merge did not move the integration ref"
    );

    let rec = r.record();
    assert_eq!(rec["decision"], "accepted");
    assert_eq!(rec["state"], "merged");
    assert_eq!(rec["merge"]["result"], "merged");
    assert_eq!(rec["verifier"]["command"], serde_json::json!(["true"]));
    assert_eq!(rec["evidence_refs"], serde_json::json!(["theory:q1"]));
}

#[test]
fn spawn_auto_merge_does_not_merge_when_the_verifier_fails() {
    let fx = Fx::new();
    let task_file = fx.raw_file("task.md", "do the work\n");
    let driver = ok_driver(&fx, "drv-ok2");
    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&[
        "spawn",
        "--task",
        "model",
        "--integration",
        INTEGRATION,
        "--task-file",
        task_file.to_str().unwrap(),
        "--verifier",
        "false",
        "--evidence",
        "theory:q1",
        "--auto-merge",
        "--",
        driver.to_str().unwrap(),
    ]);

    assert_ne!(r.code, 0, "a failing verifier must not report success");
    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_eq!(
        before, after,
        "a rejected auto_merge attempt moved the integration ref"
    );

    let rec = r.record();
    assert_eq!(rec["decision"], "rejected");
    assert_eq!(rec["state"], "rejected");
    assert!(rec["merge"].is_null());
}

#[test]
fn spawn_force_merge_merges_the_moment_the_driver_exits_zero() {
    let fx = Fx::new();
    let task_file = fx.raw_file("task.md", "do the work\n");
    let driver = ok_driver(&fx, "drv-ok3");
    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&[
        "spawn",
        "--task",
        "model",
        "--integration",
        INTEGRATION,
        "--task-file",
        task_file.to_str().unwrap(),
        "--force-merge",
        "--",
        driver.to_str().unwrap(),
    ]);
    r.ok();

    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_ne!(
        before, after,
        "spawn --force-merge did not move the integration ref"
    );

    let rec = r.record();
    assert_eq!(rec["state"], "merged");
    assert_eq!(rec["merge"]["result"], "merged");
    // The whole point: no verifier ran, and nothing calls itself evidence.
    assert!(rec["decision"].is_null());
    assert!(rec["verifier"].is_null());
    assert!(rec["evidence_refs"].as_array().unwrap().is_empty());
}

#[test]
fn spawn_force_merge_skips_when_the_driver_exits_nonzero() {
    let fx = Fx::new();
    let task_file = fx.raw_file("task.md", "do the work\n");
    let driver = fx.script("drv-fail", "exit 7\n");
    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&[
        "spawn",
        "--task",
        "model",
        "--integration",
        INTEGRATION,
        "--task-file",
        task_file.to_str().unwrap(),
        "--force-merge",
        "--",
        driver.to_str().unwrap(),
    ]);

    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_eq!(
        before, after,
        "--force-merge merged despite the driver exiting nonzero"
    );

    let rec = r.record();
    assert_ne!(rec["state"], "merged");
    assert!(rec["merge"].is_null());
}
