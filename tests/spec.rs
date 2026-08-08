//! The `SPEC-001-circus-agent-harness` test suite: TEST-001 through TEST-033.
//!
//! One test per specification entry, named for it, attributing the requirement
//! atoms it validates. The requirement-attribution map π is total in both
//! directions, and `tests/traceability.rs` checks that mechanically rather
//! than trusting this comment.

mod common;

use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

use common::{Fx, INTEGRATION, count_entries};

/// Wait for a condition, or fail. Used wherever a tmux pane's existence is the
/// thing under test and the launch is still running.
fn wait_until(what: &str, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if f() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {what}");
}

// ── REQ-001: Isolated Task Worktree ────────────────────────────────────────

/// TEST-001 — REQ-001.a, REQ-001.b, CON-001. Positive, exact-count.
#[test]
fn test_001_creates_exactly_one_worktree_and_branch() {
    let fx = Fx::new();
    let worktrees_before = fx.git(&["worktree", "list"]).stdout.lines().count();
    let branches_before = fx.git(&["branch", "--list"]).stdout.lines().count();

    let r = fx.circus(&["prepare", "--task", "model", "--integration", INTEGRATION]);
    r.ok();
    let rec = r.record();

    assert_eq!(rec["attempt_id"], "model/1");
    assert_eq!(rec["state"], "prepared");
    assert_eq!(
        fx.git(&["worktree", "list"]).stdout.lines().count(),
        worktrees_before + 1,
        "exactly one worktree added"
    );
    assert_eq!(
        fx.git(&["branch", "--list"]).stdout.lines().count(),
        branches_before + 1,
        "exactly one branch added"
    );

    // REQ-001.b — the attempt starts from the integration ref.
    let tip = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    let branch = rec["branch"].as_str().unwrap();
    assert_eq!(fx.git(&["rev-parse", branch]).stdout.trim(), tip);
}

/// TEST-002 — REQ-001.d. Scope-invariant.
#[test]
fn test_002_preserves_the_caller_worktree() {
    let fx = Fx::new();
    fs::write(fx.repo.join("dirty.txt"), "uncommitted\n").unwrap();
    fs::write(fx.repo.join("README.md"), "modified\n").unwrap();
    fx.git(&["add", "README.md"]).ok();

    let head = fx.git(&["rev-parse", "HEAD"]).stdout;
    let status = fx.git(&["status", "--porcelain"]).stdout;
    let readme = fs::read(fx.repo.join("README.md")).unwrap();

    fx.circus(&["prepare", "--task", "model", "--integration", INTEGRATION])
        .ok();

    assert_eq!(fx.git(&["rev-parse", "HEAD"]).stdout, head, "HEAD moved");
    assert_eq!(
        fx.git(&["status", "--porcelain"]).stdout,
        status,
        "index or tree changed"
    );
    assert_eq!(fs::read(fx.repo.join("README.md")).unwrap(), readme);
    assert!(fx.repo.join("dirty.txt").exists());
}

/// TEST-015 — REQ-001.c. Positive.
#[test]
fn test_015_names_the_worktree_path() {
    let fx = Fx::new();
    let id = fx.prepare("model");
    let wt = fx.worktree_of(&id).to_string_lossy().into_owned();
    assert!(wt.contains("circus"), "no repository name in {wt}");
    assert!(wt.contains("model"), "no task in {wt}");
    assert!(wt.contains('1'), "no attempt number in {wt}");
}

/// TEST-016 — CON-001 error model. Negative, rollback.
#[test]
fn test_016_rolls_back_a_partial_preparation() {
    let fx = Fx::new();
    let real_git = common::which("git").unwrap();

    // A Git that creates the branch and then fails the worktree add, which is
    // the partial state CON-001 says must not survive.
    let dir = fx.bin.join("brokengit");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("git"),
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = worktree ] && [ \"$2\" = add ]; then\n\
             \x20 \"{git}\" branch \"$4\" \"$6\" >/dev/null 2>&1\n\
             \x20 echo 'simulated worktree failure' >&2\n\
             \x20 exit 1\n\
             fi\n\
             exec \"{git}\" \"$@\"\n",
            git = real_git.display()
        ),
    )
    .unwrap();
    fs::set_permissions(
        dir.join("git"),
        <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
    )
    .unwrap();

    let path = format!("{}:{}", dir.display(), std::env::var("PATH").unwrap());
    let r = fx.circus_env(
        &["prepare", "--task", "model", "--integration", INTEGRATION],
        &[("PATH", &path)],
    );

    assert_eq!(r.code, 70, "a Git failure is exit 70; stderr: {}", r.stderr);
    assert!(
        !fx.git(&["branch", "--list", "circus/model/1"])
            .stdout
            .contains("circus/model/1"),
        "the branch survived a failed preparation"
    );
    assert!(
        !fx.attempt_dir("model/1").exists(),
        "the attempt directory survived"
    );
    assert!(
        !fx.repo.parent().unwrap().join("circus-model-1").exists(),
        "the worktree directory survived"
    );
}

