//! Shared fixture for the `SPEC-001-circus-agent-harness` test suite.
//!
//! Every test runs the real binary against a real Git repository, a real tmux
//! server, and the real withdone — Constitutional Principle 5,
//! Integration-First Testing. The tmux server is isolated per test through
//! `TMUX_TMPDIR`, so a run never touches the developer's own session.

#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

pub const BIN: &str = env!("CARGO_BIN_EXE_circus");
pub const INTEGRATION: &str = "circus/spec-001";

pub struct Fx {
    _dir: tempfile::TempDir,
    /// The main worktree. Named `circus`, so REQ-001.c is observable.
    pub repo: PathBuf,
    pub tmux_tmpdir: PathBuf,
    pub bin: PathBuf,
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn ok(&self) -> &Self {
        assert_eq!(self.code, 0, "expected success\nstderr: {}", self.stderr);
        self
    }
    pub fn record(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not a run record ({e}): {:?}", self.stdout))
    }
}

impl Fx {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = fs::canonicalize(dir.path()).expect("canonical tempdir");
        let repo = root.join("circus");
        let tmux_tmpdir = root.join("tmux");
        let bin = root.join("bin");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&tmux_tmpdir).unwrap();
        fs::create_dir_all(&bin).unwrap();

        let fx = Self {
            _dir: dir,
            repo,
            tmux_tmpdir,
            bin,
        };
        fx.git(&["init", "-q", "-b", "main", "."]).ok();
        fx.git(&["config", "user.email", "t@example.invalid"]).ok();
        fx.git(&["config", "user.name", "Test"]).ok();
        fs::write(fx.repo.join("README.md"), "base\n").unwrap();
        fx.git(&["add", "-A"]).ok();
        fx.git(&["commit", "-q", "-m", "base"]).ok();
        fx.git(&["branch", INTEGRATION]).ok();
        fx
    }

    // ── running things ────────────────────────────────────────────────────

    pub fn git(&self, args: &[&str]) -> Run {
        self.git_in(&self.repo, args)
    }

    pub fn git_in(&self, cwd: &Path, args: &[&str]) -> Run {
        into_run(
            Command::new("git")
                .args(args)
                .current_dir(cwd)
                .output()
                .expect("git runs"),
        )
    }

    /// Run the Circus binary with the fixture's isolated tmux server.
    pub fn circus(&self, args: &[&str]) -> Run {
        self.circus_env(args, &[])
    }

    pub fn circus_env(&self, args: &[&str], env: &[(&str, &str)]) -> Run {
        let mut cmd = Command::new(BIN);
        cmd.args(args)
            .current_dir(&self.repo)
            .env("TMUX_TMPDIR", &self.tmux_tmpdir);
        for (k, v) in env {
            cmd.env(k, v);
        }
        into_run(cmd.output().expect("circus runs"))
    }

    /// Spawn Circus without waiting, for the concurrency tests.
    pub fn circus_spawn(&self, args: &[&str]) -> std::process::Child {
        Command::new(BIN)
            .args(args)
            .current_dir(&self.repo)
            .env("TMUX_TMPDIR", &self.tmux_tmpdir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("circus spawns")
    }

    // ── locations ─────────────────────────────────────────────────────────

    pub fn state_root(&self) -> PathBuf {
        self.repo.join(".git").join("circus")
    }

    pub fn attempt_dir(&self, attempt: &str) -> PathBuf {
        let (task, n) = attempt.split_once('/').expect("task/attempt");
        self.state_root().join(task).join(n)
    }

    pub fn record(&self, attempt: &str) -> Value {
        let p = self.attempt_dir(attempt).join("record.json");
        let s = fs::read_to_string(&p).unwrap_or_else(|e| panic!("no record at {p:?}: {e}"));
        serde_json::from_str(&s).expect("record parses")
    }

    pub fn record_exists(&self, attempt: &str) -> bool {
        self.attempt_dir(attempt).join("record.json").exists()
    }

    pub fn worktree_of(&self, attempt: &str) -> PathBuf {
        PathBuf::from(
            self.record(attempt)["worktree_path"]
                .as_str()
                .expect("worktree_path"),
        )
    }

    pub fn sentinel_of(&self, attempt: &str) -> PathBuf {
        self.attempt_dir(attempt).join("sentinel")
    }

    pub fn log_of(&self, attempt: &str) -> PathBuf {
        self.attempt_dir(attempt).join("transcript.log")
    }

    // ── fixture artefacts ─────────────────────────────────────────────────

    /// Write an executable shell fixture and return its path.
    pub fn script(&self, name: &str, body: &str) -> PathBuf {
        let p = self.bin.join(name);
        fs::write(&p, format!("#!/bin/sh\n{body}")).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    /// A prompt naming the attempt's sentinel path, as REQ-003.c requires.
    pub fn prompt(&self, attempt: &str) -> PathBuf {
        self.prompt_named(attempt, "prompt.md")
    }

    pub fn prompt_named(&self, attempt: &str, name: &str) -> PathBuf {
        let p = self.bin.join(name);
        fs::write(
            &p,
            format!(
                "Do the work in this worktree.\nWhen done: echo 0 > {}\n",
                self.sentinel_of(attempt).display()
            ),
        )
        .unwrap();
        p
    }

    /// A verifier record conforming to CON-003.
    pub fn verifier_record(&self, exit_code: i32) -> PathBuf {
        let p = self.bin.join(format!("verifier-{exit_code}.json"));
        fs::write(
            &p,
            serde_json::json!({
                "command": ["cargo", "test"],
                "exit_code": exit_code,
                "output_path": "/tmp/verifier-output.txt",
            })
            .to_string(),
        )
        .unwrap();
        p
    }

    pub fn raw_file(&self, name: &str, body: &str) -> PathBuf {
        let p = self.bin.join(name);
        fs::write(&p, body).unwrap();
        p
    }

    // ── composite steps ───────────────────────────────────────────────────

    /// Prepare an attempt and return its `task/attempt` handle.
    pub fn prepare(&self, task: &str) -> String {
        let r = self.circus(&["prepare", "--task", task, "--integration", INTEGRATION]);
        r.ok();
        r.record()["attempt_id"].as_str().unwrap().to_owned()
    }

    /// Prepare, then launch a driver that writes the sentinel and stops.
    pub fn prepare_and_complete(&self, task: &str) -> String {
        let id = self.prepare(task);
        let drv = self.script(
            &format!("drv-{task}"),
            &format!(
                "echo out; echo err >&2; echo 0 > {}; sleep 30\n",
                self.sentinel_of(&id).display()
            ),
        );
        self.circus(&[
            "launch",
            "--attempt",
            &id,
            "--prompt",
            self.prompt(&id).to_str().unwrap(),
            "--",
            drv.to_str().unwrap(),
        ])
        .ok();
        id
    }

    /// Prepare, complete, add a commit on the task branch, and accept it.
    pub fn prepare_and_accept(&self, task: &str, file: &str) -> String {
        let id = self.prepare_and_complete(task);
        let wt = self.worktree_of(&id);
        fs::write(wt.join(file), "work\n").unwrap();
        self.git_in(&wt, &["add", "-A"]).ok();
        self.git_in(&wt, &["commit", "-q", "-m", "work"]).ok();
        self.circus(&[
            "accept",
            "--attempt",
            &id,
            "--verifier-record",
            self.verifier_record(0).to_str().unwrap(),
            "--evidence",
            "theory:spec-001/q1",
        ])
        .ok();
        id
    }

    /// A `PATH` containing only the named programs, for CON-006.
    pub fn path_without(&self, missing: &[&str]) -> String {
        let dir = self.bin.join(format!("path-{}", missing.join("-")));
        fs::create_dir_all(&dir).unwrap();
        for tool in [
            "git", "tmux", "withdone", "sh", "ps", "cat", "sleep", "mv", "printf",
        ] {
            if missing.contains(&tool) {
                continue;
            }
            if let Some(src) = which(tool) {
                let _ = std::os::unix::fs::symlink(src, dir.join(tool));
            }
        }
        dir.to_string_lossy().into_owned()
    }

    pub fn kill_tmux(&self) {
        let _ = Command::new("tmux")
            .arg("kill-server")
            .env("TMUX_TMPDIR", &self.tmux_tmpdir)
            .output();
    }

    pub fn tmux(&self, args: &[&str]) -> Run {
        into_run(
            Command::new("tmux")
                .args(args)
                .env("TMUX_TMPDIR", &self.tmux_tmpdir)
                .output()
                .expect("tmux runs"),
        )
    }
}

impl Drop for Fx {
    fn drop(&mut self) {
        self.kill_tmux();
    }
}

fn into_run(o: Output) -> Run {
    Run {
        code: o.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

pub fn which(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?
        .to_string_lossy()
        .split(':')
        .filter(|d| !d.is_empty())
        .map(|d| Path::new(d).join(program))
        .find(|c| c.is_file())
}

/// Count the entries under a directory tree, for exact-count assertions.
pub fn count_entries(root: &Path) -> usize {
    if !root.exists() {
        return 0;
    }
    let mut n = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            n += 1;
            if e.path().is_dir() {
                stack.push(e.path());
            }
        }
    }
    n
}
