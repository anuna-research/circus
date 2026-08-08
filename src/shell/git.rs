//! Git operations. Every one runs through [`Invoker`], so every one lands in
//! the invocation record of `SPEC-001-circus-agent-harness#OBS-004`.

use std::path::Path;

use super::proc::Invoker;
use super::{Error, Result};
use crate::core::grammar::ref_prefilter;
use crate::core::record::MergeOutcome;

/// Recognise a ref name.
///
/// Two stages, for the reason `crate::core::grammar::ref_prefilter` records:
/// the pure prefilter rejects option-shaped and control-bearing input before
/// it reaches a subprocess, and `git check-ref-format` — the authority on this
/// language — decides everything else. Circus never re-implements Git's rules.
pub fn recognise_ref(inv: &mut Invoker, r: &str) -> Result<String> {
    let r = ref_prefilter(r)?;
    let out = inv.run("git", &["check-ref-format", "--branch", r])?;
    if !out.status.success() {
        return Err(Error::usage(format!(
            "`{r}` is not a valid Git branch name: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(r.to_owned())
}

/// Whether a ref or commit resolves in this repository.
pub fn resolves(inv: &mut Invoker, cwd: &Path, r: &str) -> Result<bool> {
    let spec = format!("{r}^{{commit}}");
    let out = inv.run_in(
        "git",
        &["rev-parse", "--verify", "--quiet", &spec],
        Some(cwd),
    )?;
    Ok(out.status.success())
}

/// The commit a ref points at.
pub fn commit_of(inv: &mut Invoker, cwd: &Path, r: &str) -> Result<String> {
    let spec = format!("{r}^{{commit}}");
    inv.run_ok_in("git", &["rev-parse", "--verify", &spec], Some(cwd))
}

/// The full ref name, e.g. `refs/heads/main`.
pub fn full_ref(inv: &mut Invoker, cwd: &Path, r: &str) -> Result<String> {
    inv.run_ok_in("git", &["rev-parse", "--symbolic-full-name", r], Some(cwd))
}

/// Whether a branch exists.
pub fn branch_exists(inv: &mut Invoker, cwd: &Path, branch: &str) -> Result<bool> {
    let spec = format!("refs/heads/{branch}");
    let out = inv.run_in(
        "git",
        &["show-ref", "--verify", "--quiet", &spec],
        Some(cwd),
    )?;
    Ok(out.status.success())
}

/// Create one worktree on one new branch, starting from `start`.
///
/// A single `git worktree add -b` does both, so there is no window in which a
/// branch exists without its worktree — which is most of what `#CON-001`'s
/// "creates no partially registered attempt" asks for. The rest is
/// [`rollback`].
pub fn worktree_add(
    inv: &mut Invoker,
    cwd: &Path,
    worktree: &Path,
    branch: &str,
    start: &str,
) -> Result<()> {
    let out = inv.run_in(
        "git",
        &[
            "worktree",
            "add",
            "-b",
            branch,
            worktree.to_string_lossy().as_ref(),
            start,
            "--",
        ],
        Some(cwd),
    )?;
    if !out.status.success() {
        return Err(Error::software(format!(
            "git worktree add failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(())
}

/// Undo whatever part of an attempt did get created.
///
/// Called only on the failure path of `prepare`. It is not a general removal
/// command: `#NFR-001.b` forbids Circus deleting an attempt that exists, and
/// nothing here runs once `prepare` has returned successfully.
pub fn rollback(inv: &mut Invoker, cwd: &Path, worktree: &Path, branch: &str) {
    if worktree.exists() {
        let _ = inv.run_in(
            "git",
            &[
                "worktree",
                "remove",
                "--force",
                worktree.to_string_lossy().as_ref(),
            ],
            Some(cwd),
        );
    }
    let _ = inv.run_in("git", &["worktree", "prune"], Some(cwd));
    if matches!(branch_exists(inv, cwd, branch), Ok(true)) {
        let _ = inv.run_in("git", &["branch", "-D", branch, "--"], Some(cwd));
    }
    if worktree.exists() {
        let _ = std::fs::remove_dir_all(worktree);
    }
}

/// The outcome of an attempted merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeReport {
    pub outcome: MergeOutcome,
    /// The new merge commit, when one was created.
    pub commit: Option<String>,
    /// Conflicted paths, when the merge conflicted.
    pub conflicts: Vec<String>,
}

/// The worktree that has `full_ref` checked out, if any.
///
/// Git refuses to check one branch out in two worktrees, so there is at most
/// one.
pub fn checked_out_in(
    inv: &mut Invoker,
    cwd: &Path,
    full_ref: &str,
) -> Result<Option<std::path::PathBuf>> {
    let listing = inv.run_ok_in("git", &["worktree", "list", "--porcelain"], Some(cwd))?;
    let mut current: Option<&str> = None;
    for line in listing.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            current = Some(p);
        } else if let Some(b) = line.strip_prefix("branch ")
            && b == full_ref
        {
            return Ok(current.map(std::path::PathBuf::from));
        }
    }
    Ok(None)
}

/// Whether a worktree has no uncommitted change, tracked or untracked.
pub fn is_clean(inv: &mut Invoker, worktree: &Path) -> Result<bool> {
    let out = inv.run_ok_in("git", &["status", "--porcelain"], Some(worktree))?;
    Ok(out.trim().is_empty())
}

/// A merge that has been computed but not applied.
///
/// Producing one touches no ref and no file, which is what lets `--dry-run`
/// (`#REQ-009`) be the same computation as the merge with [`apply_merge`]
/// omitted, rather than a second implementation that can disagree with it.
#[derive(Debug, Clone)]
pub struct MergePlan {
    /// `Merged` here means "would merge cleanly".
    pub outcome: MergeOutcome,
    pub conflicts: Vec<String>,
    tree: Option<String>,
    before: String,
    head: String,
    full: String,
    occupied: Option<std::path::PathBuf>,
}

/// Compute the merge of `branch` into `integration`, applying nothing.
///
/// `git merge-tree --write-tree` computes the merge in the object database and
/// reports a conflict by exit status, so on the conflict path the integration
/// ref and every worktree are untouched by construction rather than by a
/// cleanup step — which is exactly what `#REQ-005.d` requires.
pub fn plan_merge(
    inv: &mut Invoker,
    cwd: &Path,
    integration: &str,
    branch: &str,
) -> Result<MergePlan> {
    let full = full_ref(inv, cwd, integration)?;
    let before = commit_of(inv, cwd, integration)?;
    let head = commit_of(inv, cwd, branch)?;

    // A ref that some worktree has checked out cannot simply be moved. That
    // worktree's index still describes the old commit, so everything the merge
    // added reads as a deletion, and one `git commit -a` there would undo the
    // merge. Refuse a dirty checkout up front — `#REQ-005.f` — so the refresh
    // below cannot fail after the ref has already moved.
    let occupied = checked_out_in(inv, cwd, &full)?;
    if let Some(wt) = &occupied
        && !is_clean(inv, wt)?
    {
        return Err(Error::usage(format!(
            "`{integration}` is checked out at {} and has uncommitted changes; \
             commit or stash them before merging",
            wt.display()
        )));
    }

    // `merge-tree` takes no `--` separator; both operands were recognised as
    // refs before reaching here, so neither can be read as an option.
    let out = inv.run_in(
        "git",
        &["merge-tree", "--write-tree", integration, branch],
        Some(cwd),
    )?;
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Exit 1 means "merged with conflicts". Anything else non-zero is
    // `merge-tree` failing, and reporting that as a conflict would turn a
    // Circus fault into a silent "your branches disagree".
    if out.status.code() != Some(0) && out.status.code() != Some(1) {
        return Err(Error::software(format!(
            "git merge-tree failed with status {}: {}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }

    if !out.status.success() {
        let conflicts = stdout
            .lines()
            .skip(1)
            .filter_map(|l| l.split_once('\t').map(|(_, p)| p.trim().to_owned()))
            .collect();
        return Ok(MergePlan {
            outcome: MergeOutcome::Conflict,
            conflicts,
            tree: None,
            before,
            head,
            full,
            occupied,
        });
    }

    let tree = stdout
        .lines()
        .next()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Error::software("git merge-tree produced no tree"))?
        .to_owned();

    Ok(MergePlan {
        outcome: MergeOutcome::Merged,
        conflicts: Vec::new(),
        tree: Some(tree),
        before,
        head,
        full,
        occupied,
    })
}

/// Apply a plan that merged cleanly.
///
/// The ref moves by `git update-ref <ref> <new> <old>`, a compare-and-swap.
/// The advisory lock of `#REQ-005.b` serialises Circus against Circus; the CAS
/// additionally refuses to clobber a change made by anything else between the
/// read and the write.
///
/// Nothing here rebases, amends, cherry-picks, force-updates, or pushes —
/// `#REQ-005.e`.
pub fn apply_merge(
    inv: &mut Invoker,
    cwd: &Path,
    integration: &str,
    branch: &str,
    plan: &MergePlan,
) -> Result<MergeReport> {
    let MergePlan {
        tree,
        before,
        head,
        full,
        occupied,
        ..
    } = plan;
    let Some(tree) = tree else {
        return Ok(MergeReport {
            outcome: MergeOutcome::Conflict,
            commit: None,
            conflicts: plan.conflicts.clone(),
        });
    };

    let message = format!("Merge {branch} into {integration}\n\nMerged by circus.\n");
    let commit = inv.run_ok_in(
        "git",
        &[
            "commit-tree",
            tree,
            "-p",
            before,
            "-p",
            head,
            "-m",
            &message,
        ],
        Some(cwd),
    )?;

    let out = inv.run_in(
        "git",
        &["update-ref", "-m", "circus merge", full, &commit, before],
        Some(cwd),
    )?;
    if !out.status.success() {
        return Err(Error::software(format!(
            "`{integration}` moved while the merge was being prepared; nothing was changed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }

    // The ref has moved. Bring the checkout that holds it in line, so the
    // operator's `git status` describes the merge rather than a pile of
    // phantom deletions — `#REQ-005.g`.
    //
    // `read-tree -m -u <before> <after>` is the two-tree merge `git checkout`
    // is built on: it updates exactly the paths that differ between the two
    // commits, and it refuses rather than clobbers if a local change is in the
    // way. `reset --keep HEAD` cannot do this job, because HEAD already points
    // at the new commit and the reset would compute an empty difference.
    if let Some(wt) = occupied {
        let out = inv.run_in("git", &["read-tree", "-m", "-u", before, &commit], Some(wt))?;
        if !out.status.success() {
            return Err(Error::software(format!(
                "merged {commit}, but the checkout at {} could not be refreshed: {}",
                wt.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
    }

    Ok(MergeReport {
        outcome: MergeOutcome::Merged,
        commit: Some(commit),
        conflicts: Vec::new(),
    })
}

/// Plan and apply in one step, for the ordinary merge path.
pub fn merge_into(
    inv: &mut Invoker,
    cwd: &Path,
    integration: &str,
    branch: &str,
) -> Result<MergeReport> {
    let plan = plan_merge(inv, cwd, integration, branch)?;
    apply_merge(inv, cwd, integration, branch, &plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct Repo {
        _dir: tempfile::TempDir,
        path: PathBuf,
        inv: Invoker,
    }

    impl Repo {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("repo");
            fs::create_dir_all(&path).unwrap();
            let mut inv = Invoker::new();
            for args in [
                vec!["init", "-q", "-b", "main", "."],
                vec!["config", "user.email", "t@e"],
                vec!["config", "user.name", "t"],
                vec!["commit", "-q", "--allow-empty", "-m", "base"],
            ] {
                inv.run_ok_in("git", &args, Some(&path)).unwrap();
            }
            Self {
                _dir: dir,
                path,
                inv,
            }
        }

        fn write_commit(&mut self, branch: &str, file: &str, body: &str) {
            let exists = branch_exists(&mut self.inv, &self.path, branch).unwrap();
            let args = if exists {
                vec!["checkout", "-q", branch]
            } else {
                vec!["checkout", "-q", "-b", branch]
            };
            self.inv.run_ok_in("git", &args, Some(&self.path)).unwrap();
            fs::write(self.path.join(file), body).unwrap();
            self.inv
                .run_ok_in("git", &["add", "-A"], Some(&self.path))
                .unwrap();
            self.inv
                .run_ok_in("git", &["commit", "-q", "-m", "c"], Some(&self.path))
                .unwrap();
        }
    }

    #[test]
    fn recognises_a_valid_branch_name_and_refuses_an_invalid_one() {
        let mut r = Repo::new();
        assert!(recognise_ref(&mut r.inv, "main").is_ok());
        assert!(recognise_ref(&mut r.inv, "circus/spec-001").is_ok());
        for bad in ["-f", "a..b", "a b", "a~1", ".hidden", "x.lock"] {
            assert!(recognise_ref(&mut r.inv, bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn a_dash_leading_ref_never_reaches_git() {
        // The prefilter's whole job: refuse before a subprocess sees it.
        let mut inv = Invoker::new();
        assert!(recognise_ref(&mut inv, "--upload-pack=evil").is_err());
        assert!(
            inv.invocations().is_empty(),
            "the prefilter must short-circuit"
        );
    }

    #[test]
    fn resolves_distinguishes_a_real_ref_from_a_well_formed_absent_one() {
        // CON-001's pre-condition. A name can pass `check-ref-format` and
        // still point at nothing, and that case has to exit 64 rather than
        // produce a worktree from a commit that does not exist.
        let mut r = Repo::new();
        assert!(resolves(&mut r.inv, &r.path, "main").unwrap());
        assert!(resolves(&mut r.inv, &r.path, "HEAD").unwrap());
        assert!(!resolves(&mut r.inv, &r.path, "no-such-branch").unwrap());
        assert!(!resolves(&mut r.inv, &r.path, "circus/never/created").unwrap());
    }

    #[test]
    fn commit_of_and_full_ref_agree_with_git() {
        let mut r = Repo::new();
        let head = r
            .inv
            .run_ok_in("git", &["rev-parse", "HEAD"], Some(&r.path))
            .unwrap();
        assert_eq!(commit_of(&mut r.inv, &r.path, "main").unwrap(), head);
        assert_eq!(
            full_ref(&mut r.inv, &r.path, "main").unwrap(),
            "refs/heads/main"
        );
    }

    #[test]
    fn adds_one_worktree_on_one_new_branch() {
        let mut r = Repo::new();
        let wt = r.path.parent().unwrap().join("wt");
        worktree_add(&mut r.inv, &r.path, &wt, "circus/model/1", "main").unwrap();
        assert!(wt.join(".git").exists());
        assert!(branch_exists(&mut r.inv, &r.path, "circus/model/1").unwrap());
        let list = r
            .inv
            .run_ok_in("git", &["worktree", "list"], Some(&r.path))
            .unwrap();
        assert_eq!(
            list.lines().count(),
            2,
            "one main worktree plus one attempt"
        );
    }

    #[test]
    fn rollback_removes_both_the_worktree_and_the_branch() {
        let mut r = Repo::new();
        let wt = r.path.parent().unwrap().join("wt");
        worktree_add(&mut r.inv, &r.path, &wt, "circus/model/1", "main").unwrap();
        rollback(&mut r.inv, &r.path, &wt, "circus/model/1");
        assert!(!wt.exists());
        assert!(!branch_exists(&mut r.inv, &r.path, "circus/model/1").unwrap());
    }

    #[test]
    fn merges_cleanly_and_creates_exactly_one_merge_commit() {
        let mut r = Repo::new();
        r.write_commit("feat", "a.txt", "a");
        r.inv
            .run_ok_in("git", &["checkout", "-q", "main"], Some(&r.path))
            .unwrap();
        let before = commit_of(&mut r.inv, &r.path, "main").unwrap();

        let rep = merge_into(&mut r.inv, &r.path, "main", "feat").unwrap();
        assert_eq!(rep.outcome, MergeOutcome::Merged);

        let after = commit_of(&mut r.inv, &r.path, "main").unwrap();
        assert_ne!(before, after);
        assert_eq!(rep.commit.as_deref(), Some(after.as_str()));
        let parents = r
            .inv
            .run_ok_in(
                "git",
                &["rev-list", "--parents", "-n", "1", &after],
                Some(&r.path),
            )
            .unwrap();
        assert_eq!(
            parents.split_whitespace().count(),
            3,
            "a merge commit has exactly two parents"
        );
    }

    #[test]
    fn a_conflict_leaves_the_integration_ref_untouched() {
        let mut r = Repo::new();
        r.write_commit("feat", "f.txt", "one");
        r.inv
            .run_ok_in("git", &["checkout", "-q", "main"], Some(&r.path))
            .unwrap();
        r.write_commit("main", "f.txt", "two");
        let before = commit_of(&mut r.inv, &r.path, "main").unwrap();

        let rep = merge_into(&mut r.inv, &r.path, "main", "feat").unwrap();
        assert_eq!(rep.outcome, MergeOutcome::Conflict);
        assert_eq!(rep.commit, None);
        assert!(
            rep.conflicts.iter().any(|p| p == "f.txt"),
            "{:?}",
            rep.conflicts
        );
        assert_eq!(commit_of(&mut r.inv, &r.path, "main").unwrap(), before);
    }

    #[test]
    fn merging_refreshes_the_checkout_that_holds_the_integration_ref() {
        // REQ-005.g. Without this, `git status` in the operator's checkout
        // reports everything the merge added as deleted, and `git commit -a`
        // would undo the merge.
        let mut r = Repo::new();
        r.write_commit("feat", "a.txt", "a");
        r.inv
            .run_ok_in("git", &["checkout", "-q", "main"], Some(&r.path))
            .unwrap();
        assert!(!r.path.join("a.txt").exists());

        merge_into(&mut r.inv, &r.path, "main", "feat").unwrap();

        assert!(
            r.path.join("a.txt").exists(),
            "the checkout should describe the merge commit"
        );
        let status = r
            .inv
            .run_ok_in("git", &["status", "--porcelain"], Some(&r.path))
            .unwrap();
        assert_eq!(
            status.trim(),
            "",
            "the checkout should be clean: {status:?}"
        );
    }

    #[test]
    fn merging_refuses_a_dirty_integration_checkout() {
        // REQ-005.f — refuse before the ref moves, not after.
        let mut r = Repo::new();
        r.write_commit("feat", "a.txt", "a");
        r.inv
            .run_ok_in("git", &["checkout", "-q", "main"], Some(&r.path))
            .unwrap();
        fs::write(r.path.join("dirty.txt"), "uncommitted").unwrap();
        let before = commit_of(&mut r.inv, &r.path, "main").unwrap();

        let e = merge_into(&mut r.inv, &r.path, "main", "feat").unwrap_err();
        assert_eq!(e.code(), crate::exit::USAGE);
        assert_eq!(
            commit_of(&mut r.inv, &r.path, "main").unwrap(),
            before,
            "a refused merge must leave the ref where it was"
        );
        assert!(
            r.path.join("dirty.txt").exists(),
            "and must not touch the tree"
        );
    }

    #[test]
    fn merging_touches_no_worktree_when_the_ref_is_checked_out_nowhere() {
        let mut r = Repo::new();
        r.write_commit("feat", "a.txt", "a");
        r.write_commit("trunk", "b.txt", "b");
        // Leave HEAD on `feat`, so `trunk` is checked out nowhere.
        r.inv
            .run_ok_in("git", &["checkout", "-q", "feat"], Some(&r.path))
            .unwrap();
        fs::write(r.path.join("dirty.txt"), "uncommitted").unwrap();
        let status_before = r
            .inv
            .run_ok_in("git", &["status", "--porcelain"], Some(&r.path))
            .unwrap();

        let rep = merge_into(&mut r.inv, &r.path, "trunk", "feat").unwrap();
        assert_eq!(rep.outcome, MergeOutcome::Merged);

        let status_after = r
            .inv
            .run_ok_in("git", &["status", "--porcelain"], Some(&r.path))
            .unwrap();
        assert_eq!(status_before, status_after, "no worktree should have moved");
    }

    #[test]
    fn reports_which_worktree_holds_a_ref() {
        let mut r = Repo::new();
        let full = full_ref(&mut r.inv, &r.path, "main").unwrap();
        let held = checked_out_in(&mut r.inv, &r.path, &full).unwrap();
        assert!(held.is_some(), "main is checked out in the main worktree");
        let absent = checked_out_in(&mut r.inv, &r.path, "refs/heads/nowhere").unwrap();
        assert_eq!(absent, None);
    }
}