/// TEST-025 — CON-001 concurrency, ADR-006. Concurrency.
#[test]
fn test_025_serialises_concurrent_preparations() {
    let fx = Fx::new();
    let a = fx.circus_spawn(&["prepare", "--task", "model", "--integration", INTEGRATION]);
    let b = fx.circus_spawn(&["prepare", "--task", "model", "--integration", INTEGRATION]);
    let ra = a.wait_with_output().unwrap();
    let rb = b.wait_with_output().unwrap();

    assert!(
        ra.status.success() && rb.status.success(),
        "both preparations should succeed"
    );
    let ids: Vec<String> = [&ra, &rb]
        .iter()
        .map(|o| {
            let v: serde_json::Value = serde_json::from_slice(&o.stdout).expect("a run record");
            v["attempt_id"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_ne!(
        ids[0], ids[1],
        "two preparations chose the same attempt number"
    );

    assert_eq!(
        fx.git(&["worktree", "list"]).stdout.lines().count(),
        3,
        "one main worktree plus two attempts"
    );
    for id in &ids {
        assert!(fx.record_exists(id));
        assert!(fx.worktree_of(id).exists());
    }
}

// ── REQ-002 / REQ-003: pane, log, explicit completion ──────────────────────

/// TEST-003 — REQ-002.a, REQ-002.b. Positive.
#[test]
fn test_003_starts_a_named_tmux_pane() {
    let fx = Fx::new();
    let id = fx.prepare("model");
    let sentinel = fx.sentinel_of(&id);
    let drv = fx.script("slow", "sleep 60\n");

    let mut child = fx.circus_spawn(&[
        "launch",
        "--attempt",
        &id,
        "--prompt",
        fx.prompt(&id).to_str().unwrap(),
        "--",
        drv.to_str().unwrap(),
    ]);

    wait_until("the named pane to appear", || {
        fx.tmux(&["list-windows", "-a", "-F", "#{window_name}"])
            .stdout
            .lines()
            .any(|l| l.trim() == "circus-model-1")
    });

    // REQ-002.b — the name carries task and attempt.
    let names = fx
        .tmux(&["list-windows", "-a", "-F", "#{window_name}"])
        .stdout;
    let pane = names
        .lines()
        .find(|l| l.trim().starts_with("circus-"))
        .unwrap();
    assert!(pane.contains("model"), "pane name lacks the task: {pane}");
    assert!(pane.contains('1'), "pane name lacks the attempt: {pane}");

    fs::write(&sentinel, "0\n").unwrap();
    let out = child.wait().unwrap();
    assert!(
        out.success(),
        "launch should finish once the sentinel is written"
    );
}

/// TEST-004 — REQ-003.c. Negative-input.
#[test]
fn test_004_requires_the_literal_sentinel_in_the_prompt() {
    let fx = Fx::new();
    let id = fx.prepare("model");
    let bad = fx.raw_file("no-sentinel.md", "Do the work and stop.\n");
    let drv = fx.script("never", "echo SHOULD-NOT-RUN > /dev/stderr\n");

    let r = fx.circus(&[
        "launch",
        "--attempt",
        &id,
        "--prompt",
        bad.to_str().unwrap(),
        "--",
        drv.to_str().unwrap(),
    ]);

    assert_eq!(r.code, 64, "stderr: {}", r.stderr);
    assert!(
        fx.tmux(&["list-windows", "-a"]).code != 0
            || !fx.tmux(&["list-windows", "-a"]).stdout.contains("circus-"),
        "a pane was created for a rejected prompt"
    );
    assert!(
        !fx.log_of(&id).exists(),
        "a log was created for a rejected prompt"
    );
    assert_eq!(
        fx.record(&id)["state"],
        "prepared",
        "the attempt state moved"
    );
}

/// TEST-005 — REQ-003.a, NFR-002.a, NFR-002.b. Positive.
#[test]
fn test_005_terminates_after_explicit_completion() {
    let fx = Fx::new();
    let id = fx.prepare("model");
    let marker = fx.bin.join("child-alive");
    // A driver that spawns a child, writes the sentinel, and then keeps
    // running: withdone's kill is what must end the group.
    let drv = fx.script(
        "spawner",
        &format!(
            "( while true; do touch {m}; sleep 0.2; done ) &\n\
             sleep 0.3\n\
             echo 0 > {s}\n\
             sleep 60\n",
            m = marker.display(),
            s = fx.sentinel_of(&id).display()
        ),
    );

    let r = fx.circus(&[
        "launch",
        "--attempt",
        &id,
        "--prompt",
        fx.prompt(&id).to_str().unwrap(),
        "--",
        drv.to_str().unwrap(),
    ]);
    r.ok();

    let rec = fx.record(&id);
    assert_eq!(rec["completion_method"], "sentinel", "record: {rec}");
    assert_eq!(rec["sentinel_value"], 0);
    assert_eq!(
        rec["process_group_residue"], 0,
        "the process group was not reaped"
    );
    assert_eq!(rec["state"], "completed");

    // Nothing in the group is still touching the marker.
    let seen = fs::metadata(&marker).map(|m| m.modified().unwrap()).ok();
    sleep(Duration::from_millis(600));
    let now = fs::metadata(&marker).map(|m| m.modified().unwrap()).ok();
    assert_eq!(
        seen, now,
        "a member of the fixture process group is still alive"
    );
}

/// TEST-017 — REQ-002.c, REQ-002.d. Scope-invariant.
#[test]
fn test_017_keeps_the_attempt_log_outside_the_worktree() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");

    let log = fx.log_of(&id);
    let body = fs::read_to_string(&log).unwrap();
    assert!(
        body.contains("out"),
        "stdout missing from the log: {body:?}"
    );
    assert!(
        body.contains("err"),
        "stderr missing from the log: {body:?}"
    );

    let wt = fx.worktree_of(&id);
    assert!(!log.starts_with(&wt), "the log is inside the worktree");
    assert_eq!(
        fx.git_in(&wt, &["status", "--porcelain"]).stdout.trim(),
        "",
        "the worktree gained a file"
    );
}

/// TEST-018 — REQ-003.b. Scope-invariant.
#[test]
fn test_018_keeps_the_sentinel_outside_the_worktree() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    let wt = fx.worktree_of(&id);
    let sentinel = Path::new(fx.record(&id)["sentinel_path"].as_str().unwrap()).to_path_buf();

    assert!(
        !sentinel.starts_with(&wt),
        "the sentinel is inside the worktree"
    );
    assert_eq!(
        fx.git_in(&wt, &["status", "--porcelain"]).stdout.trim(),
        "",
        "an untracked sentinel appeared in the worktree"
    );
}

/// TEST-019 — REQ-003.d, REQ-007.b. Prohibited-action.
#[test]
fn test_019_passes_the_prompt_through_unchanged() {
    let fx = Fx::new();
    let id = fx.prepare("model");

    // Bytes a templating layer would be tempted to touch.
    let body = format!(
        "\u{feff}line one\r\n\t{{{{handlebars}}}} $SHELL `id`\nsentinel: {}\n\u{1f409}\n",
        fx.sentinel_of(&id).display()
    );
    let prompt = fx.raw_file("exotic.md", &body);
    let copy = fx.bin.join("seen-by-driver");
    let drv = fx.script(
        "copier",
        &format!(
            "cat \"$CIRCUS_PROMPT\" > {c}\necho 0 > {s}\nsleep 30\n",
            c = copy.display(),
            s = fx.sentinel_of(&id).display()
        ),
    );

    fx.circus(&[
        "launch",
        "--attempt",
        &id,
        "--prompt",
        prompt.to_str().unwrap(),
        "--",
        drv.to_str().unwrap(),
    ])
    .ok();

    assert_eq!(
        fs::read(&copy).unwrap(),
        fs::read(&prompt).unwrap(),
        "the driver did not receive the caller's bytes"
    );
}

/// TEST-027 — CON-002 error model. Negative-input.
#[test]
fn test_027_rejects_relaunch_of_a_launched_attempt() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    let log_before = fs::read(fx.log_of(&id)).unwrap();
    let drv = fx.script("again", "echo second run\n");

    let r = fx.circus(&[
        "launch",
        "--attempt",
        &id,
        "--prompt",
        fx.prompt(&id).to_str().unwrap(),
        "--",
        drv.to_str().unwrap(),
    ]);

    assert_eq!(r.code, 64, "stderr: {}", r.stderr);
    assert_eq!(
        fs::read(fx.log_of(&id)).unwrap(),
        log_before,
        "the log changed"
    );
    assert!(
        !fx.tmux(&["list-windows", "-a", "-F", "#{window_name}"])
            .stdout
            .contains("circus-model-1"),
        "a second pane was created"
    );
}

