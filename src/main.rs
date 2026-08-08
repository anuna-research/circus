//! The Circus command line — `SPEC-001-circus-agent-harness#CON-001` through
//! `#CON-004`.
//!
//! Every subcommand has the same shape: recognise the input in full, read the
//! run record, do one effectful thing, write the record back, print it. The
//! record is written before control returns to the lead in every case,
//! including the failing ones, because `#NFR-003.a` says so and because an
//! attempt that is invisible is an attempt nobody can recover.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use circus::core::decision::acceptance_decision;
use circus::core::grammar::{AttemptId, AttemptN, EvidenceRef, Task};
use circus::core::paths::{self, AttemptPaths, Roots};
use circus::core::prompt::recognise_prompt;
use circus::core::record::{self, Decision, Merge, RunRecord, State};
use circus::core::time::now_rfc3339;
use circus::core::verifier;
use circus::exit;
use circus::shell::proc::Invoker;
use circus::shell::{Error, Result, git, pane, state};

#[derive(Parser)]
#[command(
    name = "circus",
    version,
    about = "Run coding-agent CLIs in isolated Git worktrees",
    long_about = "Circus opens a ring for each act, records the result, and never \
                  decides whether the act was good.\n\n\
                  Specified by SPEC-001-circus-agent-harness."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create one isolated worktree and branch for one task attempt.
    Prepare {
        /// Task identifier: a lowercase letter, then up to 62 of
        /// [a-z0-9-].
        #[arg(long)]
        task: String,
        /// The branch or commit the attempt starts from.
        #[arg(long)]
        integration: String,
        /// Attempt number. Allocated automatically when omitted.
        #[arg(long)]
        attempt: Option<String>,
    },

    /// Start a prepared attempt in a named tmux pane, through withdone.
    Launch {
        /// The `task/attempt` handle printed by `prepare`.
        #[arg(long)]
        attempt: String,
        /// A prompt file containing the attempt's literal sentinel path.
        #[arg(long)]
        prompt: PathBuf,
        /// The driver to run, and its arguments.
        #[arg(last = true, required = true)]
        driver: Vec<String>,
    },

    /// Record an acceptance decision from a verifier record and evidence.
    Accept {
        #[arg(long)]
        attempt: String,
        /// JSON file naming the verifier command, its exit code, and its
        /// captured output.
        #[arg(long)]
        verifier_record: PathBuf,
        /// One or more durable Elephant references. Recorded verbatim and
        /// never resolved.
        #[arg(long, required = true, num_args = 1..)]
        evidence: Vec<String>,
    },

    /// Merge an accepted attempt into its recorded integration ref.
    Merge {
        #[arg(long)]
        attempt: String,
        #[arg(long)]
        into: String,
    },
}

fn main() -> ExitCode {
    // clap exits 2 on a usage error by default. Every contract in
    // `SPEC-001-circus-agent-harness` assigns 64 to input that fails
    // recognition, and an argument that is missing or malformed is exactly
    // that, so the code is remapped rather than left to differ by which layer
    // noticed.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let help = matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion
                    | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            );
            let _ = e.print();
            return ExitCode::from(if help { exit::OK } else { exit::USAGE } as u8);
        }
    };
    let mut inv = Invoker::new();

    let outcome = match cli.command {
        Command::Prepare {
            task,
            integration,
            attempt,
        } => prepare(&mut inv, &task, &integration, attempt.as_deref()),
        Command::Launch {
            attempt,
            prompt,
            driver,
        } => launch(&mut inv, &attempt, &prompt, &driver),
        Command::Accept {
            attempt,
            verifier_record,
            evidence,
        } => accept(&mut inv, &attempt, &verifier_record, &evidence),
        Command::Merge { attempt, into } => merge(&mut inv, &attempt, &into),
    };

    match outcome {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("circus: {e}");
            ExitCode::from(e.code() as u8)
        }
    }
}

fn cwd() -> Result<PathBuf> {
    std::env::current_dir().map_err(Error::Io)
}

/// Load the roots and the paths for an existing attempt.
fn locate(inv: &mut Invoker, attempt: &str) -> Result<(Roots, AttemptId, AttemptPaths)> {
    let id = AttemptId::recognise(attempt)?;
    let roots = state::discover_roots(inv, &cwd()?)?;
    let paths = paths::attempt_paths(&roots, &id);
    Ok((roots, id, paths))
}

