//! The state store: root discovery, the advisory lock, and atomic record I/O.
//!
//! Layout and placement are fixed by `SPEC-001-circus-agent-harness#ADR-006`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::proc::Invoker;
use super::{Error, Result};
use crate::core::paths::Roots;
use crate::core::record::{self, RunRecord};

/// Discover the state root, the worktree root, and the repository name.
///
/// `git rev-parse --git-common-dir` is the load-bearing call. From inside a
/// linked worktree it returns the *main* repository's `.git`, so every
/// worktree of one repository agrees on one state root without Circus needing
/// a configuration key.
pub fn discover_roots(inv: &mut Invoker, cwd: &Path) -> Result<Roots> {
    let common = inv.run_ok_in("git", &["rev-parse", "--git-common-dir"], Some(cwd))?;
    let common = absolutise(Path::new(&common), cwd);

    // The main worktree is the parent of the common directory for any
    // non-bare repository, which is what makes an attempt worktree a sibling
    // of the checkout it came from rather than a child of it.
    let main_worktree = common
        .parent()
        .ok_or_else(|| Error::software("the Git common directory has no parent"))?
        .to_path_buf();

    let repository = main_worktree
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| Error::software("cannot derive a repository name from the checkout path"))?;

    let worktrees = main_worktree
        .parent()
        .ok_or_else(|| Error::software("the checkout has no parent directory"))?
        .to_path_buf();

    Ok(Roots {
        state: common.join("circus"),
        worktrees,
        repository,
    })
}

fn absolutise(p: &Path, cwd: &Path) -> PathBuf {
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    };
    // `canonicalize` also resolves symlinks, which keeps two invocations from
    // deriving two state roots for one repository reached by different paths.
    fs::canonicalize(&joined).unwrap_or(joined)
}

/// A held advisory lock. Released when dropped, including on panic and on a
/// process exit the OS handles.
#[derive(Debug)]
pub struct LockGuard {
    file: File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // A failed unlock is not actionable: the OS releases the lock when the
        // descriptor closes a moment later.
        //
        // That also makes this body an equivalent mutant — deleting it changes
        // no observable behaviour, because the close that follows releases the
        // lock anyway. It stays because releasing explicitly at the end of the
        // critical section states the intent, and because a future change that
        // keeps the descriptor alive past the guard would otherwise silently
        // hold the lock.
        let _ = self.file.unlock();
    }
}

/// Take the advisory lock, blocking until it is available.
///
/// One lock serves both attempt allocation (`#CON-001`) and merge
/// (`#REQ-005.b`). Blocking rather than failing is what `#CON-004` specifies:
/// "A lock held by another process blocks until it is released."
///
/// Note what is *not* locked: `launch`. A held lock across an agent run would
/// serialise every worker in the repository, and would deadlock a worker that
/// itself calls Circus.
pub fn lock(roots: &Roots) -> Result<LockGuard> {
    fs::create_dir_all(&roots.state)?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(roots.lock())?;
    file.lock()?;
    Ok(LockGuard { file })
}

/// Read and recognise a run record.
///
/// A record that fails the schema is [`Error::DataErr`] (exit 65) and no field
/// of it reaches the caller — `#CON-007`.
pub fn read_record(path: &Path) -> Result<RunRecord> {
    let bytes = fs::read(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::usage(format!(
                "no attempt record at {}; prepare the attempt first",
                path.display()
            ))
        } else {
            Error::Io(e)
        }
    })?;
    record::recognise(&bytes).map_err(|e| {
        Error::DataErr(format!(
            "the record at {} is not readable: {e}",
            path.display()
        ))
    })
}

/// Write a run record atomically.
///
/// Write to a sibling temporary file, then rename. A concurrent reader
/// observes either the previous record or the new one, never a partial
/// document — the post-condition `#CON-007` states.
pub fn write_record(path: &Path, rec: &RunRecord) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = File::create(&tmp)?;
        f.write_all(record::serialise(rec).as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// The most Circus copies of a verifier log — `#REQ-014.b`.
///
/// A verifier log has no natural size, and the attempt directory is meant to
/// stay inspectable rather than become an archive.
pub const VERIFIER_LOG_LIMIT: u64 = 1024 * 1024;

/// Copy the verifier's output beside the attempt — `#REQ-014`.
///
/// Returns where it landed and whether it was truncated, or `None` when the
/// source cannot be read. An unreadable source is not an error: `#REQ-014.d`
/// keeps the capture out of the acceptance gate, which rests on the exit code
/// and the evidence references alone.
///
/// The source is opened read-only and never written, so `#REQ-014.c` holds by
/// construction.
pub fn capture_verifier_log(source: &Path, dest: &Path) -> Option<(PathBuf, bool)> {
    use std::io::Read;

    let file = File::open(source).ok()?;
    let mut buf = Vec::new();
    // Read one byte past the limit, so a source of exactly the limit is not
    // reported as truncated. `File` implements both Read and Write, so the
    // adaptor is named through the trait rather than by method call.
    Read::take(file, VERIFIER_LOG_LIMIT + 1)
        .read_to_end(&mut buf)
        .ok()?;

    let truncated = buf.len() as u64 > VERIFIER_LOG_LIMIT;
    if truncated {
        buf.truncate(VERIFIER_LOG_LIMIT as usize);
    }

    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).ok()?;
    }
    fs::write(dest, &buf).ok()?;
    Some((dest.to_path_buf(), truncated))
}

