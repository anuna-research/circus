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
use std::time::Duration;

use clap::{Parser, Subcommand};

use circus::core::ansi;
use circus::core::decision::acceptance_decision;
use circus::core::grammar::{AttemptId, AttemptN, EvidenceRef, Task};
use circus::core::instruction::instruction_text;
use circus::core::paths::{self, AttemptPaths, Roots};
use circus::core::prompt::recognise_prompt;
use circus::core::record::{
    self, CompletionMethod, Decision, Merge, MergeOutcome, RunRecord, State,
};
use circus::core::time::{UnixSeconds, now_rfc3339};
use circus::core::verifier;
use circus::exit;
use circus::shell::proc::Invoker;
use circus::shell::status as shell_status;
use circus::shell::ui::{self, Level, Ui};
use circus::shell::{Error, Result, git, pane, state};

const EXAMPLES: &str = "\
Examples:
  # Open a ring. Prints a run record naming the attempt and its sentinel path.
  circus prepare --task model --integration main

  # Or do all three at once. Circus appends the completion instruction to
  # your task file; the file itself is not modified.
  circus spawn --task model --integration main \
      --task-file task.md -- circus-driver-codex

  # The long way, when you want to write the prompt yourself.
  { cat task.md; circus instruction --attempt model/1; } > prompt.md
  circus launch --attempt model/1 --prompt prompt.md -- circus-driver-codex

  # Record your own verdict. Circus never forms one.
  circus accept --attempt model/1 --verifier-record v.json --evidence theory:q42

  # Or merge as part of the same run, once accepted.
  circus accept --attempt model/1 --verifier-record v.json --evidence theory:q42 --auto-merge

  # What has already been tried on this task, for the next prompt.
  circus history --task model

  # Look in on it from another terminal while it runs.
  circus status --attempt model/1
  circus logs --attempt model/1 --follow --plain

  # See whether it would conflict, then merge.
  circus merge --attempt model/1 --into main --dry-run
  circus merge --attempt model/1 --into main

Output:
  stdout carries the run record as JSON, and nothing else.
  stderr carries progress, hints, and errors.

Exit codes:
  0 success · 1 rejected or conflicted · 64 bad input · 65 bad record
  70 an external program failed · 127 a required program is missing

Docs: docs/circus/index.md · Specification: specs/SPEC-001-circus-agent-harness.md";

#[derive(Parser)]
#[command(
    name = "circus",
    version,
    about = "Run coding-agent CLIs in isolated Git worktrees",
    long_about = "Circus opens a ring for each act, records the result, and never \
                  decides whether the act was good.\n\n\
                  Specified by SPEC-001-circus-agent-harness.",
    after_help = EXAMPLES,
    after_long_help = EXAMPLES
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Suppress every message that is not an error.
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// Report each external program Circus invokes.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Never style stderr, even when it is a terminal.
    #[arg(long, global = true)]
    no_color: bool,
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

    /// Prepare an attempt and launch it, in one command.
    Spawn {
        /// Task identifier: a lowercase letter, then up to 62 of [a-z0-9-].
        #[arg(long)]
        task: String,
        /// The branch or commit the attempt starts from.
        #[arg(long)]
        integration: String,
        /// The task description. `-` reads it from stdin. Circus appends the
        /// completion instruction; your file is not modified.
        #[arg(long)]
        task_file: PathBuf,
        /// Attempt number. Allocated automatically when omitted.
        #[arg(long)]
        attempt: Option<String>,
        /// After the driver finishes, run this as the verifier and — if it
        /// exits 0 — accept with the given `--evidence` and merge, all in
        /// this one run. The verifier's exit code decides accept/reject
        /// exactly as it would from a `circus accept --verifier-record` a
        /// caller built by hand; Circus is just the one running it here.
        /// Requires `--evidence`.
        #[arg(long, num_args = 1..)]
        verifier: Vec<String>,
        /// Evidence for `--auto-merge`'s verifier run. Same shape and
        /// meaning as `accept --evidence`.
        #[arg(long, num_args = 1..)]
        evidence: Vec<String>,
        /// After the driver finishes, run `--verifier` and merge only if it
        /// exits 0. Requires `--verifier` and `--evidence`.
        #[arg(long, conflicts_with = "force_merge")]
        auto_merge: bool,
        /// Merge the moment the driver process exits 0 — no verifier, no
        /// evidence, no independent check at all. Named for what it is: an
        /// explicit bypass of the separation `#REQ-004.b` otherwise keeps
        /// between "the driver stopped" and "the work is good". For a
        /// caller who has already decided that gate does not apply here,
        /// not a shortcut to reach for by default.
        #[arg(long, conflicts_with = "auto_merge")]
        force_merge: bool,
        /// The driver to run, and its arguments.
        #[arg(last = true, required = true)]
        driver: Vec<String>,
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
        /// On acceptance, immediately merge into the ref the attempt was
        /// prepared from — the same as running `circus merge` right after,
        /// with one run record instead of two. A rejected attempt is never
        /// merged, and a conflicting merge still reports `rejected`: this
        /// changes nothing about when a merge is allowed to happen, only
        /// whether a second command is needed to ask for it.
        #[arg(long)]
        auto_merge: bool,
    },

    /// Write an attempt's transcript — what the agent itself printed.
    Logs {
        /// The `task/attempt` handle.
        #[arg(long)]
        attempt: String,
        /// Keep writing until the attempt stops running.
        #[arg(short, long)]
        follow: bool,
        /// Remove terminal control sequences.
        #[arg(long)]
        plain: bool,
    },

    /// Report every attempt of a task: what was tried, and what came of it.
    History {
        /// The task whose attempts to report.
        #[arg(long)]
        task: String,
    },

    /// Report what is true of an attempt right now, recorded and live.
    Status {
        /// One attempt. Omit to report every attempt in the repository.
        #[arg(long)]
        attempt: Option<String>,
    },

    /// Print the completion instruction a prompt for this attempt needs.
    Instruction {
        /// The `task/attempt` handle printed by `prepare`.
        #[arg(long)]
        attempt: String,
    },

    /// Merge an accepted attempt into its recorded integration ref.
    Merge {
        #[arg(long)]
        attempt: String,
        #[arg(long)]
        into: String,
        /// Report whether the merge conflicts, and change nothing.
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
}

