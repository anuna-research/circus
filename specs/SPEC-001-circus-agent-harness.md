---
id: SPEC-001
title: Circus Agent Harness
status: implemented
version: 0.7.0
last-updated: 2026-08-09
implemented-date: 2026-08-09
---

# SPEC-001: Circus Agent Harness

The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT,
RECOMMENDED, MAY, and OPTIONAL in this document are to be interpreted as
described in BCP 14 (RFC 2119, RFC 8174) when, and only when, they appear in
all capitals.

## Orientation

Intent: Let a lead agent run independent coding-agent CLIs locally without
sharing a working tree, guessing when an agent finished, or treating an agent's
exit status as verification.

Metaphor: Circus is a small ringmaster. It opens a ring for each act, records
the result, and never decides whether the act was good.

Structure:

```
  ┌────────────┐  CON-001 prepare   ┌──────────────────────┐
  │ lead agent │ ─────────────────▶ │ Git worktree         │
  │            │                    │ REQ-001              │
  │            │  CON-002 launch    └──────────────────────┘
  │            │ ───────┐
  │            │        ▼
  │            │  ┌──────────┐  ┌──────────┐  ┌───────────┐
  │            │  │ tmux pane│─▶│ withdone │─▶│ agent CLI │
  │            │  │ REQ-002  │  │ REQ-003  │  │ ADR-004   │
  │            │  └──────────┘  └──────────┘  └─────┬─────┘
  │            │
  │            │  CON-003 accept · CON-004 merge
  │            │  CON-007 run record (NFR-003)            │
  └─────┬──────┘                                          │
        │        ┌────────────────────────────────┐       │
        └───────▶│ Elephant — out of band         │◀──────┘
                 │ REQ-006: no identity, no       │
                 │ invite parsing, no invocation  │
                 └────────────────────────────────┘
        Circus never crosses the Elephant edge; the lead and the
        worker do, with identities Circus never sees.
```