/// Whether an attempt directory already exists.
pub fn attempt_exists(attempt_dir: &Path) -> bool {
    attempt_dir.exists()
}

/// The lowest unused attempt number for a task.
///
/// Called under the lock, so the scan and the subsequent creation are one
/// critical section and two concurrent preparations cannot choose the same
/// number — `#CON-001`, `#TEST-025`.
pub fn next_attempt(roots: &Roots, task: &crate::core::grammar::Task) -> Result<u16> {
    let dir = roots.task_dir(task);
    let mut n = 1u16;
    while dir.join(n.to_string()).exists() {
        n = n
            .checked_add(1)
            .filter(|n| *n <= 9999)
            .ok_or_else(|| Error::usage(format!("task `{task}` has used all 9999 attempts")))?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::grammar::Task;
    use crate::core::record::State;

    fn roots_in(dir: &Path) -> Roots {
        Roots {
            state: dir.join("state"),
            worktrees: dir.join("work"),
            repository: "circus".into(),
        }
    }

    fn sample() -> RunRecord {
        let id = crate::core::grammar::AttemptId::recognise("model/1").unwrap();
        let roots = Roots {
            state: "/repo/.git/circus".into(),
            worktrees: "/work".into(),
            repository: "circus".into(),
        };
        let paths = crate::core::paths::attempt_paths(&roots, &id);
        RunRecord::prepared(
            &id,
            &paths,
            "circus".into(),
            "main".into(),
            "2026-08-08T00:00:00Z".into(),
        )
    }

    #[test]
    fn a_record_survives_a_write_and_read() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a/record.json");
        let r = sample();
        write_record(&p, &r).unwrap();
        assert_eq!(read_record(&p).unwrap(), r);
    }

    #[test]
    fn writing_leaves_no_temporary_file_behind() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("record.json");
        write_record(&p, &sample()).unwrap();
        let names: Vec<_> = fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["record.json"]);
    }

    #[test]
    fn a_rewrite_replaces_rather_than_appends() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("record.json");
        write_record(&p, &sample()).unwrap();
        let mut r = sample();
        r.state = State::Completed;
        write_record(&p, &r).unwrap();
        assert_eq!(read_record(&p).unwrap().state, State::Completed);
    }

    #[test]
    fn a_corrupt_record_is_a_data_error_not_a_usage_error() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("record.json");
        fs::write(&p, b"{\"schema_version\": 1").unwrap();
        let e = read_record(&p).unwrap_err();
        assert_eq!(e.code(), crate::exit::DATAERR);
    }

    #[test]
    fn a_missing_record_is_a_usage_error() {
        let d = tempfile::tempdir().unwrap();
        let e = read_record(&d.path().join("nope.json")).unwrap_err();
        assert_eq!(e.code(), crate::exit::USAGE);
    }

    #[test]
    fn a_verifier_log_is_copied_verbatim() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("v.txt");
        fs::write(&src, b"pytest\n1 failed\n").unwrap();
        let dest = d.path().join("a/verifier.log");

        let (path, truncated) = capture_verifier_log(&src, &dest).unwrap();
        assert_eq!(path, dest);
        assert!(!truncated);
        assert_eq!(fs::read(&dest).unwrap(), b"pytest\n1 failed\n");
        // REQ-014.c — the caller's file is untouched.
        assert_eq!(fs::read(&src).unwrap(), b"pytest\n1 failed\n");
    }

    #[test]
    fn a_log_at_the_limit_is_not_reported_as_truncated() {
        // The off-by-one that would mislabel an exactly-sized log.
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("v.txt");
        fs::write(&src, vec![b'x'; VERIFIER_LOG_LIMIT as usize]).unwrap();
        let (_, truncated) = capture_verifier_log(&src, &d.path().join("out.log")).unwrap();
        assert!(!truncated, "exactly the limit is whole, not truncated");
    }

    #[test]
    fn an_oversized_log_is_bounded_and_says_so() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("v.txt");
        fs::write(&src, vec![b'x'; VERIFIER_LOG_LIMIT as usize + 5000]).unwrap();
        let dest = d.path().join("out.log");
        let (_, truncated) = capture_verifier_log(&src, &dest).unwrap();
        assert!(truncated);
        assert_eq!(fs::metadata(&dest).unwrap().len(), VERIFIER_LOG_LIMIT);
    }

    #[test]
    fn an_unreadable_source_is_none_rather_than_an_error() {
        // REQ-014.d — a missing log must not be able to veto an acceptance.
        let d = tempfile::tempdir().unwrap();
        assert!(capture_verifier_log(&d.path().join("nope"), &d.path().join("o")).is_none());
        assert!(
            capture_verifier_log(d.path(), &d.path().join("o2")).is_none(),
            "a directory"
        );
    }

    #[test]
    fn an_empty_log_is_still_captured() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("empty.txt");
        fs::write(&src, b"").unwrap();
        let dest = d.path().join("out.log");
        let (_, truncated) = capture_verifier_log(&src, &dest).unwrap();
        assert!(!truncated);
        assert!(
            dest.exists(),
            "an empty verifier log is a fact, not an absence"
        );
    }

    #[test]
    fn attempt_exists_reports_the_directory_rather_than_a_constant() {
        // CON-001 refuses an attempt that already exists, so this predicate
        // is the difference between a fresh ring and a clobbered one.
        let d = tempfile::tempdir().unwrap();
        let r = roots_in(d.path());
        let task = Task::recognise("model").unwrap();
        let dir = r.task_dir(&task).join("1");
        assert!(
            !attempt_exists(&dir),
            "an unprepared attempt must not exist"
        );
        fs::create_dir_all(&dir).unwrap();
        assert!(attempt_exists(&dir), "a prepared attempt must exist");
    }

    #[test]
    fn attempt_numbers_start_at_one_and_fill_gaps_upward() {
        let d = tempfile::tempdir().unwrap();
        let r = roots_in(d.path());
        let task = Task::recognise("model").unwrap();
        assert_eq!(next_attempt(&r, &task).unwrap(), 1);
        fs::create_dir_all(r.task_dir(&task).join("1")).unwrap();
        assert_eq!(next_attempt(&r, &task).unwrap(), 2);
        fs::create_dir_all(r.task_dir(&task).join("2")).unwrap();
        assert_eq!(next_attempt(&r, &task).unwrap(), 3);
    }

    #[test]
    fn the_lock_is_exclusive_across_processes() {
        // Held here, so a child process must fail to take it without blocking.
        let d = tempfile::tempdir().unwrap();
        let r = roots_in(d.path());
        let _held = lock(&r).unwrap();

        let probe = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                // shlock(1) is not portable; use a tiny flock via python3 if
                // present, else skip the assertion rather than fake it.
                "command -v python3 >/dev/null || exit 77; \
                 python3 -c \"import fcntl,sys; f=open(sys.argv[1],'a'); \
                 fcntl.flock(f, fcntl.LOCK_EX|fcntl.LOCK_NB)\" {}",
                r.lock().display()
            ))
            .output()
            .unwrap();
        if probe.status.code() == Some(77) {
            return; // no python3; the lock is still exercised by TEST-024/025
        }
        assert!(
            !probe.status.success(),
            "a second process took a lock this process holds"
        );
    }

    #[test]
    fn the_lock_is_released_when_the_guard_drops() {
        let d = tempfile::tempdir().unwrap();
        let r = roots_in(d.path());
        drop(lock(&r).unwrap());
        // Re-taking it in this process would succeed even if it were still
        // held, so take it from a child instead.
        let probe = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "command -v python3 >/dev/null || exit 77; \
                 python3 -c \"import fcntl,sys; f=open(sys.argv[1],'a'); \
                 fcntl.flock(f, fcntl.LOCK_EX|fcntl.LOCK_NB)\" {}",
                r.lock().display()
            ))
            .output()
            .unwrap();
        if probe.status.code() == Some(77) {
            return;
        }
        assert!(probe.status.success(), "the lock outlived its guard");
    }

    #[test]
    fn roots_agree_between_the_main_worktree_and_a_linked_one() {
        // The property ADR-006 rests on. Built with real Git rather than
        // asserted, because the whole design depends on it.
        let d = tempfile::tempdir().unwrap();
        let main = d.path().join("circus");
        fs::create_dir_all(&main).unwrap();
        let git = |args: &[&str], cwd: &Path| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(cwd)
                .output()
                .unwrap()
        };
        git(&["init", "-q", "-b", "main", "."], &main);
        git(&["config", "user.email", "t@e"], &main);
        git(&["config", "user.name", "t"], &main);
        git(&["commit", "-q", "--allow-empty", "-m", "x"], &main);
        let linked = d.path().join("linked");
        git(
            &[
                "worktree",
                "add",
                "-q",
                linked.to_str().unwrap(),
                "-b",
                "wt",
            ],
            &main,
        );

        let mut inv = Invoker::new();
        let from_main = discover_roots(&mut inv, &main).unwrap();
        let from_linked = discover_roots(&mut inv, &linked).unwrap();
        assert_eq!(from_main.state, from_linked.state);
        assert_eq!(from_main.worktrees, from_linked.worktrees);
        assert_eq!(from_main.repository, from_linked.repository);
        assert_eq!(from_main.repository, "circus");
        assert!(
            from_main.state.ends_with("circus/.git/circus"),
            "state root should sit under the common dir: {:?}",
            from_main.state
        );
    }
}
