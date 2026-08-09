---
title: Command line
mode: reference
spec: SPEC-001
---

# Command line

The four Circus commands. Contracts CON-001 through CON-004 are the normative
source.

## Shared productions

Every command recognises its arguments in full before it acts.

```abnf
task         = LOWER *62( LOWER / DIGIT / "-" )
attempt-n    = NZDIGIT *3DIGIT                 ; 1..9999
attempt-id   = task "/" attempt-n
evidence-ref = 1*128( ALPHA / DIGIT / "-" / "_" / "." / ":" / "/" )
```

A ref must not begin with `-`, must carry no control character, and must be
accepted by `git check-ref-format --branch`.

## Global options

Accepted by every command, meaning the same thing in each.

| Option | Default | Effect |
|---|---|---|
| `-q`, `--quiet` | off | Suppress every message that is not an error |
| `-v`, `--verbose` | off | Report each external program invoked |
| `--no-color` | off | Never style stderr, even on a terminal |
| `-h`, `--help` | — | Print help and exit 0 |
| `-V`, `--version` | — | Print the version and exit 0 |

`--quiet` and `--verbose` are mutually exclusive. Supplying both exits 64.

Stdout carries the run record and nothing else, whatever these are set to.
`--quiet` silences stderr; it never withholds the record.

## Streams and styling

| Stream | Carries |
|---|---|
| stdout | The run record, as JSON. Nothing else, ever |
| stderr | Progress, state changes, hints, and errors |

Stderr is styled only when it is a terminal, `--no-color` was not given,
`NO_COLOR` is unset or empty, and `TERM` is not `dumb`. Stdout is never styled.

Progress on a terminal rewrites one line in place. Anywhere else it emits a new
line at each interval, so a log file stays readable.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | A recorded negative outcome: verifier rejected, or merge conflicted |
| 64 | Input failed a grammar, or a pre-condition did not hold |
| 65 | A run record on disk failed schema recognition |
| 70 | An external program failed |
| 127 | A required external program is absent from `PATH` |

Code 1 is not a fault. The command worked and the answer was no.

## `circus prepare`

```
circus prepare --task TASK --integration REF [--attempt N]
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--task` | `task` | — | Required |
| `--integration` | `ref` | — | Required; must resolve to a commit |
| `--attempt` | `attempt-n` | Lowest unused | Must not already exist |

Creates exactly one worktree and one branch. Prints the run record.

The worktree is `<parent-of-checkout>/<repo>-<task>-<attempt>`. The branch is
`circus/<task>/<attempt>`.

Attempt allocation holds the advisory lock, so concurrent preparations of one
task receive distinct numbers.

A Git failure exits 70 and leaves no branch, worktree, or attempt directory
behind.

Contract: CON-001.

## `circus launch`

```
circus launch --attempt ATTEMPT --prompt FILE -- DRIVER [ARGS...]
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--attempt` | `attempt-id` | — | State must be `prepared` |
| `--prompt` | absolute path | — | Must contain the sentinel path literally |
| `DRIVER` | path or `PATH` name | — | Must be executable |

Starts one detached tmux session named `circus-<task>-<attempt>`, running
`withdone` around the driver. Blocks until the attempt reaches a terminal
outcome.

Attach to a running attempt with `tmux attach -t circus-<task>-<attempt>`.

The driver receives these variables:

| Variable | Value |
|---|---|
| `CIRCUS_ATTEMPT` | The `task/attempt` handle |
| `CIRCUS_PROMPT` | A byte-identical copy of the caller's prompt file |
| `CIRCUS_SENTINEL` | The path the worker writes to signal completion |
| `CIRCUS_WORKTREE` | The attempt worktree, also the pane's working directory |

A prompt lacking the sentinel path exits 64 and starts no process. A relaunch
of an attempt that is not `prepared` exits 64.

Exit 1 reports that the process group outlived the 10 s cleanup deadline. The
attempt state becomes `failed`.

Contract: CON-002. Limits: NFR-002.

## `circus accept`

```
circus accept --attempt ATTEMPT --verifier-record FILE --evidence REF...
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--attempt` | `attempt-id` | — | State must be `completed` |
| `--verifier-record` | absolute path | — | Must satisfy the schema below |
| `--evidence` | `evidence-ref` | — | At least one required |

The verifier record:

```json
{
  "command": ["cargo", "test"],
  "exit_code": 0,
  "output_path": "/tmp/verifier.txt"
}
```

`command` has at least one element. `exit_code` is in `0..=255`. `output_path`
is absolute. No other property is accepted.

Records `accepted` only when the exit code is 0 and at least one reference is
present. Records `rejected` otherwise, and exits 1.

The verifier's `output_path` is copied to `<attempt-dir>/verifier.log`, capped
at 1 MiB, and the record names the copy. The caller's file is not modified, and
it is usually the copy that survives — a verifier writing into a temporary
directory leaves nothing behind by the next day. An unreadable source is
recorded as absent and never changes the decision.

An evidence reference is recorded verbatim and never resolved. Circus cannot
tell a real reference from a plausible string; establishing that is the lead's
work, done before acceptance.