/// TEST-030 — NFR-002.c. Negative.
#[test]
fn test_030_records_a_failed_attempt_on_cleanup_timeout() {
    let fx = Fx::new();
    let id = fx.prepare("model");

    // A withdone that runs the child but reaps nothing. Circus's obligation is
    // to *measure* cleanup and fail the attempt when it did not happen; making
    // the real withdone leave residue is not possible, since it finishes with
    // SIGKILL.
    let dir = fx.bin.join("stubwd");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("withdone"),
        "#!/bin/sh\nwhile [ \"$1\" != \"--\" ]; do shift; done\nshift\n\"$@\"\nexit $?\n",
    )
    .unwrap();
    fs::set_permissions(
        dir.join("withdone"),
        <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
    )
    .unwrap();

    // The child must survive two reapers to leave residue. withdone is stubbed
    // out above, and tmux sends SIGHUP to the pane's process group when the
    // pane dies, so the fixture ignores it. A process that shrugs off both is
    // exactly the condition NFR-002.c exists to record.
    let lingerer = fx.bin.join("lingering");
    let drv = fx.script(
        "leaky",
        &format!(
            "( trap '' HUP TERM INT; sleep 45; touch {} ) &\nexit 0\n",
            lingerer.display()
        ),
    );

    let path = format!("{}:{}", dir.display(), std::env::var("PATH").unwrap());
    let r = fx.circus_env(
        &[
            "launch",
            "--attempt",
            &id,
            "--prompt",
            fx.prompt(&id).to_str().unwrap(),
            "--",
            drv.to_str().unwrap(),
        ],
        &[("PATH", &path)],
    );

    let rec = fx.record(&id);
    assert_eq!(rec["state"], "failed", "record: {rec}");
    assert!(
        rec["process_group_residue"].as_u64().unwrap() > 0,
        "residue should be recorded: {rec}"
    );
    assert_eq!(r.code, 1, "an unreaped attempt is not a success");
}