/// Append this run's invocations and emit the record.
fn finish(inv: &mut Invoker, path: &Path, mut rec: RunRecord, code: i32) -> Result<i32> {
    rec.external_programs.extend(inv.take());
    state::write_record(path, &rec)?;
    print!("{}", record::serialise(&rec));
    Ok(code)
}

// ── CON-001 ────────────────────────────────────────────────────────────────

fn prepare(inv: &mut Invoker, task: &str, integration: &str, attempt: Option<&str>) -> Result<i32> {
    let task = Task::recognise(task)?;
    let requested = attempt.map(AttemptN::recognise).transpose()?;
    let cwd = cwd()?;

    let integration = git::recognise_ref(inv, integration)?;
    let roots = state::discover_roots(inv, &cwd)?;

    if !git::resolves(inv, &cwd, &integration)? {
        return Err(Error::usage(format!(
            "`{integration}` does not resolve to a commit in this repository"
        )));
    }

    // Allocation and creation are one critical section, so two concurrent
    // preparations cannot choose the same attempt number — `#CON-001`.
    let _lock = state::lock(&roots)?;

    let n = match requested {
        Some(n) => n,
        None => AttemptN::from_number(state::next_attempt(&roots, &task)?)
            .ok_or_else(|| Error::usage("no attempt number is available for this task"))?,
    };
    let id = AttemptId::new(task, n);
    let paths = paths::attempt_paths(&roots, &id);

    if state::attempt_exists(&paths.attempt_dir) {
        return Err(Error::usage(format!("attempt `{id}` already exists")));
    }
    if paths.worktree.exists() {
        return Err(Error::usage(format!(
            "{} already exists",
            paths.worktree.display()
        )));
    }
    if git::branch_exists(inv, &cwd, &paths.branch)? {
        return Err(Error::usage(format!(
            "branch `{}` already exists",
            paths.branch
        )));
    }

    std::fs::create_dir_all(&paths.attempt_dir)?;

    if let Err(e) = git::worktree_add(inv, &cwd, &paths.worktree, &paths.branch, &integration) {
        // "creates no partially registered attempt" — `#CON-001`.
        git::rollback(inv, &cwd, &paths.worktree, &paths.branch);
        let _ = std::fs::remove_dir_all(&paths.attempt_dir);
        return Err(e);
    }

    let rec = RunRecord::prepared(
        &id,
        &paths,
        roots.repository.clone(),
        integration,
        now_rfc3339(),
    );
    finish(inv, &paths.record, rec, exit::OK)
}

// ── CON-002 ────────────────────────────────────────────────────────────────

fn launch(inv: &mut Invoker, attempt: &str, prompt: &Path, driver: &[String]) -> Result<i32> {
    let (_roots, id, paths) = locate(inv, attempt)?;
    let mut rec = state::read_record(&paths.record)?;

    if rec.state != State::Prepared {
        return Err(Error::usage(format!(
            "attempt `{id}` is `{:?}` and cannot be launched; prepare a new attempt",
            rec.state
        )));
    }

    // Recognise the prompt in full before anything starts — `#REQ-003.c`.
    let bytes = std::fs::read(prompt).map_err(|e| {
        Error::usage(format!(
            "cannot read the prompt at {}: {e}",
            prompt.display()
        ))
    })?;
    recognise_prompt(&bytes, &paths.sentinel)?;

    // The prompt reaches the driver byte-for-byte — `#REQ-003.d`. Copying it
    // into the attempt directory also keeps it for review after the caller's
    // file is gone.
    let prompt_copy = paths.attempt_dir.join("prompt");
    std::fs::write(&prompt_copy, &bytes)?;

    let (driver_name, args) = driver.split_first().expect("clap requires at least one");

    let env = vec![
        ("CIRCUS_ATTEMPT".to_string(), id.to_string()),
        (
            "CIRCUS_PROMPT".to_string(),
            prompt_copy.to_string_lossy().into_owned(),
        ),
        (
            "CIRCUS_SENTINEL".to_string(),
            paths.sentinel.to_string_lossy().into_owned(),
        ),
        (
            "CIRCUS_WORKTREE".to_string(),
            paths.worktree.to_string_lossy().into_owned(),
        ),
    ];

    rec.pane_name = Some(paths.pane_name.clone());
    rec.log_path = Some(paths.log.to_string_lossy().into_owned());
    rec.sentinel_path = Some(paths.sentinel.to_string_lossy().into_owned());
    rec.timestamps.launched_at = Some(now_rfc3339());

    let outcome = match pane::launch(inv, &paths, driver_name, args, &env) {
        Ok(o) => o,
        Err(e) => {
            // The attempt stays `prepared` so a fixed environment can relaunch
            // it, and the record still shows the attempt was tried.
            let _ = finish(inv, &paths.record, rec, exit::OK);
            return Err(e);
        }
    };

    rec.transport_exit_code = Some(outcome.transport_exit_code);
    rec.sentinel_value = outcome.sentinel_value;
    rec.completion_method = outcome.completion_method;
    rec.process_group_residue = outcome.process_group_residue;
    rec.timestamps.completed_at = Some(now_rfc3339());

    // A group that outlived the deadline is a failed attempt — `#NFR-002.c`.
    // Note what this is not: a judgement about the work. `#REQ-004.b` keeps
    // the transport exit code out of the state entirely.
    rec.state = if outcome.process_group_residue > 0 {
        State::Failed
    } else {
        State::Completed
    };

    let code = if rec.state == State::Failed {
        exit::REJECTED
    } else {
        exit::OK
    };
    finish(inv, &paths.record, rec, code)
}