Contract: CON-003. Signals: OBS-002.

## `circus spawn`

```
circus spawn --task TASK --integration REF --task-file FILE [--attempt N] -- DRIVER [ARGS...]
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--task` | `task` | — | Required |
| `--integration` | `ref` | — | Required; must resolve to a commit |
| `--task-file` | path or `-` | — | `-` reads the task from stdin |
| `--attempt` | `attempt-n` | Lowest unused | Must not already exist |

`prepare` and `launch` under one command, with the step between them done for
you: the prompt is the task file followed by the attempt's completion
instruction, written to `<attempt-dir>/prompt`.

Your task file is never modified, and the composed prompt is kept where the
rest of the attempt lives, so you can read exactly what the agent was sent.

A launch failure leaves the prepared attempt standing — the worktree, branch,
and record all remain, and Circus never deletes an attempt.

Contract: CON-011.

## `circus history`

```
circus history --task TASK
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--task` | `task` | — | Required |

Writes one entry per attempt of the task to stdout, oldest first: the verdict,
the verifier command and exit code, the captured log, the evidence, what the
branch changed, and the transcript path. A task with no attempts writes nothing
and exits 0.

Unlike every other command, stdout carries prose rather than a record. The
output is prompt material:

```sh
{ cat task.md; circus history --task parser; circus instruction --attempt parser/3; } > prompt.md
```

`changed nothing` is the line worth reading first. An attempt that reported
success while altering no files is the most useful single fact a later attempt
can be given, and no transcript states it as plainly as a diff does.

Contract: CON-012.

## `circus status`

```
circus status [--attempt ATTEMPT]
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--attempt` | `attempt-id` | every attempt | A run record must exist |

Reports each attempt as recorded and as observed right now. With `--attempt`,
stdout carries one object; without it, an array ordered by task then attempt
number. Creates and modifies nothing.

| `live` field | Meaning |
|---|---|
| `pane` | The tmux session name |
| `pane_alive` | Whether tmux still holds it |
| `running_for_seconds` | Seconds since launch, while still running |
| `sentinel_present` | Whether the agent has signalled |
| `worktree_present` | Whether the attempt worktree is on disk |
| `worktree_dirty` | Whether it has uncommitted changes |
| `commits_ahead` | Commits on the task branch over the integration ref |
| `transcript_bytes` | Size of the transcript |

A running attempt's record still reads `prepared`, because the state advances
only when the launch returns. `pane_alive` is the live answer.

Contract: CON-009. Signals: OBS-005.

## `circus logs`

```
circus logs --attempt ATTEMPT [--follow] [--plain]
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--attempt` | `attempt-id` | — | A run record must exist |
| `-f`, `--follow` | flag | off | Keep writing until the attempt stops |
| `--plain` | flag | off | Remove terminal control sequences |

Writes what the agent itself printed. The transcript is a capture of a
terminal, so raw output carries colour and cursor movement; `--plain` removes
them. An attempt that has printed nothing yields empty output and exit 0.

`--follow` returns on its own when the pane goes away, which is what it has
over `tail -f`.

`tmux attach -t circus-<task>-<attempt>` shows the same thing live and
interactively, and is the better choice for a person watching. `circus logs`
is the one that puts bytes on stdout for a program.

Contract: CON-010.

## `circus instruction`

```
circus instruction --attempt ATTEMPT
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--attempt` | `attempt-id` | — | A run record must exist |

Writes the attempt's completion instruction to stdout, containing its sentinel
path verbatim. Creates and modifies nothing.

```sh
{ cat task.md; circus instruction --attempt model/1; } > prompt.md
```

Circus prints the instruction; it never inserts it into a prompt. Assembling
the sentinel path by hand is the one reliable way to get a launch refused — on
macOS `/tmp` is a symlink, and a typed path does not match the canonical one.

Contract: CON-008.

## `circus merge`

```
circus merge --attempt ATTEMPT --into REF [--dry-run]
```

| Argument | Type | Default | Constraint |
|---|---|---|---|
| `--attempt` | `attempt-id` | — | Must be recorded as accepted |
| `--into` | `ref` | — | Must equal the recorded `integration_ref` |
| `-n`, `--dry-run` | flag | off | Report the outcome and change nothing |

Merges the task branch into the integration ref under the advisory lock. The
merge is computed in the object database, and the ref moves by
compare-and-swap.

`--dry-run` runs every pre-condition and the same merge computation, then stops
before applying. No ref moves, no worktree is refreshed, and no run record is
written. A preview of a conflicting merge exits 1, so a script can branch on it.

A worktree holding the ref is brought in line with the merge commit. A worktree
holding the ref with an uncommitted change exits 64 before the ref moves.

A conflict exits 1. The integration ref and the attempt worktree are unchanged.

Circus never rebases, amends, cherry-picks, force-updates, or pushes.

Contract: CON-004.

## What Circus never does

- Create, copy, share, or enrol an Elephant identity.
- Parse, store as structured data, or transmit an Elephant invite secret.
- Invoke the `elephant` executable.
- Delete an attempt worktree, branch, log, or record.
- Substitute an internal implementation for an absent external program.