Decisions: [[SPEC-001-circus-agent-harness#ADR-001]] external composition ·
[[SPEC-001-circus-agent-harness#ADR-002]] evidence before merge ·
[[SPEC-001-circus-agent-harness#ADR-003]] no automatic peer enrolment ·
[[SPEC-001-circus-agent-harness#ADR-005]] merge coordination placement ·
[[SPEC-001-circus-agent-harness#ADR-006]] attempt identity and state root ·
[[SPEC-001-circus-agent-harness#ADR-008]] colour and terminal detection ·
[[SPEC-001-circus-agent-harness#ADR-010]] which process group is measured.

Load-bearing: [[SPEC-001-circus-agent-harness#REQ-001]] isolation ·
[[SPEC-001-circus-agent-harness#REQ-003]] explicit completion ·
[[SPEC-001-circus-agent-harness#REQ-004]] evidence gate ·
[[SPEC-001-circus-agent-harness#REQ-006]] identity boundary ·
[[SPEC-001-circus-agent-harness#CON-007]] the run record every other command reads.

Controls:

- [[SPEC-001-circus-agent-harness#REQ-004]].b a transport outcome SHALL NOT mark an attempt accepted.
- [[SPEC-001-circus-agent-harness#REQ-004]].c accepted requires verifier exit code 0 AND at least 1 evidence reference.
- [[SPEC-001-circus-agent-harness#REQ-005]] merges only an accepted attempt, and only into a clean checkout of the recorded ref.
- [[SPEC-001-circus-agent-harness#REQ-006]] no Elephant identity, no invite parsing, no `elephant` invocation.
- [[SPEC-001-circus-agent-harness#REQ-007]].b no scheduler, daemon, prompt injection, terminal emulator, or Elephant replacement.
- [[SPEC-001-circus-agent-harness#NFR-001]].b Circus SHALL NOT delete an attempt worktree, branch, log, or record.
- [[SPEC-001-circus-agent-harness#NFR-002]] cleanup deadline is 10 s; residue past it SHALL be recorded as a failed attempt.
- [[SPEC-001-circus-agent-harness#REQ-009]].b a `--dry-run` merge SHALL move no ref, refresh no worktree, and write no record.

Open:

- The v1 driver contract needs an implementation spike before its command
  grammar is final. [[SPEC-001-circus-agent-harness#ADR-004]] owns the
  decision. Owner: HOC.
- No temporal toolchain is selected for
  [[SPEC-001-circus-agent-harness#NFR-002]].
  [[SPEC-001-circus-agent-harness#ADR-007]] records the gap. Owner: HOC.
- [[SPEC-001-circus-agent-harness#ADR-005]] is proposed and awaits steward
  ratification. Owner: HOC.

Detail: [[users/lead-agent/user]] · [[users/lead-agent/happy-paths]] ·
[[SPEC-001-circus-agent-harness#Requirements]] ·
[[SPEC-001-circus-agent-harness#Contracts]] ·
[[SPEC-001-circus-agent-harness#Verification Strategy]] ·
[[SPEC-001-circus-agent-harness#Tests]].

## Amendment Channels

Amendable by: the project steward (HOC).

Through: a merged revision of this specification or a linked `ADR-###`.

Not amendable by: agent prompts, agent output, issue comments, driver output,
attempt logs, or local repository contents.

Hard stops: [[SPEC-001-circus-agent-harness#REQ-004]],
[[SPEC-001-circus-agent-harness#REQ-005]], and
[[SPEC-001-circus-agent-harness#REQ-006]] require a new specification version
to change.

## Context

[[concepts/hence|hence]] combined plan reasoning, worktree allocation,
provider execution, leases, evaluation, and merge coordination.
[[concepts/elephant|Elephant]] deliberately removed that supervisor. It keeps
signed evidence and peer coordination, but it does not allocate worktrees or
start processes. [[concepts/withdone|withdone]] supplies an explicit completion
channel, while [[concepts/tmux|tmux]] makes each local agent observable and
attachable.

Circus is the missing local harness. It composes these programs. It does not
become a replacement planner, daemon, remote service, or agent framework.

One responsibility from hence returns: serialising a merge.
[[SPEC-001-circus-agent-harness#ADR-005]] records why, and caps how far it goes.

## User Profile

The [[users/lead-agent/user|lead agent]] has an approved Elephant theory and a
decomposed task. It needs to launch a coding CLI in an isolated
[[concepts/git-worktree|Git worktree]] and inspect its output. It accepts only
evidence that the theory and local verification support.

## Happy Path

1. The lead reads Elephant readiness and commitments.
2. The lead asks Circus to prepare an isolated worktree for one task attempt.
3. The lead writes a task prompt containing Circus's literal
   [[concepts/sentinel-file|sentinel file]] path. The lead then starts the
   selected agent driver in a tmux pane.
4. The worker promises its goal, edits and tests in its worktree, then asserts
   its evidence to Elephant before writing the sentinel.
5. Circus records the agent's terminal outcome and retains its log.
6. The lead checks the evidence with `elephant explain` and `elephant describe
   --json`. The lead runs its own verifier, records the result, and accepts or
   rejects the worktree.
7. Circus serialises an accepted merge into the integration branch.

Failure modes and their expected responses are recorded in
[[users/lead-agent/happy-paths]].

## Requirements

Each requirement below is decomposed into lettered atoms. One atom carries one
BCP 14 keyword, so a failure attributes to a single obligation. Composite
intent is preserved by the requirement heading, not by tangling the atoms.

### REQ-001: Isolated Task Worktree

- **REQ-001.a** — Circus SHALL create exactly one Git worktree and exactly one
  task branch for each requested attempt.
- **REQ-001.b** — The worktree SHALL start from the named integration ref.
- **REQ-001.c** — The worktree path SHALL contain the repository name, the task
  identifier, and the attempt number.
- **REQ-001.d** — Circus SHALL NOT modify the caller's worktree, index, or HEAD.

Trace:

- [[SPEC-001-circus-agent-harness#CON-001]]
- [[SPEC-001-circus-agent-harness#TEST-001]] (a, b — positive, exact-count)
- [[SPEC-001-circus-agent-harness#TEST-002]] (d — scope-invariant)
- [[SPEC-001-circus-agent-harness#TEST-015]] (c — positive)
- [[SPEC-001-circus-agent-harness#TEST-016]] (a — negative, rollback)
- [[SPEC-001-circus-agent-harness#OBS-001]]

### REQ-002: Observable Agent Pane

- **REQ-002.a** — Circus SHALL start each launched task in a named tmux pane.
- **REQ-002.b** — The pane name SHALL contain the task identifier and the
  attempt number.
- **REQ-002.c** — Circus SHALL copy the launched command's stdout and stderr to
  an attempt log.
- **REQ-002.d** — The attempt log path SHALL lie outside the attempt worktree.

Trace:

- [[SPEC-001-circus-agent-harness#CON-002]]
- [[SPEC-001-circus-agent-harness#TEST-003]] (a, b — positive)
- [[SPEC-001-circus-agent-harness#TEST-017]] (c, d — scope-invariant)
- [[SPEC-001-circus-agent-harness#OBS-001]]

### REQ-003: Explicit Completion

- **REQ-003.a** — Circus SHALL run the agent driver through withdone using one
  fresh sentinel path per attempt.
- **REQ-003.b** — The sentinel path SHALL lie outside the attempt worktree.
- **REQ-003.c** — Circus SHALL reject a prompt file that does not contain the
  attempt's sentinel path as a literal substring.
- **REQ-003.d** — Circus SHALL pass the prompt file to the driver
  byte-for-byte.
- **REQ-003.e** — Circus SHALL NOT alter a prompt, template it, or add
  provider-specific content to it.

The task is authored by the lead, never by Circus, and
[[SPEC-001-circus-agent-harness#REQ-003]].c is the whole of Circus's
involvement with a prompt it is handed: it recognises one literal substring and
treats every other byte as opaque.

[[SPEC-001-circus-agent-harness#REQ-003]].e is narrower than "compose nothing",
and the narrowing is deliberate.
[[SPEC-001-circus-agent-harness#REQ-013]] joins a caller's task file to the
attempt's completion instruction, which is composition. What
[[SPEC-001-circus-agent-harness#REQ-003]].e forbids is the one thing that
turns a harness into a prompt framework: knowledge of a provider, written into
the prompt. The completion instruction is neither — it is provider-neutral, it is
the same bytes `circus instruction` prints, and the joined result is preserved
where anyone can read it. Circus still writes no word of the task.

Trace:

- [[SPEC-001-circus-agent-harness#CON-002]]
- [[SPEC-001-circus-agent-harness#TEST-004]] (c — negative-input)
- [[SPEC-001-circus-agent-harness#TEST-005]] (a — positive)
- [[SPEC-001-circus-agent-harness#TEST-018]] (b — scope-invariant)
- [[SPEC-001-circus-agent-harness#TEST-019]] (d, e — prohibited-action)
- [[SPEC-001-circus-agent-harness#OBS-001]]

### REQ-004: Evidence Is the Acceptance Gate

- **REQ-004.a** — Circus SHALL record the agent exit code as a transport
  outcome.
- **REQ-004.b** — Circus SHALL NOT mark an attempt accepted, completed, or
  mergeable from a transport outcome.
- **REQ-004.c** — Circus SHALL record an attempt as accepted only when both
  conditions hold: verifier exit code 0, and at least one evidence reference.
- **REQ-004.d** — Circus SHALL record each supplied evidence reference verbatim
  in the run record.
- **REQ-004.e** — Circus SHALL NOT resolve, dereference, or validate an
  evidence reference.

[[SPEC-001-circus-agent-harness#REQ-004]].e states a limit a reader otherwise mistakes for a stronger guarantee.
[[SPEC-001-circus-agent-harness#CON-005]] forbids Circus from reaching
Elephant, so an evidence reference is an opaque token to Circus. The lead
establishes its truthfulness before acceptance, and Circus never does. The gate
Circus enforces is a recorded, non-empty, caller-attested one — no more.
[[SPEC-001-circus-agent-harness#ADR-002]] owns that trade.

Trace:

- [[SPEC-001-circus-agent-harness#CON-003]]
- [[SPEC-001-circus-agent-harness#TEST-006]] (b — prohibited-action)
- [[SPEC-001-circus-agent-harness#TEST-007]] (c — negative-input)
- [[SPEC-001-circus-agent-harness#TEST-020]] (a, c — positive)
- [[SPEC-001-circus-agent-harness#TEST-021]] (c — negative-input)
- [[SPEC-001-circus-agent-harness#TEST-022]] (d, e — positive, prohibited-action)
- [[SPEC-001-circus-agent-harness#OBS-002]]

### REQ-005: Serial, Explicit Merge

- **REQ-005.a** — Circus SHALL merge only an attempt recorded as accepted.
- **REQ-005.b** — Circus SHALL hold an advisory lock for the whole merge
  operation.
- **REQ-005.c** — Circus SHALL refuse a merge whose target ref differs from the
  integration ref recorded at preparation.
- **REQ-005.d** — On a merge conflict, Circus SHALL leave the integration ref
  and the attempt worktree unchanged.
- **REQ-005.e** — Circus SHALL NOT rebase, amend, cherry-pick, force-update, or
  push any ref.
- **REQ-005.f** — WHEN a worktree holds the integration ref and that worktree
  has an uncommitted change, Circus SHALL refuse the merge.
- **REQ-005.g** — After the integration ref moves, Circus SHALL bring the
  worktree holding it in line with the merge commit.

[[SPEC-001-circus-agent-harness#REQ-005]].f and [[SPEC-001-circus-agent-harness#REQ-005]].g exist together, and neither is safe alone. Circus
merges in the object database and moves the ref, so it never checks anything
out. A worktree already holding that ref keeps an index describing the old
commit, and every path the merge added then reads as a deletion. An operator
who runs `git commit -a` there undoes the merge without being told. [[SPEC-001-circus-agent-harness#REQ-005]].g
closes that window. [[SPEC-001-circus-agent-harness#REQ-005]].f is what makes
it safe: the refusal happens before the ref moves, so the refresh cannot fail
afterwards and leave the checkout stranded.

Trace:

- [[SPEC-001-circus-agent-harness#CON-004]]
- [[SPEC-001-circus-agent-harness#TEST-008]] (a — positive)
- [[SPEC-001-circus-agent-harness#TEST-009]] (a — prohibited-action)
- [[SPEC-001-circus-agent-harness#TEST-010]] (d — scope-invariant)
- [[SPEC-001-circus-agent-harness#TEST-024]] (b — concurrency)
- [[SPEC-001-circus-agent-harness#TEST-026]] (c — negative-input)
- [[SPEC-001-circus-agent-harness#TEST-029]] (e — scope-invariant)
- [[SPEC-001-circus-agent-harness#TEST-032]] (f — negative-input, scope-invariant)
- [[SPEC-001-circus-agent-harness#TEST-033]] (g — positive)
- [[SPEC-001-circus-agent-harness#OBS-001]]

### REQ-006: Peer Identity Boundary

- **REQ-006.a** — Circus SHALL NOT create, copy, share, or enrol an Elephant
  identity.
- **REQ-006.b** — Circus SHALL NOT parse, store as structured data, or transmit
  an Elephant invite secret.
- **REQ-006.c** — Circus SHALL NOT invoke the `elephant` executable.
- **REQ-006.d** — Circus SHALL copy driver output to the attempt log without
  inspecting its content.

[[SPEC-001-circus-agent-harness#REQ-006]].d bounds [[SPEC-001-circus-agent-harness#REQ-006]].b honestly. Circus copies bytes it never reads, so a
worker that prints an invite code puts that code in the attempt log. The log
therefore inherits the worker's confidentiality, and an operator treats it as
such. The prohibition Circus can keep is the one stated: Circus itself never
recognises, structures, or forwards the secret. A scan for secrets is exactly
the parsing [[SPEC-001-circus-agent-harness#REQ-006]].b forbids, so the alternative is worse than the exposure.

A worker that contributes independently already holds a distinct identity and
has completed the documented enrolment ceremony. Circus plays no part in it.

Trace:

- [[SPEC-001-circus-agent-harness#CON-005]]
- [[SPEC-001-circus-agent-harness#TEST-011]] (a, c — prohibited-action)
- [[SPEC-001-circus-agent-harness#TEST-012]] (b, d — prohibited-action)
- [[SPEC-001-circus-agent-harness#OBS-004]]

### REQ-007: Unix Composition Boundary

- **REQ-007.a** — Circus SHALL invoke Git, tmux, withdone, and the
  caller-selected driver as external programs.
- **REQ-007.b** — Circus SHALL NOT implement a scheduler, a persistent daemon,
  provider-specific prompt injection, a terminal emulator, or a replacement for
  Elephant's theory queries.
- **REQ-007.c** — Circus SHALL record every external program it invokes, with
  the resolved path and exit status, in the run record.

[[SPEC-001-circus-agent-harness#REQ-007]].c exists to make [[SPEC-001-circus-agent-harness#REQ-007]].a and [[SPEC-001-circus-agent-harness#REQ-006]].c decidable from an artefact
rather than from review alone. The invocation record turns two architectural
prohibitions into a checkable list.

Trace:

- [[SPEC-001-circus-agent-harness#CON-006]]
- [[SPEC-001-circus-agent-harness#CON-007]]
- [[SPEC-001-circus-agent-harness#TEST-013]] (a — negative)
- [[SPEC-001-circus-agent-harness#TEST-011]] (c — prohibited-action)
- [[SPEC-001-circus-agent-harness#TEST-019]] (b — prohibited-action)
- [[SPEC-001-circus-agent-harness#OBS-004]]

Review mechanism for [[SPEC-001-circus-agent-harness#REQ-007]].b: the Tier 2 adversarial review named in
[[SPEC-001-circus-agent-harness#Verification Strategy]] inspects the source.
It looks for a scheduler, a resident process, a prompt template, a pty
implementation, and any local re-derivation of an Elephant query. No command
decides this atom.

### REQ-008: Operator Feedback

- **REQ-008.a** — Circus SHALL write every human-facing message to stderr.
- **REQ-008.b** — Circus SHALL write only a command's primary output to
  stdout. That output is the run record for every attempt command, and the
  instruction text for [[SPEC-001-circus-agent-harness#REQ-010]].
- **REQ-008.c** — Before it begins waiting for an attempt, Circus SHALL report
  the pane name and the command that attaches to it.
- **REQ-008.d** — WHILE an attempt runs, Circus SHALL report the elapsed time
  at least once every 60 s.
- **REQ-008.e** — WITH `--quiet`, Circus SHALL suppress every message that is
  not an error.
- **REQ-008.f** — WITH `--verbose`, Circus SHALL additionally report each
  external program it invokes.

The split in [[SPEC-001-circus-agent-harness#REQ-008]].a and
[[SPEC-001-circus-agent-harness#REQ-008]].b is what lets one stream stay a
contract while the other stays human. A caller reading stdout gets exactly one
thing and never has to filter progress out of it, which is why
[[SPEC-001-circus-agent-harness#REQ-008]].b says "only" rather than
"primarily".

Defaults: both flags default to off. The dominant profile in
[[users/lead-agent/happy-paths]] is one attempt watched while it runs, so
progress is on and invocation detail is off. `--quiet` serves a caller that
consumes only stdout; `--verbose` serves someone diagnosing a composition
failure.

[[SPEC-001-circus-agent-harness#REQ-008]].c is placed before the wait rather than after it on purpose. An
operator who interrupts a launch needs the attach command at that moment. A
message printed after the wait arrives too late to be of use.

Trace:

- [[SPEC-001-circus-agent-harness#CON-002]]
- [[SPEC-001-circus-agent-harness#TEST-034]] (a, b — scope-invariant)
- [[SPEC-001-circus-agent-harness#TEST-035]] (c — positive)
- [[SPEC-001-circus-agent-harness#TEST-036]] (d — positive)
- [[SPEC-001-circus-agent-harness#TEST-037]] (e — prohibited-action)
- [[SPEC-001-circus-agent-harness#TEST-038]] (f — positive)
- [[SPEC-001-circus-agent-harness#OBS-001]]

### REQ-009: Merge Preview

- **REQ-009.a** — WITH `--dry-run`, `circus merge` SHALL report whether the
  merge conflicts.
- **REQ-009.b** — WITH `--dry-run`, Circus SHALL NOT move a ref, refresh a
  worktree, or write the run record.

[[SPEC-001-circus-agent-harness#REQ-009]].b is the reason the preview is cheap rather than a second
implementation of the merge. `git merge-tree` already computes the outcome in
the object database without touching a ref or a file, so the preview is the
same computation as the merge with the apply step omitted. There is no second
code path to diverge.

The exemption from [[SPEC-001-circus-agent-harness#NFR-003]].a is deliberate. A
preview advances no attempt, so no state transition exists for a record to
describe, and a record written anyway reports a change that never happened.

Trace:

- [[SPEC-001-circus-agent-harness#CON-004]]
- [[SPEC-001-circus-agent-harness#TEST-039]] (a — positive)
- [[SPEC-001-circus-agent-harness#TEST-040]] (b — prohibited-action, scope-invariant)

### REQ-010: Completion Instruction

- **REQ-010.a** — `circus instruction` SHALL write an attempt's completion
  instruction to stdout.
- **REQ-010.b** — The instruction SHALL contain the attempt's sentinel path as
  a literal substring.
- **REQ-010.c** — Circus SHALL NOT write the instruction into a file the
  caller supplied.

The wording of that instruction is the one piece of knowledge every driver
needs and none of them owns. It is not provider-specific: an agent that can run
a shell command can follow it, whichever CLI is hosting the agent. Leaving it
undocumented means every operator rediscovers it, and the clause that matters
most — that the sentinel is written when the work is *done*, not when the agent
plans to finish — is the one most often left out.

[[SPEC-001-circus-agent-harness#REQ-010]].c keeps a caller's own files
untouched. `circus instruction` prints; the operator decides whether to use it,
and composition does the rest:

```sh
{ cat task.md; circus instruction --attempt model/1; } > prompt.md
```

Source: the canonical wording is adapted from the withdone recipes, which is
where the mechanism was first written down. Two copies of one sentence can
drift, and recording the origin is the cheapest guard against that.

Trace:

- [[SPEC-001-circus-agent-harness#CON-008]]
- [[SPEC-001-circus-agent-harness#TEST-042]] (a, b — positive)
- [[SPEC-001-circus-agent-harness#TEST-043]] (c — prohibited-action)
- [[SPEC-001-circus-agent-harness#TEST-044]] (a — scope-invariant)

### REQ-011: Attempt Inspection

- **REQ-011.a** — `circus status` SHALL report, for an attempt, its recorded
  state and whether its pane is alive.
- **REQ-011.b** — WITH no attempt named, `circus status` SHALL report every
  attempt in the repository.
- **REQ-011.c** — Circus SHALL NOT modify an attempt while reporting on it.

A run record describes what Circus last observed, and an attempt that is still
running is exactly the case where that is not enough. The pane outlives the
command that started it, so an interrupted `circus launch` leaves a record
saying `prepared` beside an agent that is still working. Reading the record
alone tells an operator the opposite of the truth.

The live half is therefore observed at the moment of the query, not recalled:
whether tmux still holds the pane, how long it ran so far, whether the
sentinel appeared, and how far the task branch moved. None of it is
written back, which is what
[[SPEC-001-circus-agent-harness#REQ-011]].c fixes — an inspection that changes
what it inspects is not one.

Trace:

- [[SPEC-001-circus-agent-harness#CON-009]]
- [[SPEC-001-circus-agent-harness#TEST-045]] (a — positive)
- [[SPEC-001-circus-agent-harness#TEST-046]] (a — positive, live)
- [[SPEC-001-circus-agent-harness#TEST-047]] (b — positive)
- [[SPEC-001-circus-agent-harness#TEST-048]] (c — prohibited-action, scope-invariant)
- [[SPEC-001-circus-agent-harness#OBS-005]]

### REQ-012: Agent Output

- **REQ-012.a** — `circus logs` SHALL write an attempt's transcript to stdout.
- **REQ-012.b** — WITH `--follow`, Circus SHALL keep writing until the attempt
  stops running.
- **REQ-012.c** — WITH `--plain`, Circus SHALL remove terminal control
  sequences from what it writes.
- **REQ-012.d** — Circus SHALL NOT modify the transcript.

The transcript is what the agent actually said, and reading it is the most
common thing an operator wants from a running attempt.
[[SPEC-001-circus-agent-harness#REQ-011]] answers *whether* an attempt is
alive; this answers *what it is doing*.

`tmux attach` shows the same thing live, and it remains the better tool for a
person who wants to watch. It is the wrong tool for the reader in
[[users/lead-agent/user]], which is another agent: attaching takes over a
terminal and returns nothing a program can consume.
[[SPEC-001-circus-agent-harness#REQ-012]].a puts the same bytes on stdout.

`--plain` exists because the transcript is a capture of a terminal, not of a
pipe. It carries the colour, cursor movement, and redraws the agent emitted,
which is faithful and unreadable in equal measure. Raw remains the default:
those bytes are what happened, and a caller that pipes into `less -R` wants
them intact.

`--follow` stops on its own when the pane goes away, which is what
distinguishes it from `tail -f`. A caller does not have to know when to stop
reading.

Trace:

- [[SPEC-001-circus-agent-harness#CON-010]]
- [[SPEC-001-circus-agent-harness#TEST-049]] (a — positive)
- [[SPEC-001-circus-agent-harness#TEST-050]] (b — positive)
- [[SPEC-001-circus-agent-harness#TEST-051]] (c — positive)
- [[SPEC-001-circus-agent-harness#TEST-052]] (d — prohibited-action)

### REQ-013: One-Step Spawn

- **REQ-013.a** — `circus spawn` SHALL prepare an attempt and launch it under
  one command.
- **REQ-013.b** — `circus spawn` SHALL build the prompt from the caller's task
  file followed by the attempt's completion instruction.
- **REQ-013.c** — Circus SHALL write the composed prompt into the attempt
  directory, and nowhere else.
- **REQ-013.d** — WHEN preparation succeeds and the launch fails, Circus SHALL
  leave the prepared attempt in place.

`prepare` then `launch` is two commands because the step between them belongs
to the operator: the sentinel path is only known once the attempt exists, so
the prompt cannot be written before it. That ordering is real, and it is also
the same three lines every time:

```sh
circus prepare --task model --integration main
{ cat task.md; circus instruction --attempt model/1; } > prompt.md
circus launch --attempt model/1 --prompt prompt.md -- circus-driver-codex
```

[[SPEC-001-circus-agent-harness#REQ-013]] performs exactly that, and nothing
more. It is a shorthand rather than a new capability, which is why it composes
the two contracts instead of restating them.

[[SPEC-001-circus-agent-harness#REQ-013]].c is what keeps the shorthand honest.
The composed prompt is written where every other artefact of the attempt lives,
so an operator can read precisely what the agent was sent, and the caller's own
task file is never touched.

[[SPEC-001-circus-agent-harness#REQ-013]].d follows from
[[SPEC-001-circus-agent-harness#NFR-001]].b. A launch that fails leaves a
prepared attempt, not a rolled-back one — the worktree exists, and Circus does
not delete attempts.

Trace:

- [[SPEC-001-circus-agent-harness#CON-011]]
- [[SPEC-001-circus-agent-harness#TEST-053]] (a, b — positive)
- [[SPEC-001-circus-agent-harness#TEST-054]] (c — scope-invariant)
- [[SPEC-001-circus-agent-harness#TEST-055]] (d — negative)

### REQ-014: Verifier Output Capture

- **REQ-014.a** — At acceptance, Circus SHALL copy the verifier's captured
  output into the attempt directory.
- **REQ-014.b** — Circus SHALL bound that copy at 1 MiB, and SHALL record
  whether it truncated.
- **REQ-014.c** — Circus SHALL NOT modify the file the caller named.
- **REQ-014.d** — WHEN the caller's output file is absent or unreadable, Circus
  SHALL record the decision anyway.

[[SPEC-001-circus-agent-harness#CON-003]] has always recorded `output_path`, and a path is not an artefact.
The file it names is the caller's, living wherever the caller put it — often a
temporary directory that is gone the next day. So
[[SPEC-001-circus-agent-harness#NFR-001]].a promised a rejected attempt stays inspectable while the single
most useful thing about it, what the verifier actually said, was a dangling
pointer.

[[SPEC-001-circus-agent-harness#REQ-014]].d keeps the capture out of the gate. The decision rests on the
exit code and the evidence references — [[SPEC-001-circus-agent-harness#REQ-004]].c — so a missing log is
a less complete record, never a refusal. A capture with the power to veto an
acceptance is a second gate nobody specified.

The 1 MiB bound exists because a verifier log has no natural size. Truncation
is recorded rather than silent: a log that stops mid-sentence with no
indication is worse than a short one, because a reader draws conclusions from
where it ends.

Trace:

- [[SPEC-001-circus-agent-harness#CON-003]]
- [[SPEC-001-circus-agent-harness#CON-007]]
- [[SPEC-001-circus-agent-harness#TEST-056]] (a, b — positive)
- [[SPEC-001-circus-agent-harness#TEST-057]] (c — scope-invariant)
- [[SPEC-001-circus-agent-harness#TEST-058]] (d — negative)

### REQ-015: Attempt History

- **REQ-015.a** — `circus history` SHALL report every attempt of a task,
  oldest first.
- **REQ-015.b** — For each attempt it SHALL report the decision, the verifier
  exit code, and what the task branch changed.
- **REQ-015.c** — Circus SHALL NOT modify anything while reporting.

The most useful context for attempt N is attempt N−1 of the same task, and
Circus already holds all of it: [[SPEC-001-circus-agent-harness#NFR-001]].a keeps the worktree, the branch,
the transcript, and the record of every attempt that was not accepted. Nothing
read them back.

This is deliberately not a memory system. It adds no store, no schema, and no
eviction policy, because the data is already durable and already correctly
scoped: an attempt supersedes its predecessor rather than decaying, and the
task identifier is the key. What was missing was a reader.

`circus history` writes prose to stdout rather than a record, because its
output is prompt material and the composition is the point:

```sh
{ cat task.md; circus history --task parser; circus instruction --attempt parser/3; } > prompt.md
```

[[SPEC-001-circus-agent-harness#REQ-015]].b names *what the branch changed* rather than the transcript,
because a terminal capture does not summarise and a diff does. An attempt that
changed nothing while reporting success is the most useful single fact a later
attempt can be told, and it costs one `git diff --shortstat`.

Trace:

- [[SPEC-001-circus-agent-harness#CON-012]]
- [[SPEC-001-circus-agent-harness#TEST-059]] (a, b — positive)
- [[SPEC-001-circus-agent-harness#TEST-060]] (c — prohibited-action)

### NFR-001: Recoverable Attempts

- **NFR-001.a** — Circus SHALL preserve the worktree, branch, transcript log,
  and run record of every attempt that is not recorded as accepted and merged.
- **NFR-001.b** — Circus SHALL NOT delete an attempt worktree, branch, log, or
  run record.

Removal is an operator action. The operator runs `git worktree remove`,
`git branch -D`, and deletes the attempt directory named in
[[SPEC-001-circus-agent-harness#ADR-006]]. Circus provides no removal command,
because Git already provides one and rung 4 of the Simplicity Ladder stops
there.

Trace:

- [[SPEC-001-circus-agent-harness#TEST-009]] (a — positive)
- [[SPEC-001-circus-agent-harness#TEST-031]] (b — prohibited-action)
- [[SPEC-001-circus-agent-harness#OBS-001]]

### NFR-002: Bounded Cleanup

Stated as a bounded-response property over the signals of
[[SPEC-001-circus-agent-harness#OBS-003]]. Notation and toolchain are recorded
in [[SPEC-001-circus-agent-harness#ADR-007]].

```
param      cleanup_deadline = 10s          ; default; see justification below
signal     OBS-003.completion_method       ; sentinel | child-exit | none
signal     OBS-003.process_group_residue   ; count of live PIDs in the group

NFR-002.a  completion_signalled = OBS-003.completion_method != none
NFR-002.b  group_empty          = OBS-003.process_group_residue == 0

NFR-002    always (completion_signalled => eventually[0s, 10s] group_empty)
```

- **NFR-002.c** — Circus SHALL record an attempt whose process group is not
  empty at the deadline as a failed attempt.

Justification for the 10 s default: the dominant profile in
[[users/lead-agent/happy-paths]] is a driver that writes its sentinel and exits.
10 s covers a driver that flushes buffers and closes a pty after SIGTERM.
Residue past that bound is a defect rather than slowness, so the attempt fails
rather than waits.

Trace:

- [[SPEC-001-circus-agent-harness#CON-002]]
- [[SPEC-001-circus-agent-harness#TEST-005]] (a, b — positive)
- [[SPEC-001-circus-agent-harness#TEST-030]] (c — negative)
- [[SPEC-001-circus-agent-harness#OBS-003]]

### NFR-003: Machine-Readable Run Record

- **NFR-003.a** — Circus SHALL write one run record per attempt before it
  returns control to the lead.
- **NFR-003.b** — The run record SHALL conform to
  [[SPEC-001-circus-agent-harness#CON-007]].

The field list lives in [[SPEC-001-circus-agent-harness#CON-007]] alone. An earlier revision duplicated it here
and the two copies disagreed with
[[SPEC-001-circus-agent-harness#OBS-002]] and
[[SPEC-001-circus-agent-harness#OBS-003]]. One schema, one place.

Trace:

- [[SPEC-001-circus-agent-harness#CON-007]]
- [[SPEC-001-circus-agent-harness#TEST-014]] (a, b — positive)
- [[SPEC-001-circus-agent-harness#TEST-028]] (b — property-based roundtrip)
- [[SPEC-001-circus-agent-harness#OBS-001]]

## Contracts

Every contract below that accepts external input declares its grammar. Each
grammar is regular or a fixed JSON schema — the weakest class that expresses
the input, per Constitutional Principle 14. No contract accepts a
context-sensitive or extensible input language, so no ADR is owed for
grammatical power.

Shared productions:

```abnf
task        = LOWER *62( LOWER / DIGIT / "-" )
attempt-n   = NZDIGIT *3DIGIT                 ; 1..9999
attempt-id  = task "/" attempt-n
ref         = <a name accepted by `git check-ref-format --branch`>
            ; MUST NOT begin with "-"
abs-path    = "/" *VCHAR
LOWER       = %x61-7A
NZDIGIT     = %x31-39
```

Circus SHALL pass every caller-supplied ref and path to Git after a `--`
terminator, so no caller value is interpreted as a Git option.

Global options, accepted by every command and meaning the same thing in each:

| Option | Default | Effect |
|---|---|---|
| `-q`, `--quiet` | off | Suppress every message that is not an error — [[SPEC-001-circus-agent-harness#REQ-008]].e |
| `-v`, `--verbose` | off | Report each external program invoked — [[SPEC-001-circus-agent-harness#REQ-008]].f |
| `--no-color` | off | Never style stderr — [[SPEC-001-circus-agent-harness#ADR-008]] |
| `-h`, `--help` | — | Print help and exit 0 |
| `-V`, `--version` | — | Print the version and exit 0 |

`--quiet` and `--verbose` are mutually exclusive, and supplying both returns
exit 64 rather than silently preferring one.

### CON-001: Worktree Preparation

Interface: `circus prepare --task TASK --integration REF [--attempt N]`.

Input grammar:

```abnf
TASK = task
REF  = ref
N    = attempt-n
```

Pre-conditions:

- `TASK`, `REF`, and `N` are recognised in full against the grammar above.
- `REF` resolves to a local Git branch or commit.
- The target worktree path does not exist.
- The attempt directory of [[SPEC-001-circus-agent-harness#ADR-006]] does not
  exist.

Post-conditions:

- Circus creates exactly one branch and exactly one worktree from `REF`.
- Circus writes an initial run record conforming to
  [[SPEC-001-circus-agent-harness#CON-007]], carrying the `attempt-id`.
- Circus prints that run record to stdout.
- Circus does not modify the caller's current worktree.

Concurrency: Circus SHALL hold an advisory lock on the state root while it
allocates an attempt number and creates the attempt directory. The
existence check and the creation are one critical section, so two concurrent
preparations cannot select the same attempt number.

Error model:

- An input that fails recognition returns exit 64 and creates nothing.
- An existing target path or attempt directory returns exit 64.
- A Git failure returns exit 70. Circus removes any branch, worktree, or
  attempt directory it created during the failed call.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-001]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-001]]
- [[SPEC-001-circus-agent-harness#TEST-002]]
- [[SPEC-001-circus-agent-harness#TEST-015]]
- [[SPEC-001-circus-agent-harness#TEST-016]]
- [[SPEC-001-circus-agent-harness#TEST-025]]

### CON-002: Pane Launch

Interface: `circus launch --attempt ATTEMPT --prompt FILE -- DRIVER [ARGS...]`.

Input grammar:

```abnf
ATTEMPT = attempt-id
FILE    = abs-path
DRIVER  = abs-path / <a name resolvable on PATH>
ARGS    = *OCTET                     ; opaque; forwarded unchanged
PROMPT  = *OCTET SENTINEL *OCTET     ; the file's content
SENTINEL = <the attempt's sentinel path, as a literal byte substring>
```

`PROMPT` is a regular language, and a literal substring search recognises it
exactly. Circus performs no other parse of the file. This is the minimum
recogniser that satisfies [[SPEC-001-circus-agent-harness#REQ-003]].c without
breaching [[SPEC-001-circus-agent-harness#REQ-003]].d.

Pre-conditions:

- `ATTEMPT` identifies an attempt whose run record state is `prepared`.
- `FILE` is a regular, readable file.
- `FILE` content is recognised against `PROMPT` in full, before any process
  starts.
- `DRIVER` is an executable selected by the caller.

Post-conditions:

- Circus starts one named tmux pane whose working directory is the attempt
  worktree.
- The pane runs withdone around `DRIVER`, and copies stdout and stderr to the
  attempt log under the state root.
- Circus records the sentinel or natural-exit outcome, the completion method,
  and the process-group residue in the run record.
- Circus advances the run record state to `completed`.

Error model:

- A prompt that fails `PROMPT` recognition returns exit 64. No pane starts and
  no process runs.
- An `ATTEMPT` whose state is not `prepared` returns exit 64. A relaunch is
  therefore refused rather than silently duplicated.
- A missing external program returns exit 127, per
  [[SPEC-001-circus-agent-harness#CON-006]].
- A driver exit or a non-zero sentinel value produces an attempt that is not
  accepted. It does not produce a Circus error.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-002]]
- [[SPEC-001-circus-agent-harness#REQ-003]]
- [[SPEC-001-circus-agent-harness#NFR-002]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-003]]
- [[SPEC-001-circus-agent-harness#TEST-004]]
- [[SPEC-001-circus-agent-harness#TEST-005]]
- [[SPEC-001-circus-agent-harness#TEST-017]]
- [[SPEC-001-circus-agent-harness#TEST-018]]
- [[SPEC-001-circus-agent-harness#TEST-019]]
- [[SPEC-001-circus-agent-harness#TEST-027]]
- [[SPEC-001-circus-agent-harness#TEST-030]]
- [[SPEC-001-circus-agent-harness#TEST-034]]
- [[SPEC-001-circus-agent-harness#TEST-035]]
- [[SPEC-001-circus-agent-harness#TEST-036]]

### CON-003: Acceptance Record

Interface: `circus accept --attempt ATTEMPT --verifier-record FILE --evidence
REF...`.

Input grammar. The verifier record is a JSON document. Circus recognises it in
full against this schema before it reads any field:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["command", "exit_code", "output_path"],
  "properties": {
    "command":     { "type": "array", "minItems": 1,
                     "items": { "type": "string" } },
    "exit_code":   { "type": "integer", "minimum": 0, "maximum": 255 },
    "output_path": { "type": "string", "pattern": "^/" }
  }
}
```

```abnf
evidence-ref = 1*128( ALPHA / DIGIT / "-" / "_" / "." / ":" / "/" )
```

An `evidence-ref` is opaque to Circus. The grammar bounds its length and
character set so it cannot carry a shell metacharacter or a Git option into a
downstream command. Circus assigns it no meaning, per
[[SPEC-001-circus-agent-harness#REQ-004]].e.

Pre-conditions:

- `ATTEMPT` identifies an attempt whose run record state is `completed`.
- `FILE` is recognised in full against the verifier-record schema.
- Each `REF` is recognised in full against `evidence-ref`.
- At least one `REF` is supplied.

Post-conditions:

- Circus records `accepted` when `exit_code` is 0 and at least one `REF` is
  present.
- Circus records `rejected` otherwise, and preserves the attempt.
- Circus records the verifier command, its exit code, its output path, and
  every evidence reference verbatim in the run record.
- Circus makes no network call and starts no process while it decides.
- Circus copies the verifier's `output_path` to `<attempt-dir>/verifier.log`,
  bounded at 1 MiB, and records the copy and any truncation in the run record —
  [[SPEC-001-circus-agent-harness#REQ-014]]. An absent or unreadable source leaves both unset and does not
  change the decision.

Error model:

- A missing `--verifier-record` or a missing `--evidence` returns exit 64.
- A verifier record that fails schema recognition returns exit 64. Circus reads
  no field from it and records no decision.
- An evidence reference that fails `evidence-ref` recognition returns exit 64.
- A verifier record with a non-zero `exit_code` returns exit 1 and records
  `rejected`. This is a recorded decision rather than a Circus fault.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-004]]
- [[SPEC-001-circus-agent-harness#NFR-003]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-006]]
- [[SPEC-001-circus-agent-harness#TEST-007]]
- [[SPEC-001-circus-agent-harness#TEST-014]]
- [[SPEC-001-circus-agent-harness#TEST-020]]
- [[SPEC-001-circus-agent-harness#TEST-021]]
- [[SPEC-001-circus-agent-harness#TEST-022]]
- [[SPEC-001-circus-agent-harness#TEST-023]]
- [[SPEC-001-circus-agent-harness#TEST-056]]
- [[SPEC-001-circus-agent-harness#TEST-057]]
- [[SPEC-001-circus-agent-harness#TEST-058]]

### CON-004: Integration Merge

Interface: `circus merge --attempt ATTEMPT --into REF [--dry-run]`.

Input grammar:

```abnf
ATTEMPT = attempt-id
REF     = ref
```

`--dry-run` reports the outcome and applies nothing —
[[SPEC-001-circus-agent-harness#REQ-009]]. Every pre-condition below is still
checked, so a preview reporting a clean merge is a statement about a merge
Circus permits.

Pre-conditions:

- `ATTEMPT` and `REF` are recognised in full against the grammar above.
- `ATTEMPT` is recorded as accepted.
- `REF` is byte-identical to the `integration_ref` recorded by
  [[SPEC-001-circus-agent-harness#CON-001]].
- No worktree holds `REF` with an uncommitted change.

Post-conditions:

- Circus holds an advisory lock for the whole operation.
- Circus computes the merge in the object database, so no worktree is read or
  written while the outcome is still unknown.
- Circus creates one merge commit, or records a conflict.
- Circus moves `REF` by compare-and-swap against the commit it read.
- WHEN a worktree holds `REF`, Circus brings that worktree in line with the
  merge commit.
- Circus records the merge commit or the conflict result in the run record.
- Circus performs no rebase, amend, cherry-pick, force-update, or push.

Error model:

- An unaccepted attempt returns exit 64 and invokes no Git merge.
- A `REF` that differs from the recorded `integration_ref` returns exit 64 and
  invokes no Git merge.
- A worktree holding `REF` with an uncommitted change returns exit 64. The ref
  does not move and that worktree is not touched.
- A `REF` that moved between the read and the compare-and-swap returns exit 70.
  No ref changes.
- A merge conflict returns exit 1. `REF` remains at its pre-command commit and
  the attempt worktree is unchanged.
- A lock held by another process blocks until it is released. Circus does not
  proceed without the lock.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-005]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-008]]
- [[SPEC-001-circus-agent-harness#TEST-009]]
- [[SPEC-001-circus-agent-harness#TEST-010]]
- [[SPEC-001-circus-agent-harness#TEST-024]]
- [[SPEC-001-circus-agent-harness#TEST-026]]
- [[SPEC-001-circus-agent-harness#TEST-029]]
- [[SPEC-001-circus-agent-harness#TEST-032]]
- [[SPEC-001-circus-agent-harness#TEST-033]]
- [[SPEC-001-circus-agent-harness#TEST-039]]
- [[SPEC-001-circus-agent-harness#TEST-040]]

### CON-005: Elephant Isolation

Interface: every Circus command.

Input grammar: none. This contract accepts no input; it constrains what every
other contract does not do.

Pre-conditions:

- None.

Post-conditions:

- Circus does not invoke the `elephant` executable, under any subcommand.
- Circus does not read or write `ELEPHANT_HOME`, and does not read the
  Elephant default store.
- The `external_programs` array of the run record contains no entry named
  `elephant`.

Error model: this contract has no runtime error surface, because Circus has no
Elephant code path to fail. Its enforcement is split between a test and a
review:

- Decided by [[SPEC-001-circus-agent-harness#TEST-011]] over
  [[SPEC-001-circus-agent-harness#OBS-004]] and over the default store.
- Reviewed by the Tier 2 adversarial review, whose mandate includes finding an
  Elephant code path that the invocation record does not surface.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-006]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-011]]
- [[SPEC-001-circus-agent-harness#TEST-012]]

### CON-006: External Tool Boundary

Interface: all commands.

Pre-conditions:

- Git, tmux, and withdone are discoverable on `PATH` before the command that
  needs each program runs.

Post-conditions:

- Circus reports the absent executable by name and exits 127 when a required
  external program is unavailable.
- Circus records every invoked program in the run record, per
  [[SPEC-001-circus-agent-harness#REQ-007]].c.

Error model:

- Exit 127 names the missing program and starts no other process.
- Circus never substitutes an internal implementation for an unavailable
  external program. This clause is a design prohibition rather than a runtime
  error, and the Tier 2 adversarial review decides it.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-007]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-013]]

### CON-007: Run Record

Interface: the JSON document at `<state-root>/<task>/<attempt-n>/record.json`,
where `<state-root>` is defined by
[[SPEC-001-circus-agent-harness#ADR-006]].

Circus both writes and reads this document. It is the only durable state
Circus owns, and every command's pre-condition on attempt state is a predicate
over it.

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["schema_version", "attempt_id", "repository", "task",
               "attempt", "state", "integration_ref", "worktree_path",
               "branch", "timestamps"],
  "properties": {
    "schema_version":  { "const": 1 },
    "attempt_id":      { "type": "string" },
    "repository":      { "type": "string" },
    "task":            { "type": "string" },
    "attempt":         { "type": "integer", "minimum": 1, "maximum": 9999 },
    "state":           { "enum": ["prepared", "completed", "accepted",
                                  "rejected", "merged", "failed"] },
    "integration_ref": { "type": "string" },
    "worktree_path":   { "type": "string" },
    "branch":          { "type": "string" },
    "pane_name":       { "type": ["string", "null"] },
    "log_path":        { "type": ["string", "null"] },
    "sentinel_path":   { "type": ["string", "null"] },
    "sentinel_value":  { "type": ["integer", "null"] },
    "transport_exit_code": { "type": ["integer", "null"] },
    "completion_method":   { "enum": ["sentinel", "child-exit", "none"] },
    "process_group_residue": { "type": "integer", "minimum": 0 },
    "external_programs": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["name", "resolved_path", "exit_status"],
        "properties": {
          "name":          { "type": "string" },
          "resolved_path": { "type": "string" },
          "exit_status":   { "type": ["integer", "null"] }
        }
      }
    },
    "verifier": {
      "type": ["object", "null"],
      "additionalProperties": false,
      "required": ["command", "exit_code", "output_path"],
      "properties": {
        "command":     { "type": "array", "items": { "type": "string" } },
        "exit_code":   { "type": "integer" },
        "output_path": { "type": "string" }
      }
    },
    "verifier_log": { "type": ["string", "null"] },
    "verifier_log_truncated": { "type": "boolean" },
    "evidence_refs": { "type": "array", "items": { "type": "string" } },
    "decision":      { "enum": ["accepted", "rejected", null] },
    "merge": {
      "type": ["object", "null"],
      "additionalProperties": false,
      "required": ["result"],
      "properties": {
        "result": { "enum": ["merged", "conflict"] },
        "commit": { "type": ["string", "null"] }
      }
    },
    "timestamps": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "prepared_at":  { "type": "string", "format": "date-time" },
        "launched_at":  { "type": ["string", "null"], "format": "date-time" },
        "completed_at": { "type": ["string", "null"], "format": "date-time" },
        "decided_at":   { "type": ["string", "null"], "format": "date-time" },
        "merged_at":    { "type": ["string", "null"], "format": "date-time" }
      }
    }
  }
}
```

Every timestamp is RFC 3339 with a `Z` offset.

`verifier_log` is Circus's own copy of the verifier output, and is distinct
from `verifier.output_path`, which is where the caller left the original. The
copy is the one that survives — [[SPEC-001-circus-agent-harness#REQ-014]].

Pre-conditions:

- A record Circus reads is recognised in full against this schema before any
  field is used.

Post-conditions:

- Circus writes the record atomically. A reader observes either the previous
  state or the new one, never a partial document.
- `parse(serialise(record)) == record` for every record Circus produces.

Error model:

- A record that fails schema recognition returns exit 65. Circus performs no
  state transition on that attempt.

Implements:

- [[SPEC-001-circus-agent-harness#NFR-003]]
- [[SPEC-001-circus-agent-harness#REQ-007]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-014]]
- [[SPEC-001-circus-agent-harness#TEST-028]]

### CON-008: Completion Instruction

Interface: `circus instruction --attempt ATTEMPT`.

Input grammar:

```abnf
ATTEMPT = attempt-id
```

Pre-conditions:

- `ATTEMPT` is recognised in full.
- A run record exists for `ATTEMPT` and is recognised against
  [[SPEC-001-circus-agent-harness#CON-007]].

Post-conditions:

- Circus writes the completion instruction to stdout.
- The instruction contains the attempt's `sentinel_path` verbatim.
- A prompt whose content is the instruction satisfies
  [[SPEC-001-circus-agent-harness#REQ-003]].c.
- Circus creates and modifies nothing.

Error model:

- An unrecognised `ATTEMPT` returns exit 64.
- A missing record returns exit 64, naming the attempt.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-010]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-042]]
- [[SPEC-001-circus-agent-harness#TEST-043]]
- [[SPEC-001-circus-agent-harness#TEST-044]]

### CON-009: Attempt Status

Interface: `circus status [--attempt ATTEMPT]`.

Input grammar:

```abnf
ATTEMPT = attempt-id
```

Pre-conditions:

- WHEN `ATTEMPT` is supplied, it is recognised in full and a run record exists
  for it.

Post-conditions:

- Circus writes a JSON status document to stdout.
- WITH `--attempt`, the document is one object. WITHOUT it, the document is an
  array of them, ordered by task then attempt number.
- Each object carries the attempt's run record under `attempt`, unchanged and
  still conforming to [[SPEC-001-circus-agent-harness#CON-007]], and the
  observations below under `live`.
- Circus creates, modifies, and deletes nothing.

```json
{
  "attempt": { "schema_version": 1, "…": "the CON-007 record" },
  "live": {
    "pane": "circus-model-1",
    "pane_alive": true,
    "running_for_seconds": 412,
    "sentinel_present": false,
    "worktree_present": true,
    "worktree_dirty": true,
    "commits_ahead": 2,
    "transcript_bytes": 18342
  }
}
```

`running_for_seconds` is null unless the attempt was launched and has not
completed. `commits_ahead` counts the task branch over the recorded
integration ref, and is null when either is missing.

Error model:

- An unrecognised `ATTEMPT` returns exit 64.
- A missing record for a named `ATTEMPT` returns exit 64.
- A record that fails schema recognition returns exit 65.
- A repository with no attempts returns exit 0 and an empty array.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-011]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-045]]
- [[SPEC-001-circus-agent-harness#TEST-046]]
- [[SPEC-001-circus-agent-harness#TEST-047]]
- [[SPEC-001-circus-agent-harness#TEST-048]]

### CON-010: Agent Transcript

Interface: `circus logs --attempt ATTEMPT [--follow] [--plain]`.

Input grammar:

```abnf
ATTEMPT = attempt-id
```

Pre-conditions:

- `ATTEMPT` is recognised in full and a run record exists for it.

Post-conditions:

- Circus writes the transcript to stdout, byte for byte, unless `--plain` is
  given.
- WITH `--plain`, Circus removes ANSI control sequences and carriage returns
  and writes the remaining bytes.
- WITH `--follow`, Circus keeps writing appended bytes until the attempt's pane
  is gone and no more output arrives.
- Circus does not modify, truncate, or remove the transcript.

Error model:

- An unrecognised `ATTEMPT` returns exit 64.
- A missing record returns exit 64.
- An attempt with no transcript yet returns exit 0 and writes nothing. A
  prepared attempt has produced no output, which is an answer rather than a
  fault.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-012]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-049]]
- [[SPEC-001-circus-agent-harness#TEST-050]]
- [[SPEC-001-circus-agent-harness#TEST-051]]
- [[SPEC-001-circus-agent-harness#TEST-052]]

### CON-011: Spawn

Interface: `circus spawn --task TASK --integration REF --task-file FILE
[--attempt N] -- DRIVER [ARGS...]`.

Input grammar: as [[SPEC-001-circus-agent-harness#CON-001]] for `TASK`, `REF`,
and `N`, and as [[SPEC-001-circus-agent-harness#CON-002]] for `DRIVER` and
`ARGS`.

```abnf
FILE = abs-path / "-"          ; "-" reads the task from stdin
```

Pre-conditions:

- Every pre-condition of [[SPEC-001-circus-agent-harness#CON-001]].
- `FILE` is readable, or is `-` and stdin is readable.

Post-conditions:

- Circus performs [[SPEC-001-circus-agent-harness#CON-001]], then writes
  `<attempt-dir>/prompt` as the task bytes followed by the attempt's completion
  instruction, then performs
  [[SPEC-001-circus-agent-harness#CON-002]] against that file.
- The run record is the one
  [[SPEC-001-circus-agent-harness#CON-002]] produces.
- Circus does not modify `FILE`.

Error model:

- A preparation failure returns the code
  [[SPEC-001-circus-agent-harness#CON-001]] assigns it, and nothing is
  launched.
- A launch failure returns the code
  [[SPEC-001-circus-agent-harness#CON-002]] assigns it. The prepared attempt
  remains, and its record names the composed prompt.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-013]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-053]]
- [[SPEC-001-circus-agent-harness#TEST-054]]
- [[SPEC-001-circus-agent-harness#TEST-055]]

### CON-012: Attempt History

Interface: `circus history --task TASK`.

Input grammar:

```abnf
TASK = task
```

Pre-conditions:

- `TASK` is recognised in full.

Post-conditions:

- Circus writes one entry per attempt of `TASK` to stdout, oldest first.
- Each entry names the attempt, its decision, the verifier command and exit
  code, what the task branch changed against the recorded integration ref, and
  the paths to the transcript and the captured verifier log.
- A task with no attempts writes nothing and returns 0.
- Circus creates, modifies, and deletes nothing.

Unlike every other command, stdout carries prose rather than a record. The
output is prompt material, and [[SPEC-001-circus-agent-harness#REQ-008]].b fixes the stream, not the
format: this command's primary output is text a caller concatenates into a
prompt.

Error model:

- An unrecognised `TASK` returns exit 64.
- An attempt whose record fails schema recognition is reported as unreadable
  and skipped. One bad record does not withhold the rest of the history.

Implements:

- [[SPEC-001-circus-agent-harness#REQ-015]]

Verified by:

- [[SPEC-001-circus-agent-harness#TEST-059]]
- [[SPEC-001-circus-agent-harness#TEST-060]]

## Architecture Decisions

### ADR-001: Compose Existing Unix Programs

Circus will use Git worktrees for isolation, tmux for visible local process
hosting, and withdone for completion signalling. Reimplementation of those
surfaces increases the trusted code and satisfies no new user need.

Ladder rung: 4 — every capability is assembled from a program already present
on the developer's machine.

### ADR-002: Evidence Before Acceptance and Merge

Circus will not infer correctness from a process result. The lead remains
responsible for theory queries and independent verification, then supplies the
durable records to `circus accept`. This preserves Elephant's distinction
between a signed assertion and a derived conclusion.

Consequence, recorded rather than smoothed over: because
[[SPEC-001-circus-agent-harness#CON-005]] keeps Circus out of Elephant, Circus
cannot tell a real evidence reference from a plausible string. The acceptance
gate it enforces is that a verifier passed and that the caller attested at
least one reference, both recorded durably.
[[SPEC-001-circus-agent-harness#REQ-004]].e states the limit in the
requirement text so no reader infers a stronger guarantee. The alternative was
Circus resolving references itself. That re-creates the supervisor
[[concepts/hence|hence]] was dismantled to remove, and it needs an Elephant
identity [[SPEC-001-circus-agent-harness#ADR-003]] refuses.

### ADR-003: No Automatic Elephant Peer Enrolment

Circus will not automate the Elephant invite ceremony. A harness that passes
invite secrets converts a deliberately out-of-band trust decision into an
implicit local privilege. Pre-joined workers can use Circus without weakening
the identity boundary.

### ADR-004: Driver Contract Is a Governed Spike

Circus will accept a caller-selected driver rather than embed Codex, Claude, or
another provider. The first implementation will run a small, local driver
contract spike.

Exploration brief:

- Hypothesis: a driver needs only three inputs — a working directory, a prompt
  file path, and an argv tail — and needs no provider-specific templating.
- Risk: a provider that reads its prompt from stdin, or that rewrites the
  prompt, breaks [[SPEC-001-circus-agent-harness#REQ-003]].d.
- Metric: the number of distinct drivers launched unmodified through one argv
  shape.
- Exit criteria: at least two unrelated agent CLIs run to sentinel completion
  with no provider branch in Circus.
- Timebox: one week of implementation effort.
- Isolation: spike code lives outside the shipped crate and is decommissioned
  on completion.
- Owner: HOC.

Resolved 2026-08-09. The spike ran, and the contract it settled is:

| | |
|---|---|
| Working directory | The attempt worktree |
| stdin, stdout, stderr | A terminal — the tmux pane, on all three |
| `CIRCUS_PROMPT` | Absolute path to a byte-identical copy of the caller's prompt |
| `CIRCUS_SENTINEL` | The path the agent writes to signal completion |
| `CIRCUS_ATTEMPT` | The `task/attempt` handle |
| `CIRCUS_WORKTREE` | The worktree, same as the working directory |
| argv | The caller's `-- DRIVER [ARGS...]` tail, minus the driver name |

Exit criteria, against the brief above:

- Two unrelated agent CLIs ran to sentinel completion through one argv shape.
  `codex exec` (codex-cli 0.146.1) and `opencode run` (opencode 1.14.48) each
  repaired a deliberately broken test in an attempt worktree and signalled `0`.
- No provider branch exists in Circus. Provider knowledge lives in `drivers/`,
  which is not installed and which the binary does not discover. Deleting the
  directory changes nothing about how Circus treats a caller-supplied driver.

Two findings the spike produced, neither of which the brief predicted:

- **A driver receives a terminal on all three descriptors,** and tmux can type
  into the pane the driver names through `$TMUX_PANE`. Keystroke injection
  therefore needs no `expect`.
- **A full-screen agent is nonetheless unusable today, and not for that
  reason.** withdone enables job control and backgrounds the child, so the
  agent lands in a process group that is not the terminal's foreground group.
  An agent that reads the terminal as it starts receives SIGTTIN and stops,
  permanently and silently: `ps` shows state `T`, the pane stays blank, and the
  transcript is empty. Non-interactive agents never read the terminal, so none
  of the three working drivers is affected. The correction this forces is worth
  recording: `expect` in the withdone recipe allocates a *second* pty in which
  the agent is the foreground process, and that — not PTY allocation as such —
  is what keeps SIGTTIN from firing. The fix belongs upstream in withdone.
- **The completion discriminator was wrong, and the spike is what showed it.**
  See [[SPEC-001-circus-agent-harness#OBS-003]] and
  [[SPEC-001-circus-agent-harness#ADR-009]].

A third input arrived from the same source: the prompt wording every recipe
hand-writes is provider-neutral and belongs in one place, which is
[[SPEC-001-circus-agent-harness#REQ-010]].

Also unresolved: a full-screen agent, for the SIGTTIN reason above, and
Claude's per-directory trust prompt behind it, which
`--dangerously-skip-permissions` does not cover and which fires on every fresh
worktree. `drivers/circus-driver-claude-tui` is kept as an experiment carrying
the diagnosis, not as a working driver.

Not resolved: whether Circus SHOULD ever *discover* drivers by name, the way
Git finds a subcommand. The spike gives no evidence for it — `PATH` already
does the lookup — so the question stays closed until something reopens it.

### ADR-009: How a Signalled Completion Is Told From a Natural One

The first implementation inferred it from an absence. A wrapper script wrote a
marker after the child returned. withdone kills the group as soon as it sees a
sentinel write, so on that path the wrapper never reached its final line. A
missing marker therefore meant the sentinel won.

The [[SPEC-001-circus-agent-harness#ADR-004]] spike falsified that. A
non-interactive agent writes the sentinel as the last act of its turn and then
exits immediately, so the child is gone before withdone notices the write. The
marker gets written. `codex exec` repaired the code, ran
`echo 0 > <sentinel>`, and was recorded as `child-exit` with no sentinel value
— a properly signalled attempt reported as one that gave up.

The mechanism cannot be fixed by racing better. withdone unlinks the sentinel
on both paths, and Circus polls from outside the pane, so any watcher Circus
runs is slower than the wrapper that is already there.

Decision: the wrapper reads the sentinel itself, at the one instant nothing can
have removed it yet — immediately after the child returns. It records what it
saw, and Circus reads three cases from that rather than two:

| Marker | Meaning |
|---|---|
| absent | withdone killed the group, so the sentinel won |
| `wrote=yes` | the agent signalled and then ended its turn; both happened |
| `wrote=no` | the child gave up without signalling |

The middle row is the common case for every non-interactive agent, which is
why getting it wrong mattered. The marker is a declared format recognised in
full before any field is used, like every other input.

### ADR-005: Merge Coordination Stays in Circus

Status: proposed. Awaits steward ratification.

[[SPEC-001-circus-agent-harness#Context]] records that
[[concepts/hence|hence]] was dismantled partly to remove merge coordination, so
[[SPEC-001-circus-agent-harness#REQ-005]] needs an explicit placement
rationale rather than an assumption.

Composition-first check (ladder rung 4): the lead can run
`flock <lock> git merge --no-ff <branch>` itself. That covers serialisation and
the merge. It does not cover the accepted-state pre-condition, because the
accepted state lives in the run record, and only Circus owns that record.
Reproducing the check in the lead means a second reader of
[[SPEC-001-circus-agent-harness#CON-007]], which is the parser-differential
risk Constitutional Principle 14 prohibits.

Decision: the capability stays in Circus, because Circus already owns the state
the gate is stated over. This is the narrow reading of Constitutional Principle
15. An option belongs on an artefact that already holds the right
responsibility, and nowhere else.

Cap, so this does not become the supervisor again:
[[SPEC-001-circus-agent-harness#REQ-005]].e forbids rebase, amend,
cherry-pick, force-update, and push. Circus performs one merge of one accepted
branch into one recorded ref, and nothing else. Conflict resolution, strategy
selection, and publication stay with the lead.

Rejected alternative: expose `circus show --attempt` and let the lead gate its
own merge. This keeps Circus smaller, and it moves an
[[SPEC-001-circus-agent-harness#Amendment Channels|Hard stops]] control into
caller discipline, where nothing enforces it. The steward decides.

### ADR-006: Attempt Identity and State Root

An attempt is identified by `attempt-id = task "/" attempt-n`. That identifier
is the `--attempt` argument of every command after `prepare`, and it is printed
by `circus prepare`.

The state root is `$(git rev-parse --git-common-dir)/circus`. The common
directory is shared by every worktree of the repository, and it is never a
worktree itself. Three obligations fall out for free:

- [[SPEC-001-circus-agent-harness#REQ-002]].d — the attempt log is outside
  every worktree.
- [[SPEC-001-circus-agent-harness#REQ-003]].b — the sentinel path is outside
  every worktree.
- [[SPEC-001-circus-agent-harness#NFR-001]].a — attempt state survives removal
  of the worktree it describes.

Ladder rung: 3 — a native Git feature covers it, so Circus introduces no
configuration key, no environment variable, and no dotfile.

Per-attempt layout:

```
<state-root>/<task>/<attempt-n>/record.json
<state-root>/<task>/<attempt-n>/transcript.log
<state-root>/<task>/<attempt-n>/sentinel
<state-root>/.lock                              ; allocation + merge lock
```

### ADR-007: Temporal Notation for Bounded Cleanup

[[SPEC-001-circus-agent-harness#NFR-002]] is a bounded-response obligation, and
this specification is Tier 2, so the obligation is stated as a temporal
formula rather than as prose.

Notation: the generic operator set of [[PROTO-001]] — `always`,
`eventually[a, b]`, with explicit units and named atoms.

Toolchain: none selected. No monitor, exemplifier, or falsifier runs against
this formula today, so the formula is checked only by
[[SPEC-001-circus-agent-harness#TEST-005]] and
[[SPEC-001-circus-agent-harness#TEST-030]]. This gap is recorded as
`unverified` in the
[[SPEC-001-circus-agent-harness#Gate Evidence Record]] with owner HOC, rather
than presented as a satisfied obligation.

### ADR-008: Colour and Terminal Detection

The [[SPEC-001-circus-agent-harness#Contracts]] template requires behaviour to
be invariant across calling context, and names reformatting output when stdout
is a terminal as the example of what needs justifying. Circus styles its stderr
messages when stderr is a terminal, so this ADR is that justification.

The variance is confined to one stream. Stdout carries
[[SPEC-001-circus-agent-harness#CON-007]] and is byte-identical whether it
reaches a terminal, a pipe, or a file —
[[SPEC-001-circus-agent-harness#REQ-008]].b. Nothing a caller parses ever
changes shape. What varies is the styling of human messages on stderr, which no
contract describes.

Styling is applied only when every one of these holds:

- stderr is a terminal.
- `--no-color` was not supplied.
- `NO_COLOR` is unset or empty.
- `TERM` is not `dumb`.

Three of those four are the conventions the ecosystem already agreed on, and
following them is cheaper than inventing a fourth. The flag exists because an
operator sometimes wants plain output from a terminal, which no environment
variable expresses.

The same detection governs the progress display of
[[SPEC-001-circus-agent-harness#REQ-008]].d. A terminal receives one line
rewritten in place; anything else receives a new line at each interval, because
a carriage return in a log file produces an unreadable smear rather than an
update.

Ladder rung: 2 — `std::io::IsTerminal` and two environment variables. No
terminal-handling dependency is introduced for four ANSI escapes.

### ADR-010: Which Process Group Is Measured

[[SPEC-001-circus-agent-harness#NFR-002]] fails an attempt whose process group is not empty at the deadline,
and the first implementation measured the group of the tmux pane. That is the
wrong group, and the error hid on one platform.

A pane's process group legitimately holds Circus's own scaffolding: the runner
script, and withdone itself. Both are `sh`, both are meant to be there, and both
exit on their own schedule after the status file appears. Counting them
conflates the harness with the workload it is measuring.

On macOS they happened to exit within the deadline and the conflation was
invisible. On Linux one of them outlived it, so `state` became `failed`,
`launch` exited 1, and every launch-based test failed in CI while passing on the
developer's machine. Three rounds of diagnosis went into finding it: first the
count was wrong (`ps -g`, [[SPEC-001-circus-agent-harness#ADR-009]]'s sibling problem), then the count was
right but nameless, and only once the survivor was printed — `sh (pid 1925)` —
was it clear that the thing being counted was Circus's own plumbing.

Decision: measure the group the *agent* runs in. withdone enables job control,
so the child it wraps gets a process group of its own, and that group holds the
agent and its descendants and nothing of Circus's. The wrapper records it to
`<attempt-dir>/pgid` before the agent starts, because the wrapper is the only
participant that is inside that group and can name it.

No recorded group means the wrapper never ran, or `ps` is absent. Either way the
group is unmeasurable and is reported clean, which is the convention everywhere
else Circus cannot observe something.

The control is unchanged in force: [[SPEC-001-circus-agent-harness#TEST-030]] still fails an attempt whose
agent leaves a process behind. What changed is that it now watches the agent.

## Verification Strategy

Tier: 2. Circus owns core contracts, a trust boundary that reads
caller-supplied files, and a merge that can destroy work. Cross-model review
and human review are both required before status `approved`.

| Surface | Technique | Scope |
|---|---|---|
| Prompt file, verifier record, run record | Fuzzing | REQUIRED at the trust boundary — [[SPEC-001-circus-agent-harness#CON-002]], [[SPEC-001-circus-agent-harness#CON-003]], [[SPEC-001-circus-agent-harness#CON-007]] |
| Run record | Property-based roundtrip | REQUIRED — Circus reads and writes this format ([[SPEC-001-circus-agent-harness#TEST-028]]) |
| Path derivation, sentinel recognition, acceptance predicate | Property-based testing | The pure core of [[SPEC-001-circus-agent-harness#Purity Boundary Map]] |
| Every module | Mutation testing | REQUIRED — this specification is AI-synthesised, so Red Gate discipline cannot be assumed |
| Whole specification | Adversarial testing | REQUIRED before status `approved`, iterated to adversary exhaustion |
| Orientation block | Comprehension gate | REQUIRED — Tier 2 and AI-synthesised |
| Merge and preparation locking | Integration testing | Concurrent invocation, real Git — [[SPEC-001-circus-agent-harness#TEST-024]], [[SPEC-001-circus-agent-harness#TEST-025]] |

Falsification is not claimed. The pure core below is small, and no model of the
effectful shell exists to search over.

## Purity Boundary Map

### Pure Core (no I/O, no shared state, deterministic)

- `attempt_paths`: derives worktree path, branch name, pane name, log path, and
  sentinel path from repository, task, and attempt number.
- `recognise_prompt`: decides whether a byte sequence contains the sentinel
  path as a literal substring.
- `recognise_verifier_record`: decides a byte sequence against the
  [[SPEC-001-circus-agent-harness#CON-003]] schema.
- `acceptance_decision`: maps a verifier exit code and an evidence-reference
  list to `accepted` or `rejected`.
- `record_codec`: parses and serialises
  [[SPEC-001-circus-agent-harness#CON-007]].

### Effectful Shell (orchestrates I/O, calls pure core)

- `git_ops`: worktree, branch, and merge invocation.
- `pane_ops`: tmux and withdone invocation, log capture, process-group cleanup.
- `state_store`: atomic read and write of the run record, advisory locking.

### Boundary Contracts (data types crossing the boundary)

- `AttemptPaths`: core → shell.
- `RunRecord`: both directions.
- `VerifierRecord`: shell → core.
- `Decision`: core → shell.

### Dependency Rule

Dependencies point inward: shell → core. Core MUST NOT import from shell.

### Enforcement

Module visibility plus an import lint in CI.

## Implementation

The `CODE` link of the traceability chain. Every contract names the module that
satisfies it. A failing test lifts to a requirement, and that requirement
reaches one file rather than a search.

| Artefact | Module |
|---|---|
| Shared productions, [[SPEC-001-circus-agent-harness#CON-001]] grammar | `src/core/grammar.rs` |
| [[SPEC-001-circus-agent-harness#CON-002]] prompt recognition | `src/core/prompt.rs` |
| [[SPEC-001-circus-agent-harness#CON-003]] verifier record | `src/core/verifier.rs` |
| [[SPEC-001-circus-agent-harness#REQ-004]].c acceptance predicate | `src/core/decision.rs` |
| [[SPEC-001-circus-agent-harness#CON-007]] run record codec | `src/core/record.rs` |
| [[SPEC-001-circus-agent-harness#ADR-006]] path derivation | `src/core/paths.rs` |
| Shell-text generation | `src/core/shquote.rs` |
| RFC 3339 timestamps | `src/core/time.rs` |
| [[SPEC-001-circus-agent-harness#CON-001]], [[SPEC-001-circus-agent-harness#CON-004]] Git operations | `src/shell/git.rs` |
| [[SPEC-001-circus-agent-harness#CON-002]] pane launch and cleanup measurement | `src/shell/pane.rs` |
| [[SPEC-001-circus-agent-harness#REQ-005]].b lock, atomic record I/O | `src/shell/state.rs` |
| [[SPEC-001-circus-agent-harness#REQ-007]].c invocation recording | `src/shell/proc.rs` |
| Command dispatch and error models | `src/main.rs` |

Two facts about the composed tools were established by experiment during
implementation, and both shape the design:

- A tmux pane is forked by the tmux server, not by its caller, so completion
  cannot be learned by waiting on a child. It is read from a status file the
  pane writes.
- withdone unlinks the sentinel on both exit paths. Its exit code is the
  sentinel value on one path and the child's own code on the other. A wrapper
  script therefore records the child's own exit, and its absence identifies
  the sentinel path.

## Tests

Every entry names the atoms it validates and the test type it supplies. One
intent per entry, so a failure attributes to one obligation.

Each entry is implemented as a test of the same number in `tests/spec.rs`.
`tests/traceability.rs` checks that correspondence mechanically, in both
directions.

### TEST-001: Create Exactly One Worktree and Branch

Validates: [[SPEC-001-circus-agent-harness#REQ-001]].a,
[[SPEC-001-circus-agent-harness#REQ-001]].b,
[[SPEC-001-circus-agent-harness#CON-001]] — positive, exact-count

Given a repository with integration branch `circus/spec-001`, the lead runs
`circus prepare --task model --integration circus/spec-001`. The command prints
one run record. `git worktree list` gains exactly one entry, and `git branch`
gains exactly one branch. The new branch points at the tip of
`circus/spec-001`.

### TEST-002: Preserve the Caller Worktree

Validates: [[SPEC-001-circus-agent-harness#REQ-001]].d — scope-invariant

Given a dirty caller worktree, the lead prepares an attempt. The caller's HEAD,
index, and every tracked file are byte-identical to their pre-command state.
The caller's untracked file set is unchanged.

### TEST-003: Start a Named tmux Pane

Validates: [[SPEC-001-circus-agent-harness#REQ-002]].a,
[[SPEC-001-circus-agent-harness#REQ-002]].b — positive

Given a prepared attempt and a fixture driver, the lead launches the attempt.
Exactly one new tmux pane exists. Its name contains `model` and `1`. Its
working directory is the attempt worktree.

### TEST-004: Require the Literal Sentinel in the Prompt

Validates: [[SPEC-001-circus-agent-harness#REQ-003]].c — negative-input

Given a prompt that omits the attempt sentinel path, the lead launches the
attempt. Circus exits 64. No pane exists and no driver process ran.

### TEST-005: Terminate After Explicit Completion

Validates: [[SPEC-001-circus-agent-harness#REQ-003]].a,
[[SPEC-001-circus-agent-harness#NFR-002]].a,
[[SPEC-001-circus-agent-harness#NFR-002]].b — positive

Given a fixture driver that spawns a child and then writes `0` to its sentinel,
withdone observes the write. Within 10 s of the write, no member of the fixture
process group is alive. The run record reports `completion_method: sentinel`
and `process_group_residue: 0`.

### TEST-006: Reject a Transport-Only Outcome

Validates: [[SPEC-001-circus-agent-harness#REQ-004]].b — prohibited-action

Given a completed attempt whose driver exited 0, the lead runs `circus accept`
with no verifier record. Circus exits 64. The run record `state` is still
`completed` and `decision` is still null.

### TEST-007: Reject a Failed Verifier

Validates: [[SPEC-001-circus-agent-harness#REQ-004]].c — negative-input

Given a schema-valid verifier record whose `exit_code` is 1, and one evidence
reference, the lead requests acceptance. Circus exits 1 and records `rejected`.
The attempt worktree and branch still exist.

### TEST-008: Merge an Accepted Attempt

Validates: [[SPEC-001-circus-agent-harness#REQ-005]].a — positive

Given an accepted attempt with a non-conflicting change, the lead runs
`circus merge --into circus/spec-001`. The integration branch gains exactly one
merge commit. The run record reports `merge.result: merged`.

### TEST-009: Preserve a Rejected Attempt

Validates: [[SPEC-001-circus-agent-harness#REQ-005]].a,
[[SPEC-001-circus-agent-harness#NFR-001]].a — prohibited-action

Given a rejected attempt, the lead runs `circus merge`. Circus exits 64 and
invokes no Git merge. The worktree, branch, transcript log, and run record all
still exist.

### TEST-010: Reject a Conflicting Merge

Validates: [[SPEC-001-circus-agent-harness#REQ-005]].d — scope-invariant

Given an accepted branch that conflicts with the integration branch, the lead
runs `circus merge`. Circus exits 1. The integration ref is at its pre-command
commit, and the attempt worktree is unchanged.

### TEST-011: Never Invoke Elephant

Validates: [[SPEC-001-circus-agent-harness#REQ-006]].a,
[[SPEC-001-circus-agent-harness#REQ-006]].c,
[[SPEC-001-circus-agent-harness#REQ-007]].c,
[[SPEC-001-circus-agent-harness#CON-005]] — prohibited-action

Every Circus command is exercised end to end. Three assertions hold together.
The `external_programs` array of every run record contains no entry named
`elephant`. A temporary Elephant home remains empty. The default Elephant store
is byte-identical to its pre-run state.

The third assertion is load-bearing. Circus ignores `ELEPHANT_HOME` by
[[SPEC-001-circus-agent-harness#CON-005]], so a test that watches only the
temporary home passes while a write lands in the default store.

### TEST-012: Do Not Parse Invite Material

Validates: [[SPEC-001-circus-agent-harness#REQ-006]].b,
[[SPEC-001-circus-agent-harness#REQ-006]].d — prohibited-action

Given a driver prompt containing an invite-shaped token, the lead launches the
attempt. The run record contains no field derived from that token. Circus makes
no network call and spawns no `elephant` process. The attempt log is permitted
to contain the token verbatim, because Circus copies driver output without
inspecting it.

### TEST-013: Fail Clearly for a Missing Tool

Validates: [[SPEC-001-circus-agent-harness#REQ-007]].a,
[[SPEC-001-circus-agent-harness#CON-006]] — negative

Given a `PATH` without tmux, the lead runs `circus launch`. Circus exits 127
and names tmux. No agent process starts. The same holds for a `PATH` without
withdone, and for a `PATH` without Git.

### TEST-014: Emit One Complete Run Record

Validates: [[SPEC-001-circus-agent-harness#NFR-003]].a,
[[SPEC-001-circus-agent-harness#NFR-003]].b,
[[SPEC-001-circus-agent-harness#CON-007]] — positive

Given an attempt taken through prepare, launch, accept, and merge, the run
record validates against the [[SPEC-001-circus-agent-harness#CON-007]] schema at each transition. After merge it
carries a non-null `verifier`, a non-empty `evidence_refs`, and a non-null
`decision`. It also carries a non-null `merge`, a `completion_method`, and a
`process_group_residue`.

### TEST-015: Name the Worktree Path

Validates: [[SPEC-001-circus-agent-harness#REQ-001]].c — positive

Given a repository named `circus`, the lead prepares task `model` attempt 1.
The worktree path contains `circus`, `model`, and `1`.

### TEST-016: Roll Back a Partial Preparation

Validates: [[SPEC-001-circus-agent-harness#CON-001]] error model — negative

Given a Git failure injected after branch creation, the lead prepares an
attempt. Circus exits 70. No branch, worktree, or attempt directory from that
call remains.

### TEST-017: Keep the Attempt Log Outside the Worktree

Validates: [[SPEC-001-circus-agent-harness#REQ-002]].c,
[[SPEC-001-circus-agent-harness#REQ-002]].d — scope-invariant

Given a fixture driver that writes a known line to stdout and another to
stderr, the lead launches the attempt. The attempt log contains both lines. The
log path is not under the attempt worktree. `git status` in the worktree
reports no new untracked file.

### TEST-018: Keep the Sentinel Path Outside the Worktree

Validates: [[SPEC-001-circus-agent-harness#REQ-003]].b — scope-invariant

Given a completed attempt whose worker wrote the sentinel, the sentinel path is
not under the attempt worktree. `git status` in the worktree reports no
untracked sentinel file.

### TEST-019: Pass the Prompt Through Unchanged

Validates: [[SPEC-001-circus-agent-harness#REQ-003]].d,
[[SPEC-001-circus-agent-harness#REQ-003]].e,
[[SPEC-001-circus-agent-harness#REQ-007]].b — prohibited-action

Given a prompt file with known bytes, the lead launches a fixture driver that
copies the prompt it receives. The copied bytes are identical to the caller's
file. No preamble, no suffix, and no substitution appear.

### TEST-020: Accept a Verified Attempt

Validates: [[SPEC-001-circus-agent-harness#REQ-004]].a,
[[SPEC-001-circus-agent-harness#REQ-004]].c — positive

Given a completed attempt, a schema-valid verifier record with `exit_code` 0,
and one evidence reference, the lead requests acceptance. Circus exits 0 and
records `decision: accepted`. The run record `state` becomes `accepted`.

### TEST-021: Reject Acceptance Without an Evidence Reference

Validates: [[SPEC-001-circus-agent-harness#REQ-004]].c — negative-input

Given a schema-valid verifier record with `exit_code` 0 and no `--evidence`
argument, the lead requests acceptance. Circus exits 64. The run record
`decision` is still null.

### TEST-022: Record Evidence References Verbatim

Validates: [[SPEC-001-circus-agent-harness#REQ-004]].d,
[[SPEC-001-circus-agent-harness#REQ-004]].e — positive, prohibited-action

Given two evidence references, the lead accepts the attempt. The
`evidence_refs` array holds both strings byte-for-byte and in order. Circus
opened no socket and spawned no process during the call.

### TEST-023: Reject a Malformed Verifier Record

Validates: [[SPEC-001-circus-agent-harness#CON-003]] grammar — negative-input

Given a verifier record with an unknown property, a missing `exit_code`, or an
`exit_code` of 300, the lead requests acceptance. Circus exits 64 in each case.
The run record `decision` is still null.

### TEST-024: Serialise Concurrent Merges

Validates: [[SPEC-001-circus-agent-harness#REQ-005]].b — concurrency

Given two accepted attempts on one integration branch, the lead starts both
merges at the same moment. Both succeed. The integration branch carries exactly
two merge commits, and neither merge observes a half-written run record.

### TEST-025: Serialise Concurrent Preparations

Validates: [[SPEC-001-circus-agent-harness#CON-001]] concurrency,
[[SPEC-001-circus-agent-harness#ADR-006]] — concurrency

Given no existing attempt, the lead starts two `circus prepare` calls for one
task at the same moment. The two calls return distinct attempt numbers. Two
worktrees and two branches exist, and no attempt directory is shared.

### TEST-026: Reject a Merge Into a Different Ref

Validates: [[SPEC-001-circus-agent-harness#REQ-005]].c — negative-input

Given an attempt prepared from `circus/spec-001`, the lead runs `circus merge
--into main`. Circus exits 64 and invokes no Git merge. `main` is at its
pre-command commit.

### TEST-027: Reject Relaunch of a Launched Attempt

Validates: [[SPEC-001-circus-agent-harness#CON-002]] error model —
negative-input

Given an attempt whose state is `completed`, the lead runs `circus launch`
again. Circus exits 64. No second pane exists, and the attempt log is
unchanged.

### TEST-028: Roundtrip the Run Record

Validates: [[SPEC-001-circus-agent-harness#NFR-003]].b,
[[SPEC-001-circus-agent-harness#CON-007]] — property-based

For every generated record valid under the [[SPEC-001-circus-agent-harness#CON-007]] schema,
`parse(serialise(record))` equals `record`. The generator covers every `state`
value, both null and populated optional objects, and the maximum attempt
number.

### TEST-029: Merge Performs No Rebase or Push

Validates: [[SPEC-001-circus-agent-harness#REQ-005]].e — scope-invariant

Given an accepted attempt in a repository with a remote, the lead merges. The
reflog of the integration ref gains exactly one entry. No ref was force-updated
and the remote received no push.

### TEST-030: Record a Failed Attempt on Cleanup Timeout

Validates: [[SPEC-001-circus-agent-harness#NFR-002]].c — negative

Given a fixture child that ignores SIGTERM and outlives the 10 s deadline, the
attempt completes. Circus records `state: failed` and a non-zero
`process_group_residue`. The attempt worktree and log are preserved.

### TEST-031: Never Delete Attempt State

Validates: [[SPEC-001-circus-agent-harness#NFR-001]].b — prohibited-action

Given attempts in each of `prepared`, `completed`, `rejected`, and `failed`,
every Circus command is exercised. Each attempt's worktree, branch, transcript
log, and run record still exist afterwards. The state root contains exactly the
attempt directories that existed before, plus those the run created.

### TEST-032: Refuse a Merge Into a Dirty Checkout

Validates: [[SPEC-001-circus-agent-harness#REQ-005]].f — negative-input,
scope-invariant

Given an accepted attempt and an uncommitted change in the worktree holding the
integration ref, the lead runs `circus merge`. Circus exits 64. The integration
ref is at its pre-command commit. The uncommitted change is still there.

### TEST-033: Refresh the Checkout Holding the Integration Ref

Validates: [[SPEC-001-circus-agent-harness#REQ-005]].g — positive

Given an accepted attempt that adds a file, and a clean worktree holding the
integration ref, the lead merges. That worktree contains the added file
afterwards. `git status` there reports no change.

### TEST-034: Keep the Two Streams Apart

Validates: [[SPEC-001-circus-agent-harness#REQ-008]].a,
[[SPEC-001-circus-agent-harness#REQ-008]].b — scope-invariant

Every command is run with the two streams captured separately. Stdout parses
as one run record and contains nothing else. Every progress line, hint, and
error appears on stderr.

### TEST-035: Report the Attach Command Before Waiting

Validates: [[SPEC-001-circus-agent-harness#REQ-008]].c — positive

Given a driver that runs until the sentinel is written from outside, the lead
launches it. The attach command naming the pane appears on stderr before the
attempt completes.

### TEST-036: Report Elapsed Time While an Attempt Runs

Validates: [[SPEC-001-circus-agent-harness#REQ-008]].d — positive

Given a driver that runs for longer than one progress interval, the lead
launches it. Stderr carries at least one line reporting elapsed time.

### TEST-037: Silence Everything but Errors

Validates: [[SPEC-001-circus-agent-harness#REQ-008]].e — prohibited-action

Given `--quiet`, a successful command writes nothing to stderr, and its run
record still reaches stdout. A failing command still writes its error.

### TEST-038: Report Invocations Under Verbose

Validates: [[SPEC-001-circus-agent-harness#REQ-008]].f — positive

Given `--verbose`, a successful `prepare` names `git` on stderr. The same
command without the flag does not.

### TEST-039: Preview a Conflicting Merge

Validates: [[SPEC-001-circus-agent-harness#REQ-009]].a — positive

Given an accepted attempt that conflicts, the lead runs `circus merge
--dry-run`. Circus reports the conflict and names the conflicting path.

### TEST-040: A Preview Changes Nothing

Validates: [[SPEC-001-circus-agent-harness#REQ-009]].b — prohibited-action,
scope-invariant

Given an accepted attempt that merges cleanly, the lead runs `circus merge
--dry-run`. The integration ref is at its pre-command commit, the run record is
byte-identical to before, and no worktree changed. A subsequent real merge then
succeeds.

### TEST-041: Style Only a Terminal

Validates: [[SPEC-001-circus-agent-harness#ADR-008]] — negative

With stderr captured to a pipe, no output contains an ANSI escape. The same
holds with `NO_COLOR` set, with `TERM=dumb`, and with `--no-color`.

### TEST-042: Print an Instruction a Prompt Can Use

Validates: [[SPEC-001-circus-agent-harness#REQ-010]].a,
[[SPEC-001-circus-agent-harness#REQ-010]].b — positive

Given a prepared attempt, the lead runs `circus instruction`. Stdout contains
the attempt's sentinel path verbatim. A prompt file built from that output
launches without the recogniser refusing it.

### TEST-043: Print Without Writing

Validates: [[SPEC-001-circus-agent-harness#REQ-010]].c — prohibited-action

Given a prepared attempt, the lead runs `circus instruction`. The attempt
directory holds exactly the files it held before, byte for byte, and no prompt
file exists.

### TEST-044: Keep the Instruction Off the Record Stream

Validates: [[SPEC-001-circus-agent-harness#REQ-010]].a — scope-invariant

Stdout from `circus instruction` does not parse as a run record, and carries no
message. The attempt's own record is unchanged.

### TEST-045: Report a Recorded Attempt

Validates: [[SPEC-001-circus-agent-harness#REQ-011]].a — positive

Given a prepared attempt, the lead runs `circus status --attempt`. Stdout
carries one object whose `attempt` is the attempt's record and whose `live`
reports the pane absent.

### TEST-046: Report an Attempt That Is Still Running

Validates: [[SPEC-001-circus-agent-harness#REQ-011]].a — positive

Given a launch still in progress, the lead runs `circus status --attempt` from
another process. `live.pane_alive` is true and `live.running_for_seconds` is a
number, while the record still reads `prepared`.

### TEST-047: Report Every Attempt

Validates: [[SPEC-001-circus-agent-harness#REQ-011]].b — positive

Given three attempts across two tasks, the lead runs `circus status`. Stdout
carries an array of three objects, ordered by task then attempt number. A
repository with no attempts yields an empty array.

### TEST-048: Report Without Touching

Validates: [[SPEC-001-circus-agent-harness#REQ-011]].c — prohibited-action,
scope-invariant

Given a prepared attempt, the lead runs `circus status` in both forms. Every
file under the state root is byte-identical afterwards, the worktree is
unchanged, and no ref moved.

### TEST-049: Write the Transcript

Validates: [[SPEC-001-circus-agent-harness#REQ-012]].a — positive

Given a completed attempt whose driver printed to both streams, the lead runs
`circus logs`. Stdout carries both lines. A prepared attempt yields empty
stdout and exit 0.

### TEST-050: Follow a Running Attempt

Validates: [[SPEC-001-circus-agent-harness#REQ-012]].b — positive

Given a driver that prints, waits, and prints again, the lead runs
`circus logs --follow` alongside it. Both lines arrive, and the command returns
on its own once the attempt stops.

### TEST-051: Strip Terminal Control Sequences

Validates: [[SPEC-001-circus-agent-harness#REQ-012]].c — positive

Given a driver that emits colour and a carriage return, `circus logs --plain`
writes the text with no escape byte in it. The same command without `--plain`
writes the escapes.

### TEST-052: Read Without Writing

Validates: [[SPEC-001-circus-agent-harness#REQ-012]].d — prohibited-action

Given a completed attempt, the lead runs `circus logs` in every form. The
transcript is byte-identical afterwards, and the run record is unchanged.

### TEST-053: Prepare and Launch Under One Command

Validates: [[SPEC-001-circus-agent-harness#REQ-013]].a,
[[SPEC-001-circus-agent-harness#REQ-013]].b — positive

Given a task file and a driver that writes the sentinel, the lead runs
`circus spawn`. One worktree and one branch exist, the attempt reaches
`completed` by sentinel, and the composed prompt holds the task text followed
by the sentinel path. The same works with the task on stdin.

### TEST-054: Leave the Caller's Task File Alone

Validates: [[SPEC-001-circus-agent-harness#REQ-013]].c — scope-invariant

Given a task file, the lead spawns an attempt. The task file is byte-identical
afterwards, and the composed prompt exists only inside the attempt directory.

### TEST-055: Keep a Prepared Attempt When the Launch Fails

Validates: [[SPEC-001-circus-agent-harness#REQ-013]].d — negative

Given a driver that is not executable, the lead spawns an attempt. The command
exits non-zero, and the worktree, branch, and record of the prepared attempt
all remain.

### TEST-056: Capture the Verifier Output

Validates: [[SPEC-001-circus-agent-harness#REQ-014]].a, [[SPEC-001-circus-agent-harness#REQ-014]].b — positive

Given a verifier record naming a readable output file, the lead accepts the
attempt. `<attempt-dir>/verifier.log` holds that file's bytes, and the run
record names it with `verifier_log_truncated` false. Given a source over 1 MiB,
the copy is 1 MiB and the flag is true.

### TEST-057: Leave the Caller's Output Alone

Validates: [[SPEC-001-circus-agent-harness#REQ-014]].c — scope-invariant

Given a verifier output file, the lead accepts the attempt. The caller's file
is byte-identical afterwards.

### TEST-058: Accept Without a Readable Verifier Output

Validates: [[SPEC-001-circus-agent-harness#REQ-014]].d — negative

Given a verifier record whose `output_path` does not exist, the lead accepts
the attempt. Circus records the same decision it records with a readable
output file, and `verifier_log` is null.

### TEST-059: Report Every Attempt of a Task

Validates: [[SPEC-001-circus-agent-harness#REQ-015]].a, [[SPEC-001-circus-agent-harness#REQ-015]].b — positive

Given a rejected attempt and an accepted one, `circus history` writes both to
stdout, oldest first, each naming its decision and verifier exit code. An
attempt whose branch changed nothing says so. A task with no attempts writes
nothing and returns 0.

### TEST-060: Report History Without Touching

Validates: [[SPEC-001-circus-agent-harness#REQ-015]].c — prohibited-action

Given two attempts, the lead runs `circus history`. Every file under the state
root is byte-identical afterwards, and no ref moved.

## Observability

### OBS-001: Attempt Lifecycle Record

The run record's `state` and `timestamps` fields record preparation, launch,
completion, decision, and merge. Locations are recorded by `worktree_path`,
`branch`, `pane_name`, and `log_path`.

### OBS-002: Acceptance Evidence Record

The run record's `verifier` field records the verifier command, its exit code,
and its output path. The `evidence_refs` and `decision` fields record every
evidence reference and the acceptance decision.

### OBS-003: Process Cleanup Result

The run record's `completion_method` and `process_group_residue` fields record
how the attempt ended and whether the process group was empty. The group is the
agent's, not the pane's — [[SPEC-001-circus-agent-harness#ADR-010]]. These two fields
are the signals [[SPEC-001-circus-agent-harness#NFR-002]] is stated over.

### OBS-004: External Program Invocation Record

The run record's `external_programs` array records every external program
Circus executed for the attempt, with its resolved path and exit status. It is
the artefact [[SPEC-001-circus-agent-harness#TEST-011]] checks the
[[SPEC-001-circus-agent-harness#REQ-006]].c prohibition against.

### OBS-005: Live Attempt Observation

The status document's `live` object records what was true of an attempt at the
moment it was queried: whether tmux still holds the pane, how long it ran so far, whether the
sentinel appeared, and how far the task branch moved. Unlike every other signal here it is not written to the run record,
because it describes the present rather than a transition.

## Quality Gates

Phase 2:

- [x] Every requirement atom links to at least one TEST of every applicable
      type.
- [x] Every prohibitive atom has a prohibited-action TEST.
- [x] Every side-effecting atom has a scope-invariant TEST.
- [x] Every contract accepting external input declares a grammar.
- [x] Every contract declares pre-conditions, post-conditions, and errors.
- [x] Every capability records a Simplicity Ladder rung and a placement
      rationale.
- [ ] A fresh-context reviewer has checked the Orientation block.
- [ ] A cross-model reviewer has reviewed the specification (Tier 2).
- [ ] A synthetic-user simulation covers [[users/lead-agent/happy-paths]].
- [x] The driver contract spike has resolved
      [[SPEC-001-circus-agent-harness#ADR-004]].
- [ ] The steward has ratified or rejected
      [[SPEC-001-circus-agent-harness#ADR-005]].

Phase 3:

- [x] Every TEST entry is implemented and green.
- [x] Simplicity: complexity lint clean, ladder rung recorded per capability.
- [x] Test surface: mutation kill rate on the pure core.
- [x] Architecture: the Purity Boundary Map's dependency rule is enforced.
- [x] Two-sided verification: prohibited-action and scope-invariant tests pass.
- [x] Traceability: REQ atom to TEST to CODE, checked mechanically.
- [x] Documentation: README and a four-mode user documentation tree exist.
- [ ] Security: no dependency-audit tool is installed on this machine.

## Gate Evidence Record

```yaml
phase: 2
gates:
  - gate: "π totality — every atom traces to a TEST, every TEST attributes an atom"
    mechanism: "script: parse atom declarations and Validates fields, diff the two sets"
    result: pass
    evidence: "2026-08-08 — 34 declared atoms, 34 cited; UNCOVERED: none; 31 TEST headings, 31 Validates fields. NFR-002.a and NFR-002.b are declared in the temporal formula block rather than as bold atoms."
  - gate: "Wikilinks resolve; no dead links"
    mechanism: "zetl check --dead-links --fail-on error"
    result: pass
    evidence: "0 dead links, 0 orphans, exit 0 — run 2026-08-08"
  - gate: "Controlled language"
    mechanism: "tools/usdd-lint.sh --show"
    result: pass
    evidence: "2026-08-08 — 0 errors across the spec, both user pages, and all 7 concept pages. 1 residual warning: the BCP 14 conformance sentence at line 11, which PROTO-001 mandates verbatim at 39 words."
  - gate: "Every contract accepting external input declares a grammar"
    mechanism: "manual presence check per CON"
    result: pass
    evidence: "CON-001 ABNF, CON-002 ABNF, CON-003 JSON Schema, CON-004 ABNF, CON-007 JSON Schema; CON-005 and CON-006 accept no input"
  - gate: "Fresh-context Orientation review"
    mechanism: "reviewer: separate agent, Orientation block only"
    result: unverified
    evidence: "not yet requested; owner HOC"
  - gate: "Cross-model adversarial review (Tier 2)"
    mechanism: "reviewer: different model family, spec and deliverable only"
    result: unverified
    evidence: "single-family review completed 2026-08-08; cross-family outstanding; owner HOC"
  - gate: "Synthetic-user simulation of the lead-agent happy path"
    mechanism: "sub-agent simulation per PROTO-001 Synthetic User Protocol"
    result: unverified
    evidence: "not yet run; owner HOC"
  - gate: "Temporal property NFR-002 exemplified before approval"
    mechanism: "solver; none selected"
    result: unverified
    evidence: "no toolchain; see ADR-007; owner HOC"
  - gate: "ADR-005 merge-coordination placement ratified"
    mechanism: "steward decision recorded in this document"
    result: unverified
    evidence: "ADR-005 status proposed; owner HOC"
  - gate: "Driver contract spike resolves ADR-004"
    mechanism: "spike report against the ADR-004 exit criteria"
    result: pass
    evidence: "2026-08-09 — codex-cli 0.146.1 and opencode 1.14.48 each repaired a deliberately broken test in an attempt worktree and signalled 0 through one argv shape, with no provider branch in Circus. Recorded in ADR-004. The spike also falsified the completion discriminator; see ADR-009."
```

```yaml
phase: 3
gates:
  - gate: "Every TEST entry implemented and green"
    mechanism: "cargo test"
    result: pass
    evidence: "2026-08-09 — 215 passed, 0 failed across 6 suites in 45s: 143 unit, 60 spec (TEST-001..060), 3 purity, 9 traceability"
  - gate: "A full-screen agent runs under a driver"
    mechanism: "manual: circus launch -- circus-driver-claude-tui against Claude Code 2.1.226"
    result: fail
    evidence: "2026-08-09 — the agent is SIGTTIN-stopped by withdone's job control before it renders anything. Diagnosis and the upstream fix are in ADR-004 and drivers/README.md. No requirement obliges Circus to host a TUI, so this fails an experiment rather than a gate; owner HOC."
  - gate: "The built binary runs the happy path end to end"
    mechanism: "manual: cargo build --release, then prepare/launch/accept/merge in a scratch repository"
    result: pass
    evidence: "2026-08-09 — completion_method sentinel, residue 0, transcript captured, decision accepted, one merge commit on main, the checkout left clean. A first run failed at exit 70 because the tmux socket path exceeded the platform limit; the attempt stayed `prepared` and both accept and merge refused it, which is the specified behaviour."
  - gate: "TEST entries correspond to specification entries, both directions"
    mechanism: "cargo test --test traceability"
    result: pass
    evidence: "every_specified_test_is_implemented and every_implemented_test_is_specified both pass; 34 atoms declared, 34 cited, none uncovered"
  - gate: "Simplicity: complexity lint clean"
    mechanism: "cargo clippy --all-targets -- -D warnings"
    result: pass
    evidence: "2026-08-09 — no issues. Three findings fixed rather than allowed: a collapsible branch, an over-long argument list on RunRecord::prepared, and an owned comparison."
  - gate: "Test coverage: mutation kill rate on the pure core"
    mechanism: "cargo mutants --file 'src/core/*.rs' -- --lib"
    result: pass
    evidence: "2026-08-09 — 178 mutants, 165 caught, 13 unviable, 0 missed (100% of viable). A first run missed 11; each was killed by a new test rather than by an exclusion, including the negative-era branch of civil_from_days and the 255-character ref bound."
  - gate: "Architecture: the purity dependency rule holds"
    mechanism: "cargo test --test purity"
    result: pass
    evidence: "core imports no shell module and performs no process, filesystem, or environment effect outside UnixSeconds::now"
  - gate: "Two-sided verification: prohibited-action and scope-invariant tests"
    mechanism: "cargo test --test spec"
    result: pass
    evidence: "prohibited-action: TEST-006, TEST-009, TEST-011, TEST-012, TEST-019, TEST-022, TEST-031. scope-invariant: TEST-002, TEST-010, TEST-017, TEST-018, TEST-029, TEST-032"
  - gate: "Behaviour invariant across calling context"
    mechanism: "cargo test --test spec (output captured to a pipe throughout)"
    result: pass
    evidence: "every spec test drives the binary with piped stdout and stderr; no branch in the source reads isatty"
  - gate: "Documentation: README and user documentation"
    mechanism: "tools/usdd-lint.sh --doc over docs/circus/**"
    result: pass
    evidence: "8 pages, 0 errors, 0 warnings; one page per mode declared in frontmatter; README exempt from the one-mode rule per PROTO-001"
  - gate: "Documentation prose conforms to the controlled language"
    mechanism: "tools/usdd-lint.sh over docs/circus/** and README.md"
    result: pass
    evidence: "0 errors across all 9 pages; 6 residual length warnings"
  - gate: "Security: no critical vulnerabilities"
    mechanism: "cargo audit"
    result: unverified
    evidence: "cargo-audit is not installed on this machine. Three direct dependencies (clap, serde, serde_json), all widely used, and the crate sets unsafe_code = forbid. Owner HOC."
  - gate: "Mutation testing of the merge and state modules"
    mechanism: "cargo mutants --file 'src/shell/git.rs' --file 'src/shell/state.rs' -- --lib"
    result: pass
    evidence: "2026-08-09 — 42 mutants, 35 caught, 6 unviable, 1 missed. The survivor is LockGuard::drop, an equivalent mutant: closing the descriptor releases the lock regardless, and the reason it stays is recorded at the call site. A first run missed 5; the other four were killed by tests for `resolves` and `attempt_exists`, both of which were genuinely untested pre-conditions."
  - gate: "Mutation testing of the remaining shell modules"
    mechanism: "cargo mutants --file 'src/shell/pane.rs' --file 'src/shell/proc.rs' --file 'src/shell/ui.rs'"
    result: unverified
    evidence: "pane.rs, proc.rs, and ui.rs are exercised mainly by the 41 integration tests, which take 30 s per run and make mutation prohibitively slow. Owner HOC."
```

## Status

The specification is `implemented`. Every contract has code, and every TEST
entry has a passing test of the same number. A test checks the traceability
between them, rather than prose asserting it.

Four validation gates remain open, and none of them was waived:

- The cross-model adversarial review of [[SPEC-001-circus-agent-harness#Tests]]
  and the code. One model family has reviewed both.
- The fresh-context comprehension check of the Orientation block.
- The synthetic-user simulation of [[users/lead-agent/happy-paths]].
- The steward's ruling on [[SPEC-001-circus-agent-harness#ADR-005]], and the
  driver spike's own exit criteria under
  [[SPEC-001-circus-agent-harness#ADR-004]].

Until those close, this specification records working, tested code. It does not
record a validated design.

## Changelog

<details>
<summary>Revision history — 0.1.0 → 0.7.0</summary>

- 0.7.0 — normative. Added
  [[SPEC-001-circus-agent-harness#REQ-014]] and
  [[SPEC-001-circus-agent-harness#REQ-015]].
  [[SPEC-001-circus-agent-harness#CON-003]] has always recorded the verifier's
  `output_path`, and a path is not an artefact — the file it named was usually
  in a temporary directory and gone by the next day, so
  [[SPEC-001-circus-agent-harness#NFR-001]].a promised an inspectable rejected
  attempt while its most useful part was a dangling pointer. Acceptance now
  copies that output beside the attempt, capped at 1 MiB, and records the
  truncation. [[SPEC-001-circus-agent-harness#CON-012]] adds
  `circus history`, a reader over the attempts
  [[SPEC-001-circus-agent-harness#NFR-001]].a already preserves: no store, no
  schema, no eviction. Added five tests.

- 0.6.0 — normative. Added three commands and the requirements behind them.
  [[SPEC-001-circus-agent-harness#REQ-011]] and
  [[SPEC-001-circus-agent-harness#CON-009]] give `circus status`, which reports
  an attempt as recorded *and* as observed, because a running attempt's record
  says `prepared` while an agent is working.
  [[SPEC-001-circus-agent-harness#REQ-012]] and
  [[SPEC-001-circus-agent-harness#CON-010]] give `circus logs`, which puts what
  the agent printed on stdout — `tmux attach` shows the same bytes but returns
  nothing a program can read.
  [[SPEC-001-circus-agent-harness#REQ-013]] and
  [[SPEC-001-circus-agent-harness#CON-011]] give `circus spawn`.
  [[SPEC-001-circus-agent-harness#REQ-003]].d was split, and its prohibition
  narrowed to provider-specific content, so that spawn's composition of task
  plus completion instruction is permitted while a prompt framework still is
  not. `launch` now writes the run record before it waits, which is what makes
  a running attempt answerable at all. Added
  [[SPEC-001-circus-agent-harness#OBS-005]] and seven tests.

- 0.5.0 — normative. Added [[SPEC-001-circus-agent-harness#REQ-010]] and
  [[SPEC-001-circus-agent-harness#CON-008]] for `circus instruction`, so the
  one piece of prompt wording every driver needs lives in one place instead of
  being rediscovered. Resolved
  [[SPEC-001-circus-agent-harness#ADR-004]] against codex and opencode. Added
  [[SPEC-001-circus-agent-harness#ADR-009]] after the spike falsified the
  completion discriminator: a non-interactive agent signals and then exits, so
  the child-exit marker won the race and a signalled attempt was recorded as
  `child-exit` with no sentinel value. The wrapper now reads the sentinel at
  the instant the child returns. Added three tests and four example drivers.

- 0.4.0 — normative. Added [[SPEC-001-circus-agent-harness#REQ-008]] for
  operator feedback and [[SPEC-001-circus-agent-harness#REQ-009]] for the merge
  preview, after an audit of the command line against the Command Line
  Interface Guidelines (https://clig.dev/). Added
  [[SPEC-001-circus-agent-harness#ADR-008]], which is the justification the
  contract template demands for styling stderr by terminal detection. Gave
  every command `--quiet`, `--verbose`, and `--no-color`, and `circus merge` a
  `--dry-run`. `prepare` now records and reports the sentinel path, because
  an operator assembling that path by hand gets it wrong on any platform where
  a temporary directory is a symlink. Added eight tests.

- 0.3.0 — implementation. Status moved to `implemented`. Added
  [[SPEC-001-circus-agent-harness#REQ-005]].f and
  [[SPEC-001-circus-agent-harness#REQ-005]].g with
  [[SPEC-001-circus-agent-harness#TEST-032]] and
  [[SPEC-001-circus-agent-harness#TEST-033]]. Implementation found that moving
  a ref a worktree has checked out makes every added path read as a deletion. Extended [[SPEC-001-circus-agent-harness#CON-004]] with the
  compare-and-swap and the checkout refresh. Added an Implementation section
  carrying the CODE link, and recorded in
  [[SPEC-001-circus-agent-harness#ADR-004]] what the driver contract settled.
  Added a phase 3 Gate Evidence Record. Documentation-only elsewhere.

- 0.2.0 — normative. Decomposed every REQ and NFR into lettered atoms so a
  failure attributes to one obligation. Added input grammars to [[SPEC-001-circus-agent-harness#CON-001]] through
  [[SPEC-001-circus-agent-harness#CON-004]]. Introduced [[SPEC-001-circus-agent-harness#CON-007]] for the run record, which now holds the single
  field list [[SPEC-001-circus-agent-harness#NFR-003]], [[SPEC-001-circus-agent-harness#OBS-002]], and [[SPEC-001-circus-agent-harness#OBS-003]] previously duplicated and
  contradicted. Rescoped [[SPEC-001-circus-agent-harness#REQ-006]] so the attempt log no longer conflicts with
  the invite-secret prohibition. Rewrote [[SPEC-001-circus-agent-harness#TEST-011]], which passed while a
  write landed in the default Elephant store. Added [[SPEC-001-circus-agent-harness#OBS-004]] to make the
  Elephant and composition prohibitions checkable. Bound the merge target to
  the recorded integration ref. Gave cleanup a 10 s bound as a temporal
  property. Defined the attempt identifier and the state root, and added
  [[SPEC-001-circus-agent-harness#ADR-005]] through [[SPEC-001-circus-agent-harness#ADR-007]]. Added seventeen tests, a Verification Strategy, and
  a Purity Boundary Map.
- 0.1.0 — initial draft.
</details>