// ── CON-003 ────────────────────────────────────────────────────────────────

fn accept(
    inv: &mut Invoker,
    attempt: &str,
    verifier_record: &Path,
    evidence: &[String],
) -> Result<i32> {
    let (_roots, id, paths) = locate(inv, attempt)?;
    let mut rec = state::read_record(&paths.record)?;

    if rec.state != State::Completed {
        return Err(Error::usage(format!(
            "attempt `{id}` is `{:?}`; only a completed attempt can be accepted",
            rec.state
        )));
    }

    let refs: Vec<EvidenceRef> = evidence
        .iter()
        .map(|r| EvidenceRef::recognise(r))
        .collect::<std::result::Result<_, _>>()?;

    let bytes = std::fs::read(verifier_record).map_err(|e| {
        Error::usage(format!(
            "cannot read the verifier record at {}: {e}",
            verifier_record.display()
        ))
    })?;
    let verifier = verifier::recognise(&bytes)?;

    let decision = acceptance_decision(verifier.exit_code, &refs);

    rec.evidence_refs = refs.iter().map(|r| r.as_str().to_owned()).collect();
    rec.verifier = Some(verifier);
    rec.decision = Some(decision);
    rec.state = match decision {
        Decision::Accepted => State::Accepted,
        Decision::Rejected => State::Rejected,
    };
    rec.timestamps.decided_at = Some(now_rfc3339());

    let code = match decision {
        Decision::Accepted => exit::OK,
        Decision::Rejected => exit::REJECTED,
    };
    finish(inv, &paths.record, rec, code)
}

// ── CON-004 ────────────────────────────────────────────────────────────────

fn merge(inv: &mut Invoker, attempt: &str, into: &str) -> Result<i32> {
    let (roots, id, paths) = locate(inv, attempt)?;
    let mut rec = state::read_record(&paths.record)?;

    if !rec.is_accepted() {
        return Err(Error::usage(format!(
            "attempt `{id}` is `{:?}` and is not accepted; nothing was merged",
            rec.state
        )));
    }

    let into = git::recognise_ref(inv, into)?;
    if into != rec.integration_ref {
        return Err(Error::usage(format!(
            "attempt `{id}` was prepared from `{}`, not `{into}`; \
             merge into the ref it came from",
            rec.integration_ref
        )));
    }

    let cwd = cwd()?;
    let _lock = state::lock(&roots)?;
    let report = git::merge_into(inv, &cwd, &into, &rec.branch)?;

    let conflicted = report.outcome == circus::core::record::MergeOutcome::Conflict;
    if !conflicted {
        rec.state = State::Merged;
        rec.timestamps.merged_at = Some(now_rfc3339());
    }
    rec.merge = Some(Merge {
        result: report.outcome,
        commit: report.commit,
    });

    if conflicted {
        eprintln!(
            "circus: merge conflicted in {}; `{into}` is unchanged",
            report.conflicts.join(", ")
        );
    }
    let code = if conflicted { exit::REJECTED } else { exit::OK };
    finish(inv, &paths.record, rec, code)
}