// ── REQ-004: the acceptance gate ───────────────────────────────────────────

/// TEST-006 — REQ-004.b. Prohibited-action.
#[test]
fn test_006_rejects_a_transport_only_outcome() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    assert_eq!(
        fx.record(&id)["transport_exit_code"],
        0,
        "the driver exited zero"
    );

    let r = fx.circus(&["accept", "--attempt", &id, "--evidence", "theory:q1"]);

    assert_eq!(r.code, 64, "stderr: {}", r.stderr);
    let rec = fx.record(&id);
    assert_eq!(rec["state"], "completed");
    assert!(
        rec["decision"].is_null(),
        "an exit code marked the attempt: {rec}"
    );
}

/// TEST-007 — REQ-004.c. Negative-input.
#[test]
fn test_007_rejects_a_failed_verifier() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    let wt = fx.worktree_of(&id);

    let r = fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(1).to_str().unwrap(),
        "--evidence",
        "theory:spec-001/q1",
    ]);

    assert_eq!(r.code, 1, "stderr: {}", r.stderr);
    let rec = fx.record(&id);
    assert_eq!(rec["decision"], "rejected");
    assert_eq!(rec["state"], "rejected");
    assert!(wt.exists(), "the worktree was not preserved");
}

/// TEST-020 — REQ-004.a, REQ-004.c. Positive.
#[test]
fn test_020_accepts_a_verified_attempt() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");

    let r = fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(0).to_str().unwrap(),
        "--evidence",
        "theory:spec-001/q1",
    ]);
    r.ok();

    let rec = r.record();
    assert_eq!(rec["decision"], "accepted");
    assert_eq!(rec["state"], "accepted");
    assert_eq!(rec["verifier"]["exit_code"], 0);
    assert_eq!(rec["verifier"]["command"][0], "cargo");
    assert!(rec["timestamps"]["decided_at"].is_string());
}

/// TEST-021 — REQ-004.c. Negative-input.
#[test]
fn test_021_rejects_acceptance_without_an_evidence_reference() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");

    let r = fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(0).to_str().unwrap(),
    ]);

    assert_eq!(r.code, 64, "stderr: {}", r.stderr);
    assert!(fx.record(&id)["decision"].is_null());
}

/// TEST-022 — REQ-004.d, REQ-004.e. Positive, prohibited-action.
#[test]
fn test_022_records_evidence_references_verbatim() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    let before = fx.record(&id)["external_programs"]
        .as_array()
        .unwrap()
        .len();

    let r = fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(0).to_str().unwrap(),
        "--evidence",
        "theory:spec-001/q1",
        "--evidence",
        "sha256:deadbeef",
    ]);
    r.ok();

    let rec = r.record();
    assert_eq!(
        rec["evidence_refs"],
        serde_json::json!(["theory:spec-001/q1", "sha256:deadbeef"]),
        "references must be recorded byte-for-byte and in order"
    );

    // REQ-004.e — nothing was run to resolve them. Root discovery calls Git;
    // that is the only program acceptance may invoke.
    let programs = rec["external_programs"].as_array().unwrap();
    for p in &programs[before..] {
        assert_eq!(p["name"], "git", "acceptance invoked {p}");
    }
}

/// TEST-023 — CON-003 grammar. Negative-input.
#[test]
fn test_023_rejects_a_malformed_verifier_record() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");

    let cases = [
        (
            "unknown property",
            r#"{"command":["x"],"exit_code":0,"output_path":"/t","extra":1}"#,
        ),
        (
            "missing exit_code",
            r#"{"command":["x"],"output_path":"/t"}"#,
        ),
        (
            "exit_code out of range",
            r#"{"command":["x"],"exit_code":300,"output_path":"/t"}"#,
        ),
        (
            "empty command",
            r#"{"command":[],"exit_code":0,"output_path":"/t"}"#,
        ),
        (
            "relative output_path",
            r#"{"command":["x"],"exit_code":0,"output_path":"t"}"#,
        ),
        ("not an object", r#"[]"#),
        ("not json", "verifier passed, honest"),
    ];

    for (name, body) in cases {
        let f = fx.raw_file(&format!("bad-{}.json", name.replace(' ', "-")), body);
        let r = fx.circus(&[
            "accept",
            "--attempt",
            &id,
            "--verifier-record",
            f.to_str().unwrap(),
            "--evidence",
            "theory:q1",
        ]);
        assert_eq!(r.code, 64, "{name} was accepted; stderr: {}", r.stderr);
        assert!(
            fx.record(&id)["decision"].is_null(),
            "{name} recorded a decision"
        );
    }
}

