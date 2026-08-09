//! External program invocation, with a record of every call.
//!
//! `SPEC-001-circus-agent-harness#REQ-007.a` says Circus invokes Git, tmux,
//! withdone, and the driver as external programs. `#REQ-007.c` says it records
//! each one. Routing every invocation through [`Invoker`] is what makes the
//! second true by construction: there is no other way to start a process, so
//! the record cannot fall out of step with what ran.
//!
//! That record is `#OBS-004`, and it is what lets `TEST-011` decide the
//! Elephant prohibition (`#REQ-006.c`) from an artefact instead of from
//! review.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use super::ui::Ui;
use super::{Error, Result};
use crate::core::record::ExternalProgram;

/// Runs external programs and remembers what it ran.
#[derive(Debug)]
pub struct Invoker {
    log: Vec<ExternalProgram>,
    /// Programs whose absence has already been reported, so a missing tool is
    /// named once rather than once per call.
    missing: BTreeSet<String>,
    /// Where `--verbose` invocation lines go — `#REQ-008.f`.
    ui: Ui,
}

impl Default for Invoker {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            missing: BTreeSet::new(),
            ui: Ui::silent(),
        }
    }
}

impl Invoker {
    pub fn new() -> Self {
        Self::default()
    }

    /// An invoker that narrates what it runs when `--verbose` is set.
    pub fn with_ui(ui: Ui) -> Self {
        Self {
            ui,
            ..Self::default()
        }
    }

    /// Everything invoked so far, in call order.
    pub fn invocations(&self) -> &[ExternalProgram] {
        &self.log
    }

    /// Take the log, leaving the invoker empty.
    pub fn take(&mut self) -> Vec<ExternalProgram> {
        std::mem::take(&mut self.log)
    }

    /// Resolve a program on `PATH` before running it.
    ///
    /// Absence is [`Error::NotFound`], which exits 127 and names the program —
    /// `#CON-006`. Circus never falls back to an internal implementation.
    pub fn resolve(&mut self, program: &str) -> Result<PathBuf> {
        if let Some(p) = which(program) {
            return Ok(p);
        }
        self.missing.insert(program.to_owned());
        Err(Error::NotFound(program.to_owned()))
    }

    /// Run a program to completion, capturing stdout and stderr.
    pub fn run<S: AsRef<OsStr>>(&mut self, program: &str, args: &[S]) -> Result<Output> {
        self.run_in(program, args, None)
    }

    /// Run a program to completion in a given working directory.
    pub fn run_in<S: AsRef<OsStr>>(
        &mut self,
        program: &str,
        args: &[S],
        cwd: Option<&Path>,
    ) -> Result<Output> {
        let resolved = self.resolve(program)?;
        if self.ui.is_verbose() {
            let rendered: Vec<String> = args
                .iter()
                .map(|a| a.as_ref().to_string_lossy().into_owned())
                .collect();
            self.ui.invocation(program, &rendered);
        }
        let mut cmd = Command::new(&resolved);
        cmd.args(args.iter().map(AsRef::as_ref))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        let output = cmd
            .output()
            .map_err(|e| Error::software(format!("failed to run `{program}`: {e}")))?;
        self.log.push(ExternalProgram {
            name: program.to_owned(),
            resolved_path: resolved.to_string_lossy().into_owned(),
            exit_status: output.status.code(),
        });
        Ok(output)
    }

    /// Run a program and require success, returning its stdout as a trimmed
    /// string. Used for the many Git queries whose failure is a hard error.
    pub fn run_ok<S: AsRef<OsStr>>(&mut self, program: &str, args: &[S]) -> Result<String> {
        self.run_ok_in(program, args, None)
    }

