//! Path derivation for one attempt. Pure: given the roots and the identity,
//! every path an attempt uses is a function of its arguments.
//!
//! Layout is fixed by `SPEC-001-circus-agent-harness#ADR-006`:
//!
//! ```text
//! <state-root>/<task>/<attempt>/record.json
//! <state-root>/<task>/<attempt>/transcript.log
//! <state-root>/<task>/<attempt>/sentinel
//! <state-root>/<task>/<attempt>/status
//! <state-root>/.lock
//! ```
//!
//! The state root is `$(git rev-parse --git-common-dir)/circus`. The common
//! directory is shared by every worktree of the repository and is never itself
//! a worktree, which is what puts the log
//! (`SPEC-001-circus-agent-harness#REQ-002.d`) and the sentinel
//! (`SPEC-001-circus-agent-harness#REQ-003.b`) outside every worktree without
//! a configuration key.
//!
//! The worktree root is the parent of the main worktree, so an attempt
//! worktree is a sibling of the checkout it came from. The specification fixes
//! only the path's *contents*
//! (`SPEC-001-circus-agent-harness#REQ-001.c`: repository name, task, attempt
//! number); this module fixes the location, and the sibling placement is what
//! keeps an attempt worktree from nesting inside another worktree, where its
//! files would surface as untracked in the enclosing diff.

use std::path::{Path, PathBuf};

use super::grammar::{AttemptId, Task};

/// Every path one attempt touches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptPaths {
    /// The isolated checkout the worker edits.
    pub worktree: PathBuf,
    /// The task branch the worktree is checked out on.
    pub branch: String,
    /// The tmux window name, carrying task and attempt per REQ-002.b.
    pub pane_name: String,
    /// Directory holding every durable artefact of this attempt.
    pub attempt_dir: PathBuf,
    /// The run record.
    pub record: PathBuf,
    /// The transcript, outside every worktree per REQ-002.d.
    pub log: PathBuf,
    /// The sentinel withdone waits on, outside every worktree per REQ-003.b.
    pub sentinel: PathBuf,
    /// Where the pane writes withdone's exit status.
    pub status: PathBuf,
}

/// The roots an attempt's paths hang from. Supplied by the shell, which
/// discovers them from Git; taken as an argument here so this module performs
/// no I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    /// `<git-common-dir>/circus`.
    pub state: PathBuf,
    /// Parent of the main worktree.
    pub worktrees: PathBuf,
    /// The repository name, used in the worktree path per REQ-001.c.
    pub repository: String,
}

impl Roots {
    /// The single advisory lock, per ADR-006. Attempt allocation and merge
    /// both serialise on it.
    pub fn lock(&self) -> PathBuf {
        self.state.join(".lock")
    }

    /// The directory holding every attempt of one task.
    pub fn task_dir(&self, task: &Task) -> PathBuf {
        self.state.join(task.as_str())
    }
}

/// Derive every path for one attempt.
pub fn attempt_paths(roots: &Roots, id: &AttemptId) -> AttemptPaths {
    let attempt_dir = roots.task_dir(&id.task).join(id.attempt.to_string());
    AttemptPaths {
        worktree: roots
            .worktrees
            .join(format!("{}-{}-{}", roots.repository, id.task, id.attempt)),
        branch: format!("circus/{}/{}", id.task, id.attempt),
        pane_name: format!("circus-{}-{}", id.task, id.attempt),
        record: attempt_dir.join("record.json"),
        log: attempt_dir.join("transcript.log"),
        sentinel: attempt_dir.join("sentinel"),
        status: attempt_dir.join("status"),
        attempt_dir,
    }
}

/// Whether `inner` is `outer` or lies beneath it.
///
/// Used to verify the scope invariants of REQ-002.d and REQ-003.b at runtime
/// rather than only in a test. Both arguments are compared as given: the
/// caller passes absolute paths, and no filesystem is consulted.
pub fn is_within(inner: &Path, outer: &Path) -> bool {
    inner.starts_with(outer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::grammar::AttemptN;

    fn roots() -> Roots {
        Roots {
            state: PathBuf::from("/repo/.git/circus"),
            worktrees: PathBuf::from("/work"),
            repository: "circus".into(),
        }
    }

    fn id(task: &str, n: u16) -> AttemptId {
        AttemptId::new(
            Task::recognise(task).unwrap(),
            AttemptN::from_number(n).unwrap(),
        )
    }

    #[test]
    fn worktree_path_carries_repository_task_and_attempt() {
        // REQ-001.c
        let p = attempt_paths(&roots(), &id("model", 1));
        let s = p.worktree.to_string_lossy().to_string();
        assert!(s.contains("circus"), "{s}");
        assert!(s.contains("model"), "{s}");
        assert!(s.contains('1'), "{s}");
    }

    #[test]
    fn pane_name_carries_task_and_attempt() {
        // REQ-002.b
        let p = attempt_paths(&roots(), &id("model", 12));
        assert!(p.pane_name.contains("model"));
        assert!(p.pane_name.contains("12"));
    }

    #[test]
    fn log_and_sentinel_lie_outside_the_worktree() {
        // REQ-002.d and REQ-003.b, checked as a property of the derivation
        // rather than only of one live run.
        for task in ["model", "a", "z9-x"] {
            for n in [1u16, 7, 9999] {
                let p = attempt_paths(&roots(), &id(task, n));
                assert!(
                    !is_within(&p.log, &p.worktree),
                    "log inside worktree: {p:?}"
                );
                assert!(
                    !is_within(&p.sentinel, &p.worktree),
                    "sentinel inside worktree: {p:?}"
                );
                assert!(!is_within(&p.record, &p.worktree));
                assert!(!is_within(&p.status, &p.worktree));
            }
        }
    }

    #[test]
    fn distinct_attempts_never_share_a_path() {
        let r = roots();
        let a = attempt_paths(&r, &id("model", 1));
        let b = attempt_paths(&r, &id("model", 2));
        let c = attempt_paths(&r, &id("other", 1));
        for (x, y) in [(&a, &b), (&a, &c), (&b, &c)] {
            assert_ne!(x.worktree, y.worktree);
            assert_ne!(x.branch, y.branch);
            assert_ne!(x.pane_name, y.pane_name);
            assert_ne!(x.attempt_dir, y.attempt_dir);
        }
    }

    #[test]
    fn derivation_is_deterministic() {
        let r = roots();
        assert_eq!(
            attempt_paths(&r, &id("model", 3)),
            attempt_paths(&r, &id("model", 3))
        );
    }

    #[test]
    fn is_within_is_not_fooled_by_a_shared_prefix() {
        // "/work/circus-model-1" must not count as inside "/work/circus-model-12".
        let outer = Path::new("/work/circus-model-12");
        assert!(!is_within(Path::new("/work/circus-model-1"), outer));
        assert!(is_within(Path::new("/work/circus-model-12/src"), outer));
    }
}