// ── REQ-005: serial, explicit merge ────────────────────────────────────────

/// TEST-008 — REQ-005.a. Positive.
#[test]
fn test_008_merges_an_accepted_attempt() {
    let fx = Fx::new();
    let id = fx.prepare_and_accept("model", "work.txt");
    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();

    let r = fx.circus(&["merge", "--attempt", &id, "--into", INTEGRATION]);
    r.ok();

    let after = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    assert_ne!(before, after);
    let parents = fx.git(&["rev-list", "--parents", "-n", "1", &after]).stdout;
    assert_eq!(
        parents.split_whitespace().count(),
        3,
        "one merge commit with two parents"
    );
    assert_eq!(r.record()["merge"]["result"], "merged");
    assert_eq!(r.record()["state"], "merged");
}

/// TEST-009 — REQ-005.a, NFR-001.a. Prohibited-action.
#[test]
fn test_009_preserves_a_rejected_attempt() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");
    fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(1).to_str().unwrap(),
        "--evidence",
        "theory:q1",
    ]);
    let before = fx.git(&["rev-parse", INTEGRATION]).stdout;

    let r = fx.circus(&["merge", "--attempt", &id, "--into", INTEGRATION]);

    assert_eq!(r.code, 64, "stderr: {}", r.stderr);
    assert_eq!(fx.git(&["rev-parse", INTEGRATION]).stdout, before);
    assert!(fx.worktree_of(&id).exists(), "worktree removed");
    assert!(fx.log_of(&id).exists(), "log removed");
    assert!(fx.record_exists(&id), "record removed");
    assert!(
        fx.git(&["branch", "--list", "circus/model/1"])
            .stdout
            .contains("circus/model/1"),
        "branch removed"
    );
}

/// TEST-010 — REQ-005.d. Scope-invariant.
#[test]
fn test_010_rejects_a_conflicting_merge() {
    let fx = Fx::new();
    let id = fx.prepare_and_complete("model");

    // Both sides change the same file.
    let wt = fx.worktree_of(&id);
    fs::write(wt.join("shared.txt"), "from the worker\n").unwrap();
    fx.git_in(&wt, &["add", "-A"]).ok();
    fx.git_in(&wt, &["commit", "-q", "-m", "worker"]).ok();

    fx.git(&["checkout", "-q", INTEGRATION]).ok();
    fs::write(fx.repo.join("shared.txt"), "from the trunk\n").unwrap();
    fx.git(&["add", "-A"]).ok();
    fx.git(&["commit", "-q", "-m", "trunk"]).ok();

    fx.circus(&[
        "accept",
        "--attempt",
        &id,
        "--verifier-record",
        fx.verifier_record(0).to_str().unwrap(),
        "--evidence",
        "theory:q1",
    ])
    .ok();

    let before = fx.git(&["rev-parse", INTEGRATION]).stdout.trim().to_owned();
    let r = fx.circus(&["merge", "--attempt", &id, "--into", INTEGRATION]);

    assert_ne!(r.code, 0, "a conflict must not report success");
    assert_eq!(
        fx.git(&["rev-parse", INTEGRATION]).stdout.trim(),
        before,
        "the integration ref moved on a conflict"
    );
    assert_eq!(fx.record(&id)["merge"]["result"], "conflict");
    assert_eq!(
        fx.git_in(&wt, &["status", "--porcelain"]).stdout.trim(),
        "",
        "the attempt worktree was left conflicted"
    );
}

/// TEST-024 — REQ-005.b. Concurrency.
#[test]
fn test_024_serialises_concurrent_merges() {
    let fx = Fx::new();
    let a = fx.prepare_and_accept("alpha", "alpha.txt");
    let b = fx.prepare_and_accept("beta", "beta.txt");
    let before = fx
        .git(&["rev-list", "--count", INTEGRATION])
        .stdout
        .trim()
        .to_owned();

    let ma = fx.circus_spawn(&["merge", "--attempt", &a, "--into", INTEGRATION]);
    let mb = fx.circus_spawn(&["merge", "--attempt", &b, "--into", INTEGRATION]);
    let ra = ma.wait_with_output().unwrap();
    let rb = mb.wait_with_output().unwrap();

    assert!(
        ra.status.success() && rb.status.success(),
        "both merges should succeed\na: {}\nb: {}",
        String::from_utf8_lossy(&ra.stderr),
        String::from_utf8_lossy(&rb.stderr)
    );

    let merges = fx
        .git(&["rev-list", "--merges", "--count", INTEGRATION])
        .stdout
        .trim()
        .parse::<u32>()
        .unwrap();
    assert_eq!(merges, 2, "both merge commits must be on the branch");
    assert!(
        fx.git(&["rev-list", "--count", INTEGRATION])
            .stdout
            .trim()
            .parse::<u32>()
            .unwrap()
            > before.parse::<u32>().unwrap(),
        "history did not advance"
    );
    for t in ["alpha.txt", "beta.txt"] {
        assert!(
            fx.git(&["cat-file", "-e", &format!("{INTEGRATION}:{t}")])
                .code
                == 0,
            "{t} is missing from the merged history"
        );
    }
}

