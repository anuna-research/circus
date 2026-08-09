//! Live observation of an attempt — `SPEC-001-circus-agent-harness#CON-009`.
//!
//! A run record says what Circus last observed. An attempt that is still
//! running is the case where that is not enough: the pane outlives the command
//! that started it, so an interrupted `circus launch` leaves a record reading
//! `prepared` beside an agent that is still working.
//!
//! Everything here is read at the moment of the query and nothing is written
//! back — `#REQ-011.c`.

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::proc::Invoker;
use super::{Result, git, state};
use crate::core::grammar::{AttemptId, AttemptN, Task};
use crate::core::paths::{self, Roots};
use crate::core::record::RunRecord;
use crate::core::time::UnixSeconds;

/// One attempt, as recorded and as observed.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    /// The run record, unchanged and still conforming to `#CON-007`.
    pub attempt: RunRecord,
    pub live: Live,
}

/// What was true of an attempt when it was queried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Live {
    pub pane: String,
    pub pane_alive: bool,
    /// Seconds since launch, while the attempt is still running.
    pub running_for_seconds: Option<i64>,
    pub sentinel_present: bool,
    pub worktree_present: bool,
    pub worktree_dirty: Option<bool>,
    /// Commits on the task branch over the recorded integration ref.
    pub commits_ahead: Option<u32>,
    pub transcript_bytes: u64,
}

/// Observe one attempt.
pub fn observe(
    inv: &mut Invoker,
    roots: &Roots,
    cwd: &Path,
    id: &AttemptId,
    rec: &RunRecord,
    now: UnixSeconds,
) -> Result<Status> {
    let p = paths::attempt_paths(roots, id);

    let pane_alive = inv
        .run("tmux", &["has-session", "-t", &p.pane_name])
        .map(|o| o.status.success())
        // No tmux server at all means no pane, which is an answer rather than
        // a failure.
        .unwrap_or(false);

    let worktree_present = p.worktree.is_dir();
    let worktree_dirty = if worktree_present {
        git::is_clean(inv, &p.worktree).ok().map(|clean| !clean)
    } else {
        None
    };

    let commits_ahead = if git::branch_exists(inv, cwd, &rec.branch).unwrap_or(false) {
        let range = format!("{}..{}", rec.integration_ref, rec.branch);
        inv.run_ok_in("git", &["rev-list", "--count", &range], Some(cwd))
            .ok()
            .and_then(|s| s.trim().parse().ok())
    } else {
        None
    };

    // Elapsed only means something while the attempt is between launch and
    // completion. Anywhere else the record's timestamps already say more.
    let running_for_seconds = match (&rec.timestamps.launched_at, &rec.timestamps.completed_at) {
        (Some(started), None) => crate::core::time::parse_rfc3339(started)
            .map(|t| (now.0 - t.0).max(0))
            .ok(),
        _ => None,
    };

    Ok(Status {
        attempt: rec.clone(),
        live: Live {
            pane: p.pane_name,
            pane_alive,
            running_for_seconds,
            sentinel_present: p.sentinel.exists(),
            worktree_present,
            worktree_dirty,
            commits_ahead,
            transcript_bytes: fs::metadata(&p.log).map(|m| m.len()).unwrap_or(0),
        },
    })
}

/// Every attempt in the repository, ordered by task then attempt number.
///
/// A directory under the state root that does not name a recognised task, or
/// that holds no readable record, is skipped rather than reported. The state
/// root is Circus's own, but it is still a directory anyone can drop a file
/// into, and a stray one is not a reason to refuse the whole query.
pub fn all_attempts(roots: &Roots) -> Vec<AttemptId> {
    let Ok(tasks) = fs::read_dir(&roots.state) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for task_entry in tasks.flatten() {
        let Ok(task) = Task::recognise(&task_entry.file_name().to_string_lossy()) else {
            continue;
        };
        let Ok(attempts) = fs::read_dir(task_entry.path()) else {
            continue;
        };
        for a in attempts.flatten() {
            let Ok(n) = AttemptN::recognise(&a.file_name().to_string_lossy()) else {
                continue;
            };
            if a.path().join("record.json").is_file() {
                out.push(AttemptId::new(task.clone(), n));
            }
        }
    }
    out.sort();
    out
}

/// Read the record for an attempt, for the listing path.
pub fn record_of(roots: &Roots, id: &AttemptId) -> Result<RunRecord> {
    state::read_record(&paths::attempt_paths(roots, id).record)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots_in(dir: &Path) -> Roots {
        Roots {
            state: dir.join("state"),
            worktrees: dir.join("work"),
            repository: "circus".into(),
        }
    }

    #[test]
    fn an_empty_state_root_lists_nothing() {
        let d = tempfile::tempdir().unwrap();
        assert!(all_attempts(&roots_in(d.path())).is_empty());
    }

    #[test]
    fn attempts_are_listed_by_task_then_number() {
        let d = tempfile::tempdir().unwrap();
        let r = roots_in(d.path());
        for (task, n) in [
            ("beta", "2"),
            ("alpha", "10"),
            ("alpha", "2"),
            ("beta", "1"),
        ] {
            let dir = r.state.join(task).join(n);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("record.json"), b"{}").unwrap();
        }
        let got: Vec<String> = all_attempts(&r).iter().map(|i| i.to_string()).collect();
        assert_eq!(got, ["alpha/2", "alpha/10", "beta/1", "beta/2"]);
    }

    #[test]
    fn a_directory_without_a_record_is_not_an_attempt() {
        let d = tempfile::tempdir().unwrap();
        let r = roots_in(d.path());
        fs::create_dir_all(r.state.join("model").join("1")).unwrap();
        assert!(all_attempts(&r).is_empty(), "no record.json, so no attempt");
    }

    #[test]
    fn stray_names_under_the_state_root_are_skipped() {
        let d = tempfile::tempdir().unwrap();
        let r = roots_in(d.path());
        // The lock file lives here, and so might anything else.
        fs::create_dir_all(&r.state).unwrap();
        fs::write(r.lock(), b"").unwrap();
        // Names the task grammar refuses, plus attempt numbers it refuses.
        // Nothing uppercase: a case-insensitive filesystem would fold that
        // into the valid task beside it, which tests the platform rather than
        // the code.
        for (task, n) in [
            ("not_a_task", "1"),
            ("9bad", "1"),
            ("model", "0"),
            ("model", "x"),
        ] {
            let dir = r.state.join(task).join(n);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("record.json"), b"{}").unwrap();
        }
        let good = r.state.join("model").join("3");
        fs::create_dir_all(&good).unwrap();
        fs::write(good.join("record.json"), b"{}").unwrap();

        let got: Vec<String> = all_attempts(&r).iter().map(|i| i.to_string()).collect();
        assert_eq!(got, ["model/3"], "only recognised identifiers are attempts");
    }
}