fn main() -> ExitCode {
    // clap exits 2 on a usage error by default. Every contract in
    // `SPEC-001-circus-agent-harness` assigns 64 to input that fails
    // recognition, and an argument that is missing or malformed is exactly
    // that, so the code is remapped rather than left to differ by which layer
    // noticed.
    // `--no-color` has to be honoured by clap's own error and help rendering
    // too, and clap decides that before it has parsed anything. A pre-scan of
    // argv is the only place the flag can reach it — `ADR-008`.
    let mut command = <Cli as clap::CommandFactory>::command();
    if std::env::args_os().any(|a| a == "--no-color") {
        command = command.color(clap::ColorChoice::Never);
    }

    let cli = match command
        .try_get_matches_from_mut(std::env::args_os())
        .and_then(|m| <Cli as clap::FromArgMatches>::from_arg_matches(&m))
    {
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
    let level = match (cli.quiet, cli.verbose) {
        (true, _) => Level::Quiet,
        (_, true) => Level::Verbose,
        _ => Level::Normal,
    };
    let ui = Ui::new(level, cli.no_color);
    let mut inv = Invoker::with_ui(ui);

    let outcome = match cli.command {
        Command::Prepare {
            task,
            integration,
            attempt,
        } => prepare(&mut inv, ui, &task, &integration, attempt.as_deref()),
        Command::Spawn {
            task,
            integration,
            task_file,
            attempt,
            verifier,
            evidence,
            auto_merge,
            force_merge,
            driver,
        } => spawn(
            &mut inv,
            ui,
            &task,
            &integration,
            &task_file,
            attempt.as_deref(),
            &driver,
            &verifier,
            &evidence,
            auto_merge,
            force_merge,
        ),
        Command::Launch {
            attempt,
            prompt,
            driver,
        } => launch(&mut inv, ui, &attempt, &prompt, &driver),
        Command::Accept {
            attempt,
            verifier_record,
            evidence,
            auto_merge,
        } => accept(
            &mut inv,
            ui,
            &attempt,
            &verifier_record,
            &evidence,
            auto_merge,
        ),
        Command::Logs {
            attempt,
            follow,
            plain,
        } => logs(&mut inv, ui, &attempt, follow, plain),
        Command::History { task } => history(&mut inv, ui, &task),
        Command::Status { attempt } => status(&mut inv, ui, attempt.as_deref()),
        Command::Instruction { attempt } => instruction(&mut inv, ui, &attempt),
        Command::Merge {
            attempt,
            into,
            dry_run,
        } => merge(&mut inv, ui, &attempt, &into, dry_run),
    };

    match outcome {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            ui.end_progress();
            ui.error(&e.to_string());
            if let Some(next) = e.suggestion() {
                ui.hint(&next);
            }
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

fn prepare(
    inv: &mut Invoker,
    ui: Ui,
    task: &str,
    integration: &str,
    attempt: Option<&str>,
) -> Result<i32> {
    let (_, paths, rec) = do_prepare(inv, ui, task, integration, attempt)?;
    finish(inv, &paths.record, rec, exit::OK)
}

/// The whole of `#CON-001`, without printing. `spawn` reuses it, which is what
/// keeps `#REQ-013` a shorthand for two contracts rather than a third one.
fn do_prepare(
    inv: &mut Invoker,
    ui: Ui,
    task: &str,
    integration: &str,
    attempt: Option<&str>,
) -> Result<(AttemptId, AttemptPaths, RunRecord)> {
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

    let mut rec = RunRecord::prepared(
        &id,
        &paths,
        roots.repository.clone(),
        integration,
        now_rfc3339(),
    );
    // The sentinel path is derived, not discovered, so it is known now. An
    // operator building a prompt needs it before the launch, and copying it
    // from the record is the only way to get the exact bytes REQ-003.c wants.
    rec.sentinel_path = Some(paths.sentinel.to_string_lossy().into_owned());

    ui.state(&format!("prepared {id}"));
    ui.hint(&format!("worktree:  {}", paths.worktree.display()));
    ui.hint(&format!("branch:    {}", paths.branch));
    ui.emphasis("the prompt must contain this path, exactly:");
    ui.emphasis(&format!("  {}", paths.sentinel.display()));
    ui.hint(&format!(
        "next: circus launch --attempt {id} --prompt <file> -- <driver>"
    ));

    Ok((id, paths, rec))
}

// ── CON-011 ────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn spawn(
    inv: &mut Invoker,
    ui: Ui,
    task: &str,
    integration: &str,
    task_file: &Path,
    attempt: Option<&str>,
    driver: &[String],
    verifier: &[String],
    evidence: &[String],
    auto_merge: bool,
    force_merge: bool,
) -> Result<i32> {
    if auto_merge && (verifier.is_empty() || evidence.is_empty()) {
        return Err(Error::usage("--auto-merge needs --verifier and --evidence"));
    }

    // Read the task before preparing anything. A missing file is a usage
    // error, and finding that out after a worktree exists leaves an attempt
    // nobody asked for.
    let task_bytes = if task_file == Path::new("-") {
        use std::io::Read;
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        buf
    } else {
        std::fs::read(task_file).map_err(|e| {
            Error::usage(format!(
                "cannot read the task at {}: {e}",
                task_file.display()
            ))
        })?
    };

    let (id, paths, rec) = do_prepare(inv, ui, task, integration, attempt)?;

    // `{ cat task; circus instruction; }`, done here — `#REQ-013.b`. It is
    // written into the attempt directory and nowhere else, so the caller's
    // file is untouched and the composed prompt stays readable.
    let composed = paths.attempt_dir.join("prompt");
    let mut bytes = task_bytes;
    bytes.extend_from_slice(instruction_text(&paths.sentinel).as_bytes());
    std::fs::write(&composed, &bytes)?;
    ui.hint(&format!("prompt:    {}", composed.display()));

    // A launch failure leaves the prepared attempt standing — `#REQ-013.d`,
    // which follows from NFR-001.b.
    let (mut rec, mut code) = do_launch(
        inv,
        ui,
        &id,
        &paths,
        rec,
        &composed,
        driver,
        auto_merge || force_merge,
    )?;

    // A failed transport (a process group that outlived the cleanup
    // deadline) never merges, flag or no flag — that state already means
    // something went wrong at the process level, before any question of
    // whether the work itself is good.
    if rec.state == State::Failed || !(auto_merge || force_merge) {
        return finish(inv, &paths.record, rec, code);
    }

    let roots = state::discover_roots(inv, &cwd()?)?;

    if force_merge {
        match rec.transport_exit_code {
            Some(0) => {
                let into = rec.integration_ref.clone();
                let cwd = cwd()?;
                let report = apply_merge_to_record(inv, &roots, &cwd, &mut rec, &into)?;
                if report.outcome == MergeOutcome::Conflict {
                    ui.state(&format!(
                        "{id} conflicted merging into `{into}` in {}",
                        report.conflicts.join(", ")
                    ));
                    ui.hint(&format!(
                        "`{into}` is unchanged and the attempt is untouched"
                    ));
                    ui.hint("see docs/circus/how-to/how-to-resolve-a-merge-conflict.md");
                    code = exit::REJECTED;
                } else {
                    let short = report
                        .commit
                        .as_deref()
                        .map_or("(unknown)", |c| c.get(..12).unwrap_or(c));
                    ui.state(&format!("force-merged {id} into `{into}` as {short}"));
                    ui.hint(
                        "no verifier ran and no evidence was recorded — \
                         --force-merge skips both",
                    );
                    code = exit::OK;
                }
            }
            other => ui.hint(&format!(
                "--force-merge skipped: the driver exited {}, not 0",
                other.map_or("(none)".to_string(), |c| c.to_string())
            )),
        }
        return finish(inv, &paths.record, rec, code);
    }

    // `--auto-merge`: run the caller's verifier here, in the worktree, and
    // decide from its exit code — never from the driver's own. Otherwise
    // this is exactly `accept --auto-merge`, with Circus as the one running
    // the verifier instead of the caller.
    let (verifier_program, verifier_args) = verifier
        .split_first()
        .expect("--auto-merge requires --verifier, checked above");
    let output = inv.run_in(verifier_program, verifier_args, Some(&paths.worktree))?;
    let mut captured = output.stdout.clone();
    captured.extend_from_slice(&output.stderr);
    let output_path = paths.attempt_dir.join("auto-verifier-output.raw");
    std::fs::write(&output_path, &captured)?;

    // Built as JSON and re-read through the one recogniser rather than the
    // struct literal directly — `#REQ-005`-adjacent: a verifier record this
    // process assembles is still a verifier record, and it should pass
    // exactly the same validation as one a caller wrote by hand, not a
    // second, unchecked path to the same fields.
    let verifier_bytes = serde_json::to_vec(&serde_json::json!({
        "command": verifier,
        "exit_code": output.status.code().unwrap_or(1).clamp(0, 255),
        "output_path": output_path.to_string_lossy(),
    }))
    .expect("a command list, an integer, and a path all serialise");
    let verifier_record = verifier::recognise(&verifier_bytes)?;

    code = decide_and_maybe_merge(
        inv,
        ui,
        &roots,
        &id,
        &paths,
        &mut rec,
        verifier_record,
        evidence,
        true,
    )?;
    finish(inv, &paths.record, rec, code)
}

// ── CON-002 ────────────────────────────────────────────────────────────────

fn launch(
    inv: &mut Invoker,
    ui: Ui,
    attempt: &str,
    prompt: &Path,
    driver: &[String],
) -> Result<i32> {
    let (_roots, id, paths) = locate(inv, attempt)?;
    let rec = state::read_record(&paths.record)?;
    let (rec, code) = do_launch(inv, ui, &id, &paths, rec, prompt, driver, false)?;
    finish(inv, &paths.record, rec, code)
}

/// The whole of `#CON-002`, taking the record its caller already read.
/// Leaves `finish` to the caller — `spawn` may still have `--auto-merge` or
/// `--force-merge` work to fold into the same record and the same one
/// printed JSON document, and only the caller knows whether that's coming.
/// `suppress_next_hint` silences the plain "run your verifier by hand"
/// pointer when the caller (`spawn`, asked to auto/force-merge) is about to
/// say something more specific instead.
#[allow(clippy::too_many_arguments)]
fn do_launch(
    inv: &mut Invoker,
    ui: Ui,
    id: &AttemptId,
    paths: &AttemptPaths,
    mut rec: RunRecord,
    prompt: &Path,
    driver: &[String],
    suppress_next_hint: bool,
) -> Result<(RunRecord, i32)> {
    if rec.state != State::Prepared {
        return Err(Error::usage(format!(
            "attempt `{id}` is `{:?}` and cannot be launched; prepare a new attempt",
            rec.state
        )));
    }
    // A `prepared` record whose pane is alive is an attempt someone is
    // watching, or was until they interrupted the launch. Refusing here says
    // so; without it tmux refuses a duplicate session name and the operator
    // has to work out why.
    if rec.timestamps.launched_at.is_some()
        && inv
            .run("tmux", &["has-session", "-t", &paths.pane_name])
            .map(|o| o.status.success())
            .unwrap_or(false)
    {
        return Err(Error::usage(format!(
            "attempt `{id}` is already running in pane `{}`; \
             watch it with `tmux attach -t {}`",
            paths.pane_name, paths.pane_name
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

    // Persist before waiting, not after. The pane outlives this process, so a
    // `circus launch` that is interrupted leaves an agent working with no
    // record of it — and `circus status` has nothing to report. Writing here
    // is what makes `#REQ-011.a` answerable for a running attempt.
    state::write_record(&paths.record, &rec)?;

    let outcome = match pane::launch(inv, paths, driver_name, args, &env, ui) {
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

    match rec.completion_method {
        CompletionMethod::Sentinel => ui.state(&format!(
            "{id} finished: the worker wrote {} to its sentinel",
            outcome.sentinel_value.unwrap_or_default()
        )),
        _ => ui.state(&format!(
            "{id} finished: the driver exited {} without writing a sentinel",
            outcome.transport_exit_code
        )),
    }
    if rec.state == State::Failed {
        ui.state(&format!(
            "{id} failed: {} process(es) outlived the cleanup deadline",
            outcome.process_group_residue
        ));
    } else if suppress_next_hint {
        ui.hint("this says the agent stopped, not that its work is good");
    } else {
        ui.hint("this says the agent stopped, not that its work is good");
        ui.hint(&format!(
            "next: run your verifier, then circus accept --attempt {id} \
--verifier-record <file> --evidence <ref>"
        ));
    }
    Ok((rec, code))
}

// ── CON-003 ────────────────────────────────────────────────────────────────

fn accept(
    inv: &mut Invoker,
    ui: Ui,
    attempt: &str,
    verifier_record: &Path,
    evidence: &[String],
    auto_merge: bool,
) -> Result<i32> {
    let (roots, id, paths) = locate(inv, attempt)?;
    let mut rec = state::read_record(&paths.record)?;

    if rec.state != State::Completed {
        return Err(Error::usage(format!(
            "attempt `{id}` is `{:?}`; only a completed attempt can be accepted",
            rec.state
        )));
    }

    let bytes = std::fs::read(verifier_record).map_err(|e| {
        Error::usage(format!(
            "cannot read the verifier record at {}: {e}",
            verifier_record.display()
        ))
    })?;
    let verifier = verifier::recognise(&bytes)?;

    let code = decide_and_maybe_merge(
        inv, ui, &roots, &id, &paths, &mut rec, verifier, evidence, auto_merge,
    )?;
    finish(inv, &paths.record, rec, code)
}

/// The decision-and-maybe-merge core shared by `accept` and `spawn
/// --auto-merge`: records the verifier and evidence, decides accepted or
/// rejected, and — when `auto_merge` and accepted — folds a merge into the
/// same `rec` too. The two callers differ only in where `verifier` came from
/// (a file the caller already wrote, vs. one Circus just ran itself); this
/// owns the mutation and the messaging. Never calls `finish` — exactly one
/// call to that belongs to each caller, so stdout carries exactly one JSON
/// document no matter how many things happened in the run.
#[allow(clippy::too_many_arguments)]
fn decide_and_maybe_merge(
    inv: &mut Invoker,
    ui: Ui,
    roots: &Roots,
    id: &AttemptId,
    paths: &AttemptPaths,
    rec: &mut RunRecord,
    verifier: circus::core::verifier::VerifierRecord,
    evidence: &[String],
    auto_merge: bool,
) -> Result<i32> {
    let refs: Vec<EvidenceRef> = evidence
        .iter()
        .map(|r| EvidenceRef::recognise(r))
        .collect::<std::result::Result<_, _>>()?;

    let decision = acceptance_decision(verifier.exit_code, &refs);

    // `#REQ-014` — the record has always named the caller's output_path, and a
    // path is not an artefact. Copy it beside the attempt so a rejected one is
    // inspectable later, as `#NFR-001.a` promises. Deliberately after the
    // decision is computed and unable to affect it: `#REQ-014.d`.
    match state::capture_verifier_log(
        Path::new(&verifier.output_path),
        &paths.attempt_dir.join("verifier.log"),
    ) {
        Some((path, truncated)) => {
            rec.verifier_log = Some(path.to_string_lossy().into_owned());
            rec.verifier_log_truncated = truncated;
            if truncated {
                ui.hint("the verifier log was longer than 1 MiB and was truncated");
            }
        }
        None => ui.hint(&format!(
            "could not read the verifier output at {}; the decision stands",
            verifier.output_path
        )),
    }

    rec.evidence_refs = refs.iter().map(|r| r.as_str().to_owned()).collect();
    rec.verifier = Some(verifier);
    rec.decision = Some(decision);
    rec.state = match decision {
        Decision::Accepted => State::Accepted,
        Decision::Rejected => State::Rejected,
    };
    rec.timestamps.decided_at = Some(now_rfc3339());

    let accepted = matches!(decision, Decision::Accepted);
    match decision {
        Decision::Accepted => {
            ui.state(&format!("accepted {id}"));
            if !auto_merge {
                ui.hint(&format!(
                    "next: circus merge --attempt {id} --into {}",
                    rec.integration_ref
                ));
            }
        }
        Decision::Rejected => {
            ui.state(&format!(
                "rejected {id}: the verifier exited {}",
                rec.verifier.as_ref().map_or(-1, |v| v.exit_code)
            ));
            ui.hint(&format!(
                "the attempt is preserved at {}",
                paths.worktree.display()
            ));
        }
    };

    if !(auto_merge && accepted) {
        return Ok(if accepted { exit::OK } else { exit::REJECTED });
    }

    // `--auto-merge`: fold a merge into this same run rather than requiring
    // a second `circus merge` invocation. `rec.integration_ref`
    // is already the ref `git::recognise_ref` validated back at `prepare`
    // time, so there is no untrusted `--into` here to re-recognise — unlike
    // standalone `merge`, which takes one from the caller.
    let into = rec.integration_ref.clone();
    let cwd = cwd()?;
    let report = apply_merge_to_record(inv, roots, &cwd, rec, &into)?;
    let conflicted = report.outcome == MergeOutcome::Conflict;

    if conflicted {
        ui.state(&format!(
            "{id} accepted, but conflicted merging into `{into}` in {}",
            report.conflicts.join(", ")
        ));
        ui.hint(&format!(
            "`{into}` is unchanged and the attempt is untouched"
        ));
        ui.hint("see docs/circus/how-to/how-to-resolve-a-merge-conflict.md");
    } else {
        let short = report
            .commit
            .as_deref()
            .map_or("(unknown)", |c| c.get(..12).unwrap_or(c));
        ui.state(&format!("merged {id} into `{into}` as {short}"));
        ui.hint(&format!(
            "the attempt is preserved; remove it with `git worktree remove {}`",
            paths.worktree.display()
        ));
    }

    Ok(if conflicted { exit::REJECTED } else { exit::OK })
}

/// The effectful core shared by `merge` and `accept --auto-merge`: locks the
/// repo, plans the merge, applies it, and mutates `rec` to reflect the
/// outcome. Never a dry run — `merge`'s own `--dry-run` branch returns before
/// reaching here, and `--auto-merge` never previews, since a preview is a
/// question and auto_merge is already an answer.
///
/// `#REQ-009`'s "the preview is the merge with the apply step omitted, so the
/// two can never disagree" is what lets this read `report.outcome` — from the
/// applied merge — as the one source of truth for whether it conflicted,
/// rather than needing the caller to also carry a `MergePlan` around.
fn apply_merge_to_record(
    inv: &mut Invoker,
    roots: &Roots,
    cwd: &Path,
    rec: &mut RunRecord,
    into: &str,
) -> Result<git::MergeReport> {
    let _lock = state::lock(roots)?;
    let plan = git::plan_merge(inv, cwd, into, &rec.branch)?;
    let report = git::apply_merge(inv, cwd, into, &rec.branch, &plan)?;

    if report.outcome != MergeOutcome::Conflict {
        rec.state = State::Merged;
        rec.timestamps.merged_at = Some(now_rfc3339());
    }
    rec.merge = Some(Merge {
        result: report.outcome,
        commit: report.commit.clone(),
    });
    Ok(report)
}

// ── CON-010 ────────────────────────────────────────────────────────────────

fn logs(inv: &mut Invoker, ui: Ui, attempt: &str, follow: bool, plain: bool) -> Result<i32> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let (_roots, id, paths) = locate(inv, attempt)?;
    // Reading the record is what makes this a statement about a real attempt,
    // and it is the pre-condition CON-010 states.
    let _rec = state::read_record(&paths.record)?;

    let emit = |bytes: &[u8]| {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(&if plain {
            ansi::strip(bytes)
        } else {
            bytes.to_vec()
        });
        let _ = out.flush();
    };

    let Ok(mut file) = std::fs::File::open(&paths.log) else {
        // A prepared attempt has printed nothing. That is an answer.
        ui.state(&format!("{id} has produced no output yet"));
        return Ok(exit::OK);
    };

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    emit(&buf);
    if !follow {
        return Ok(exit::OK);
    }

    ui.state(&format!("following {id}; it stops when the attempt does"));
    let mut pos = file.stream_position()?;
    loop {
        let alive = inv
            .run("tmux", &["has-session", "-t", &paths.pane_name])
            .map(|o| o.status.success())
            .unwrap_or(false);

        file.seek(SeekFrom::Start(pos))?;
        let mut chunk = Vec::new();
        file.read_to_end(&mut chunk)?;
        if chunk.is_empty() {
            // Nothing new, and nothing left to produce it — `#REQ-012.b`.
            if !alive {
                ui.state(&format!("{id} stopped"));
                return Ok(exit::OK);
            }
        } else {
            pos += chunk.len() as u64;
            emit(&chunk);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

// ── CON-012 ────────────────────────────────────────────────────────────────

fn history(inv: &mut Invoker, ui: Ui, task: &str) -> Result<i32> {
    let task = Task::recognise(task)?;
    let cwd = cwd()?;
    let roots = state::discover_roots(inv, &cwd)?;

    let ids: Vec<AttemptId> = shell_status::all_attempts(&roots)
        .into_iter()
        .filter(|i| i.task == task)
        .collect();

    if ids.is_empty() {
        ui.state(&format!("no attempts for `{task}` yet"));
        ui.hint(&format!(
            "next: circus prepare --task {task} --integration <ref>"
        ));
        return Ok(exit::OK);
    }

    for id in &ids {
        let p = paths::attempt_paths(&roots, id);
        let rec = match shell_status::record_of(&roots, id) {
            Ok(r) => r,
            // One unreadable record does not withhold the rest — `#CON-012`.
            Err(e) => {
                println!("attempt {id}  —  unreadable ({e})\n");
                continue;
            }
        };

        let verdict = rec
            .decision
            .map(|d| format!("{d:?}").to_lowercase())
            .unwrap_or_else(|| format!("{:?}", rec.state).to_lowercase());
        println!("attempt {id}  —  {verdict}");

        if let Some(v) = &rec.verifier {
            println!(
                "  verifier   {} → exit {}",
                v.command.join(" "),
                v.exit_code
            );
        }
        if let Some(log) = &rec.verifier_log {
            let note = if rec.verifier_log_truncated {
                " (truncated at 1 MiB)"
            } else {
                ""
            };
            println!("  log        {log}{note}");
        }
        if !rec.evidence_refs.is_empty() {
            println!("  evidence   {}", rec.evidence_refs.join(", "));
        }

        // What the branch changed says more than a terminal capture does, and
        // an attempt that changed nothing while reporting success is the single
        // most useful fact a later attempt can be told — `#REQ-015.b`.
        let range = format!("{}...{}", rec.integration_ref, rec.branch);
        let changed = inv
            .run_ok_in("git", &["diff", "--shortstat", &range], Some(&cwd))
            .unwrap_or_default();
        println!(
            "  changed    {}",
            if changed.is_empty() {
                "nothing"
            } else {
                changed.trim()
            }
        );
        let commits = inv
            .run_ok_in(
                "git",
                &[
                    "log",
                    "--oneline",
                    &format!("{}..{}", rec.integration_ref, rec.branch),
                ],
                Some(&cwd),
            )
            .unwrap_or_default();
        for c in commits.lines() {
            println!("    {c}");
        }

        if p.log.exists() {
            println!("  transcript {}", p.log.display());
        }
        println!();
    }

    ui.state(&format!("{} attempt(s) for `{task}`", ids.len()));
    Ok(exit::OK)
}

// ── CON-009 ────────────────────────────────────────────────────────────────

fn status(inv: &mut Invoker, ui: Ui, attempt: Option<&str>) -> Result<i32> {
    let cwd = cwd()?;
    let roots = state::discover_roots(inv, &cwd)?;
    let now = UnixSeconds::now();

    let one = |inv: &mut Invoker, id: &AttemptId| -> Result<circus::shell::status::Status> {
        let rec = shell_status::record_of(&roots, id)?;
        shell_status::observe(inv, &roots, &cwd, id, &rec, now)
    };

    // With an attempt, one object. Without, an array — `#CON-009`.
    let (rendered, reports) = match attempt {
        Some(a) => {
            let id = AttemptId::recognise(a)?;
            let s = one(inv, &id)?;
            (serde_json::to_string_pretty(&s), vec![s])
        }
        None => {
            let mut all = Vec::new();
            for id in shell_status::all_attempts(&roots) {
                // A record that will not parse should not hide every other
                // attempt from the listing.
                match one(inv, &id) {
                    Ok(s) => all.push(s),
                    Err(e) => ui.hint(&format!("skipping {id}: {e}")),
                }
            }
            (serde_json::to_string_pretty(&all), all)
        }
    };
    println!("{}", rendered.expect("a status document always serialises"));

    for s in &reports {
        let live = &s.live;
        let recorded = format!("{:?}", s.attempt.state).to_lowercase();
        // A running attempt's record still reads `prepared`, because the state
        // only advances once the launch returns. Leading with the live fact and
        // naming the recorded one second says both without contradicting itself.
        if live.pane_alive {
            let elapsed = live
                .running_for_seconds
                .map(|n| ui::human_duration(Duration::from_secs(n as u64)))
                .unwrap_or_else(|| "an unknown time".into());
            ui.state(&format!(
                "{} — running for {elapsed} (recorded: {recorded})",
                s.attempt.attempt_id
            ));
            ui.emphasis(&format!("watch it:  tmux attach -t {}", live.pane));
            ui.hint(&format!(
                "read it:   circus logs --attempt {} --follow --plain",
                s.attempt.attempt_id
            ));
        } else {
            ui.state(&format!("{} — {recorded}", s.attempt.attempt_id));
        }
    }
    if reports.is_empty() {
        ui.state("no attempts in this repository");
        ui.hint("next: circus prepare --task <name> --integration <ref>");
    }
    Ok(exit::OK)
}

// ── CON-008 ────────────────────────────────────────────────────────────────

fn instruction(inv: &mut Invoker, ui: Ui, attempt: &str) -> Result<i32> {
    let (_roots, id, paths) = locate(inv, attempt)?;
    // Reading the record is what makes this a statement about a real attempt
    // rather than a path this command derived for itself.
    let rec = state::read_record(&paths.record)?;
    let sentinel = rec
        .sentinel_path
        .as_deref()
        .ok_or_else(|| Error::usage(format!("attempt `{id}` has no sentinel path recorded")))?;

    // stdout, because it is this command's primary output — `#REQ-008.b`.
    print!("{}", instruction_text(Path::new(sentinel)));

    ui.state(&format!("completion instruction for {id}"));
    ui.hint("append it to your task description:");
    ui.hint(&format!(
        "  {{ cat task.md; circus instruction --attempt {id}; }} > prompt.md"
    ));
    Ok(exit::OK)
}

// ── CON-004 ────────────────────────────────────────────────────────────────

fn merge(inv: &mut Invoker, ui: Ui, attempt: &str, into: &str, dry_run: bool) -> Result<i32> {
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

    if dry_run {
        // `#REQ-009.b` — no ref moves, no worktree is refreshed, and no record
        // is written. The attempt is exactly as it was. This is the only
        // caller of `plan_merge` on its own; `apply_merge_to_record` below
        // computes and applies a plan in one step for every real merge, per
        // `#REQ-009`'s "the preview is the merge with the apply step
        // omitted, so the two can never disagree".
        let _lock = state::lock(&roots)?;
        let plan = git::plan_merge(inv, &cwd, &into, &rec.branch)?;
        let conflicted = plan.outcome == MergeOutcome::Conflict;
        if conflicted {
            ui.state(&format!(
                "{id} would conflict with `{into}` in {}",
                plan.conflicts.join(", ")
            ));
            ui.hint("see docs/circus/how-to/how-to-resolve-a-merge-conflict.md");
        } else {
            ui.state(&format!("{id} merges cleanly into `{into}`"));
            ui.hint(&format!("next: circus merge --attempt {id} --into {into}"));
        }
        ui.hint("this was a dry run; nothing changed");
        return Ok(if conflicted { exit::REJECTED } else { exit::OK });
    }

    let report = apply_merge_to_record(inv, &roots, &cwd, &mut rec, &into)?;
    let conflicted = report.outcome == MergeOutcome::Conflict;

    if conflicted {
        ui.state(&format!(
            "{id} conflicted with `{into}` in {}",
            report.conflicts.join(", ")
        ));
        ui.hint(&format!(
            "`{into}` is unchanged and the attempt is untouched"
        ));
        ui.hint("see docs/circus/how-to/how-to-resolve-a-merge-conflict.md");
    } else {
        // A short hash is what a person quotes back; the full one is in the
        // record for anything that needs it.
        let short = report
            .commit
            .as_deref()
            .map_or("(unknown)", |c| c.get(..12).unwrap_or(c));
        ui.state(&format!("merged {id} into `{into}` as {short}"));
        ui.hint(&format!(
            "the attempt is preserved; remove it with `git worktree remove {}`",
            paths.worktree.display()
        ));
    }
    let code = if conflicted { exit::REJECTED } else { exit::OK };
    finish(inv, &paths.record, rec, code)
}