/// TEST-026 — REQ-005.c. Negative-input.
#[test]
fn test_026_rejects_a_merge_into_a_different_ref() {
    let fx = Fx::new();
    let id = fx.prepare_and_accept("model", "work.txt");
    let before = fx.git(&["rev-parse", "main"]).stdout;

    let r = fx.circus(&["merge", "--attempt", &id, "--into", "main"]);

    assert_eq!(r.code, 64, "stderr: {}", r.stderr);
    assert_eq!(fx.git(&["rev-parse", "main"]).stdout, before, "main moved");
    assert!(fx.record(&id)["merge"].is_null());
}

/// TEST-029 — REQ-005.e. Scope-invariant.
#[test]
fn test_029_merge_performs_no_rebase_or_push() {
    let fx = Fx::new();
    // A remote to push to, so a push would be observable.
    let remote = fx.repo.parent().unwrap().join("remote.git");
    fx.git(&["init", "-q", "--bare", remote.to_str().unwrap()])
        .ok();
    fx.git(&["remote", "add", "origin", remote.to_str().unwrap()])
        .ok();

    let id = fx.prepare_and_accept("model", "work.txt");
    let reflog_before = fx
        .git(&["reflog", "show", INTEGRATION])
        .stdout
        .lines()
        .count();
    let branch_tip = fx.git(&["rev-parse", "circus/model/1"]).stdout;

    fx.circus(&["merge", "--attempt", &id, "--into", INTEGRATION])
        .ok();

    let reflog_after = fx
        .git(&["reflog", "show", INTEGRATION])
        .stdout
        .lines()
        .count();
    assert_eq!(
        reflog_after,
        reflog_before + 1,
        "the ref moved more than once"
    );
    assert_eq!(
        fx.git(&["rev-parse", "circus/model/1"]).stdout,
        branch_tip,
        "the task branch was rewritten"
    );
    assert_eq!(
        fx.git_in(&remote, &["branch", "--list"]).stdout.trim(),
        "",
        "something was pushed to the remote"
    );
}

/// TEST-032 — REQ-005.f. Negative-input, scope-invariant.
#[test]
fn test_032_refuses_a_merge_into_a_dirty_checkout() {
    let fx = Fx::new();
    let id = fx.prepare_and_accept("model", "work.txt");
    fx.git(&["checkout", "-q", INTEGRATION]).ok();
    fs::write(fx.repo.join("scratch.txt"), "uncommitted\n").unwrap();
    let before = fx.git(&["rev-parse", INTEGRATION]).stdout;

    let r = fx.circus(&["merge", "--attempt", &id, "--into", INTEGRATION]);

    assert_eq!(r.code, 64, "stderr: {}", r.stderr);
    assert_eq!(
        fx.git(&["rev-parse", INTEGRATION]).stdout,
        before,
        "the ref moved"
    );
    assert_eq!(
        fs::read_to_string(fx.repo.join("scratch.txt")).unwrap(),
        "uncommitted\n",
        "the uncommitted change was touched"
    );
}

/// TEST-033 — REQ-005.g. Positive.
#[test]
fn test_033_refreshes_the_checkout_holding_the_integration_ref() {
    let fx = Fx::new();
    let id = fx.prepare_and_accept("model", "work.txt");
    fx.git(&["checkout", "-q", INTEGRATION]).ok();
    assert!(!fx.repo.join("work.txt").exists());

    fx.circus(&["merge", "--attempt", &id, "--into", INTEGRATION])
        .ok();

    assert!(
        fx.repo.join("work.txt").exists(),
        "the checkout does not describe the merge commit"
    );
    assert_eq!(
        fx.git(&["status", "--porcelain"]).stdout.trim(),
        "",
        "the checkout was left inconsistent"
    );
}

// ── REQ-006 / REQ-007: boundaries ──────────────────────────────────────────