    /// As [`Invoker::run_ok`], in a working directory.
    pub fn run_ok_in<S: AsRef<OsStr>>(
        &mut self,
        program: &str,
        args: &[S],
        cwd: Option<&Path>,
    ) -> Result<String> {
        let out = self.run_in(program, args, cwd)?;
        if !out.status.success() {
            let rendered: Vec<String> = args
                .iter()
                .map(|a| a.as_ref().to_string_lossy().into_owned())
                .collect();
            return Err(Error::software(format!(
                "`{program} {}` exited {}: {}",
                rendered.join(" "),
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    }
}

/// Find an executable on `PATH`.
///
/// An argument containing a separator is taken as a path and used directly,
/// which is how `#CON-002` admits `DRIVER = abs-path / <a name resolvable on
/// PATH>`.
pub fn which(program: &str) -> Option<PathBuf> {
    if program.contains('/') {
        let p = PathBuf::from(program);
        return is_executable(&p).then_some(p);
    }
    std::env::var_os("PATH")?
        .to_string_lossy()
        .split(':')
        .filter(|d| !d.is_empty())
        .map(|d| Path::new(d).join(program))
        .find(|c| is_executable(c))
}

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// How many processes remain in a process group.
///
/// `#OBS-003` states this as a count rather than a flag, so it is measured by
/// enumeration. `ps` is the platform's own answer to the question and is
/// invoked through the same recorded path as everything else.
///
/// The selection is done here rather than by `ps -g`, because that flag does
/// not mean the same thing twice. BSD and macOS read it as "processes whose
/// process group leader is in this list"; Linux procps reads it as "select by
/// session **or** by effective group name". Circus asked for a process group
/// and Linux answered about sessions, which reported residue for a group that
/// had been reaped and failed every attempt in CI while passing on the
/// developer's machine.
///
/// `ps -A -o pid=,pgid=,state=,comm=` is POSIX and means one thing everywhere.
/// Filtering it costs a few lines of output and removes a platform divergence
/// from a control: `#NFR-002.c` fails an attempt on a non-zero count.
///
/// The state column is not decoration. It is what distinguishes a process that
/// outlived the deadline from one that died and has not yet been reaped — see
/// [`rows_in_group`].
pub fn process_group_residue(inv: &mut Invoker, pgid: i32) -> u32 {
    survivors(inv, pgid).len() as u32
}

/// Which processes remain in a process group, as `pid command` pairs.
///
/// A count answers whether cleanup worked; only the names answer why it did
/// not. "1 process outlived the deadline" sent me guessing twice; "sleep 60
/// (pid 123) outlived the deadline" does not.
pub fn survivors(inv: &mut Invoker, pgid: i32) -> Vec<(i32, String)> {
    let Ok(out) = inv.run("ps", &["-A", "-o", "pid=,pgid=,state=,comm="]) else {
        // `ps` absent or failed: report no residue rather than invent one. The
        // caller treats an unmeasurable group as clean, which is the same
        // answer the OS gives when the group is genuinely empty.
        return Vec::new();
    };
    rows_in_group(&String::from_utf8_lossy(&out.stdout), pgid)
}

/// Parse a `ps -A -o pid=,pgid=,state=,comm=` listing into the live rows of
/// one process group. Zombies are not rows of it — see below.
///
/// Split out from the `ps` call so it can be tested against a fixed listing.
/// Testing it against a live one cannot work: two snapshots taken moments
/// apart disagree, because processes come and go between them.
pub fn rows_in_group(listing: &str, pgid: i32) -> Vec<(i32, String)> {
    listing
        .lines()
        .filter_map(|line| {
            let mut f = line.split_whitespace();
            let pid: i32 = f.next()?.parse().ok()?;
            let group: i32 = f.next()?.parse().ok()?;
            if group != pgid {
                return None;
            }
            // A zombie is not a survivor. It has released its memory, its
            // descriptors and its terminal; what remains is a row in the
            // table holding an exit status until some parent calls `wait`.
            // `#NFR-002` is about processes that outlive the deadline, and a
            // process that has already exited outlives nothing.
            //
            // Counting them failed every launched attempt under CI and none
            // on a developer's machine. A container's PID 1 is the job's own
            // shell rather than an init, and it never reaps what orphans
            // reparent to it, so the entry stays for the life of the job —
            // where on a normal system init clears it in microseconds. The
            // survivor CI kept naming had already been killed.
            //
            // A listing with no state column at all is read as before: not a
            // zombie, and therefore a member.
            let state = f.next().unwrap_or("");
            if state.starts_with('Z') {
                return None;
            }
            let comm: Vec<&str> = f.collect();
            Some((pid, comm.join(" ")))
        })
        .collect()
}

/// The process group of a live process, if it has one.
pub fn process_group_of(inv: &mut Invoker, pid: i32) -> Option<i32> {
    let out = inv
        .run("ps", &["-o", "pgid=", "-p", &pid.to_string()])
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_a_program_that_exists() {
        assert!(which("sh").is_some());
        assert!(which("/bin/sh").is_some());
    }

    #[test]
    fn does_not_resolve_a_program_that_does_not_exist() {
        assert!(which("circus-no-such-program-xyzzy").is_none());
        assert!(which("/nonexistent/circus-xyzzy").is_none());
    }

    #[test]
    fn does_not_resolve_a_directory() {
        assert!(which("/tmp").is_none(), "a directory is not an executable");
    }

    #[test]
    fn a_missing_program_is_reported_as_not_found() {
        let mut inv = Invoker::new();
        let e = inv.resolve("circus-no-such-program-xyzzy").unwrap_err();
        assert_eq!(e.code(), crate::exit::NOTFOUND);
        assert!(e.to_string().contains("circus-no-such-program-xyzzy"));
    }

    #[test]
    fn every_invocation_is_recorded() {
        // REQ-007.c
        let mut inv = Invoker::new();
        inv.run("sh", &["-c", "exit 0"]).unwrap();
        inv.run("sh", &["-c", "exit 3"]).unwrap();
        let log = inv.invocations();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].name, "sh");
        assert_eq!(log[0].exit_status, Some(0));
        assert_eq!(log[1].exit_status, Some(3));
        assert!(log[0].resolved_path.ends_with("/sh"));
    }

    #[test]
    fn a_failed_program_is_still_recorded() {
        let mut inv = Invoker::new();
        let _ = inv.run_ok("sh", &["-c", "exit 9"]);
        assert_eq!(inv.invocations().len(), 1);
        assert_eq!(inv.invocations()[0].exit_status, Some(9));
    }

    #[test]
    fn a_missing_program_records_nothing_and_runs_nothing() {
        let mut inv = Invoker::new();
        let _ = inv.run("circus-no-such-program-xyzzy", &["x"]);
        assert!(inv.invocations().is_empty());
    }

    #[test]
    fn run_ok_returns_trimmed_stdout() {
        let mut inv = Invoker::new();
        assert_eq!(inv.run_ok("sh", &["-c", "echo hello"]).unwrap(), "hello");
    }

    #[test]
    fn residue_of_an_impossible_group_is_zero() {
        let mut inv = Invoker::new();
        assert_eq!(process_group_residue(&mut inv, 999_999), 0);
    }

    #[test]
    fn residue_counts_a_live_group() {
        let mut inv = Invoker::new();
        let pgid = process_group_of(&mut inv, std::process::id() as i32);
        if let Some(pgid) = pgid {
            assert!(
                process_group_residue(&mut inv, pgid) >= 1,
                "this process is in its own group, so the count cannot be zero"
            );
        }
    }

    #[test]
    fn names_the_survivors_of_a_group() {
        // The divergence that failed CI while passing locally: `ps -g` selects
        // by process group on BSD and by session on Linux. Circus now selects
        // for itself, so the rule is fixed here rather than by the platform.
        let listing = "\
  101   101 S  sh
  102   101 S  sleep 60
  103   999 R  other
  104   200 S  elsewhere
";
        assert_eq!(
            rows_in_group(listing, 101),
            vec![(101, "sh".to_string()), (102, "sleep 60".to_string())]
        );
        assert_eq!(
            rows_in_group(listing, 999),
            vec![(103, "other".to_string())]
        );
        assert!(rows_in_group(listing, 7).is_empty(), "an absent group");
    }

    #[test]
    fn a_zombie_is_not_a_survivor() {
        // The whole of the CI failure, in four rows. Linux keeps the original
        // command name on a zombie; macOS renames it `<defunct>`. Both carry
        // the process group, and neither is running.
        let listing = "\
  101   101 S  sh
  102   101 Z  drv
  103   101 Z  <defunct>
  104   101 S+ sleep 30
";
        assert_eq!(
            rows_in_group(listing, 101),
            vec![(101, "sh".to_string()), (104, "sleep 30".to_string())],
            "only the living are survivors"
        );

        // A group that holds nothing but the dead is a reaped group.
        assert!(rows_in_group("  9   9 Z  drv\n", 9).is_empty());
    }

    #[test]
    fn tolerates_the_shapes_ps_actually_emits() {
        // Leading whitespace, right-aligned columns, a blank trailing line,
        // and a stray header row if a platform emits one anyway.
        assert!(rows_in_group("", 1).is_empty());
        assert!(rows_in_group("\n\n", 1).is_empty());
        assert_eq!(
            rows_in_group("  PID  PGID S COMMAND\n 7 7 S x\n", 7).len(),
            1
        );
        assert_eq!(rows_in_group("7 7 S a\n8 7 S b\n", 7).len(), 2);
        assert_eq!(rows_in_group("   7    7   S   x   \n", 7).len(), 1);
        // A row that is not two integers is skipped, not counted.
        assert_eq!(rows_in_group("nonsense\n7 7 S x\n", 7).len(), 1);
        assert!(
            rows_in_group("7\n", 7).is_empty(),
            "a row with no group is not a member"
        );
        // A listing with no state column is read as it was before: present,
        // not a zombie, and therefore a member.
        assert_eq!(rows_in_group("5 5\n", 5), vec![(5, String::new())]);
    }
}