/// TEST-011 — REQ-006.a, REQ-006.c, REQ-007.c, CON-005. Prohibited-action.
#[test]
fn test_011_never_invokes_elephant() {
    let fx = Fx::new();
    let home = fx.bin.join("home");
    let elephant_home = fx.bin.join("elephant-home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&elephant_home).unwrap();
    let env = [
        ("HOME", home.to_str().unwrap()),
        ("ELEPHANT_HOME", elephant_home.to_str().unwrap()),
    ];

    // Exercise every command end to end.
    let r = fx.circus_env(
        &["prepare", "--task", "model", "--integration", INTEGRATION],
        &env,
    );
    r.ok();
    let id = r.record()["attempt_id"].as_str().unwrap().to_owned();
    let drv = fx.script(
        "quick",
        &format!("echo 0 > {}; sleep 30\n", fx.sentinel_of(&id).display()),
    );
    fx.circus_env(
        &[
            "launch",
            "--attempt",
            &id,
            "--prompt",
            fx.prompt(&id).to_str().unwrap(),
            "--",
            drv.to_str().unwrap(),
        ],
        &env,
    )
    .ok();
    let wt = fx.worktree_of(&id);
    fs::write(wt.join("w.txt"), "w\n").unwrap();
    fx.git_in(&wt, &["add", "-A"]).ok();
    fx.git_in(&wt, &["commit", "-q", "-m", "w"]).ok();
    fx.circus_env(
        &[
            "accept",
            "--attempt",
            &id,
            "--verifier-record",
            fx.verifier_record(0).to_str().unwrap(),
            "--evidence",
            "theory:q1",
        ],
        &env,
    )
    .ok();
    fx.circus_env(&["merge", "--attempt", &id, "--into", INTEGRATION], &env)
        .ok();

    // 1. No invocation named elephant, anywhere in the lifecycle.
    let rec = fx.record(&id);
    let programs = rec["external_programs"].as_array().unwrap();
    assert!(!programs.is_empty(), "the invocation record is empty");
    for p in programs {
        assert_ne!(p["name"], "elephant", "Circus invoked elephant: {p}");
        assert!(
            !p["resolved_path"].as_str().unwrap().contains("elephant"),
            "an elephant binary was resolved: {p}"
        );
    }

    // 2. A temporary Elephant home stays empty.
    assert_eq!(
        count_entries(&elephant_home),
        0,
        "ELEPHANT_HOME was written to"
    );

    // 3. And so does the default store. This assertion is the load-bearing
    //    one: CON-005 says Circus ignores ELEPHANT_HOME, so a write would land
    //    in the default location and assertion 2 alone would still pass.
    assert_eq!(
        count_entries(&home.join("Library/Application Support/elephant")),
        0,
        "the default Elephant store was written to"
    );
    assert_eq!(
        count_entries(&home.join(".local/share/elephant")),
        0,
        "the default Elephant store was written to"
    );
}

/// TEST-012 — REQ-006.b, REQ-006.d. Prohibited-action.
#[test]
fn test_012_does_not_parse_invite_material() {
    let fx = Fx::new();
    let id = fx.prepare("model");
    const TOKEN: &str = "ELEPHANT-INVITE-8f14e45fceea167a5a36dedd4bea2543";

    let prompt = fx.raw_file(
        "invite.md",
        &format!(
            "Join with {TOKEN}\nWhen done: echo 0 > {}\n",
            fx.sentinel_of(&id).display()
        ),
    );
    let drv = fx.script(
        "echoer",
        &format!(
            "echo \"the worker prints {TOKEN}\"\necho 0 > {}\nsleep 30\n",
            fx.sentinel_of(&id).display()
        ),
    );

    fx.circus(&[
        "launch",
        "--attempt",
        &id,
        "--prompt",
        prompt.to_str().unwrap(),
        "--",
        drv.to_str().unwrap(),
    ])
    .ok();

    // REQ-006.b — no field of the record is derived from the token.
    let rec = fx.record(&id);
    let rendered = serde_json::to_string(&rec).unwrap();
    assert!(
        !rendered.contains(TOKEN),
        "the run record captured invite material: {rendered}"
    );
    for p in rec["external_programs"].as_array().unwrap() {
        assert_ne!(p["name"], "elephant");
    }

    // REQ-006.d — the log is an uninspected copy of what the worker printed.
    // The token being present here is the specified consequence, not a defect.
    let log = fs::read_to_string(fx.log_of(&id)).unwrap();
    assert!(
        log.contains(TOKEN),
        "the log should be a verbatim copy: {log:?}"
    );
}

/// TEST-013 — REQ-007.a, CON-006. Negative.
#[test]
fn test_013_fails_clearly_for_a_missing_tool() {
    let fx = Fx::new();
    let id = fx.prepare("model");
    let drv = fx.script("unused", "echo SHOULD-NOT-RUN\n");
    let ran = fx.bin.join("driver-ran");

    for missing in ["tmux", "withdone"] {
        let path = fx.path_without(&[missing]);
        let r = fx.circus_env(
            &[
                "launch",
                "--attempt",
                &id,
                "--prompt",
                fx.prompt(&id).to_str().unwrap(),
                "--",
                drv.to_str().unwrap(),
            ],
            &[("PATH", &path)],
        );
        assert_eq!(r.code, 127, "missing {missing}: stderr {}", r.stderr);
        assert!(
            r.stderr.contains(missing),
            "the error must name {missing}: {}",
            r.stderr
        );
        assert!(!ran.exists(), "an agent process started without {missing}");
    }

    // Git is needed before any attempt can even be located.
    let path = fx.path_without(&["git"]);
    let r = fx.circus_env(
        &["prepare", "--task", "other", "--integration", INTEGRATION],
        &[("PATH", &path)],
    );
    assert_eq!(r.code, 127, "missing git: stderr {}", r.stderr);
    assert!(r.stderr.contains("git"));
}

// ── NFR-003 / CON-007: the run record ──────────────────────────────────────

/// TEST-014 — NFR-003.a, NFR-003.b, CON-007. Positive.
#[test]
fn test_014_emits_one_complete_run_record() {
    let fx = Fx::new();

    let prepared = fx
        .circus(&["prepare", "--task", "model", "--integration", INTEGRATION])
        .record();
    for key in [
        "schema_version",
        "attempt_id",
        "repository",
        "task",
        "attempt",
        "state",
        "integration_ref",
        "worktree_path",
        "branch",
        "timestamps",
    ] {
        assert!(!prepared[key].is_null(), "`{key}` missing after prepare");
    }
    assert_eq!(prepared["schema_version"], 1);
    assert!(prepared["timestamps"]["prepared_at"].is_string());

    let id = prepared["attempt_id"].as_str().unwrap().to_owned();
    let drv = fx.script(
        "quick",
        &format!("echo 0 > {}; sleep 30\n", fx.sentinel_of(&id).display()),
    );
    let launched = fx
        .circus(&[
            "launch",
            "--attempt",
            &id,
            "--prompt",
            fx.prompt(&id).to_str().unwrap(),
            "--",
            drv.to_str().unwrap(),
        ])
        .record();
    for key in [
        "pane_name",
        "log_path",
        "sentinel_path",
        "transport_exit_code",
    ] {
        assert!(!launched[key].is_null(), "`{key}` missing after launch");
    }
    assert!(launched["timestamps"]["launched_at"].is_string());
    assert!(launched["timestamps"]["completed_at"].is_string());

    let wt = fx.worktree_of(&id);
    fs::write(wt.join("w.txt"), "w\n").unwrap();
    fx.git_in(&wt, &["add", "-A"]).ok();
    fx.git_in(&wt, &["commit", "-q", "-m", "w"]).ok();

    let accepted = fx
        .circus(&[
            "accept",
            "--attempt",
            &id,
            "--verifier-record",
            fx.verifier_record(0).to_str().unwrap(),
            "--evidence",
            "theory:spec-001/q1",
        ])
        .record();
    // OBS-002
    assert_eq!(accepted["verifier"]["exit_code"], 0);
    assert_eq!(accepted["evidence_refs"][0], "theory:spec-001/q1");
    assert_eq!(accepted["decision"], "accepted");

    let merged = fx
        .circus(&["merge", "--attempt", &id, "--into", INTEGRATION])
        .record();
    // OBS-001, OBS-003, OBS-004
    assert_eq!(merged["merge"]["result"], "merged");
    assert!(merged["merge"]["commit"].is_string());
    assert!(merged["timestamps"]["merged_at"].is_string());
    assert_eq!(merged["completion_method"], "sentinel");
    assert_eq!(merged["process_group_residue"], 0);
    assert!(!merged["external_programs"].as_array().unwrap().is_empty());
}

/// TEST-028 — NFR-003.b, CON-007. Property-based.
#[test]
fn test_028_roundtrips_the_run_record() {
    let fx = Fx::new();
    let id = fx.prepare_and_accept("model", "work.txt");
    fx.circus(&["merge", "--attempt", &id, "--into", INTEGRATION])
        .ok();

    let path = fx.attempt_dir(&id).join("record.json");
    let text = fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Circus reads what it wrote, without loss, at every state the attempt
    // passed through. The exhaustive generator over every enum value lives in
    // the unit tests for `core::record`; this is the on-disk half.
    let reparsed: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
    assert_eq!(parsed, reparsed);

    // And the binary itself still accepts the file it produced.
    let r = fx.circus(&["merge", "--attempt", &id, "--into", INTEGRATION]);
    assert_ne!(
        r.code, 65,
        "Circus could not re-read its own record: {}",
        r.stderr
    );
}

/// TEST-031 — NFR-001.b. Prohibited-action.
#[test]
fn test_031_never_deletes_attempt_state() {
    let fx = Fx::new();
    let prepared = fx.prepare("kept");
    let completed = fx.prepare_and_complete("done");
    let rejected = fx.prepare_and_complete("bad");
    fx.circus(&[
        "accept",
        "--attempt",
        &rejected,
        "--verifier-record",
        fx.verifier_record(1).to_str().unwrap(),
        "--evidence",
        "theory:q1",
    ]);

    let ids = [&prepared, &completed, &rejected];
    let before: Vec<_> = ids.iter().map(|id| fx.attempt_dir(id)).collect();
    let worktrees: Vec<_> = ids.iter().map(|id| fx.worktree_of(id)).collect();
    let entries_before = count_entries(&fx.state_root());

    // Exercise every command, including ones that must refuse.
    let _ = fx.circus(&["merge", "--attempt", &rejected, "--into", INTEGRATION]);
    let _ = fx.circus(&["accept", "--attempt", &prepared, "--evidence", "x"]);
    let _ = fx.circus(&["prepare", "--task", "kept", "--integration", INTEGRATION]);

    for (id, dir) in ids.iter().zip(&before) {
        assert!(dir.exists(), "attempt directory for {id} was removed");
        assert!(fx.record_exists(id), "record for {id} was removed");
    }
    for wt in &worktrees {
        assert!(wt.exists(), "worktree {} was removed", wt.display());
    }
    assert!(fx.log_of(&completed).exists(), "a transcript was removed");
    assert!(
        count_entries(&fx.state_root()) >= entries_before,
        "the state root lost entries"
    );
}
