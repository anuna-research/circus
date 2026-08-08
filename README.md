# Circus

Circus runs a coding-agent CLI in an isolated Git worktree, tells you when it
actually finished, and refuses to call the result good on your behalf.

It is a small ringmaster. It opens a ring for each act, records the result, and
never decides whether the act was good.

## Quick Start

```sh
cargo install --path .

# 1. Open a ring.
circus prepare --task model --integration main
# → prints a run record naming the attempt, its worktree, and its sentinel path

# 2. Write a prompt containing that literal sentinel path.
cat > prompt.md <<'EOF'
Add the model layer described in the issue. Run the tests.
Assert your evidence to Elephant. Only then: echo 0 > /repo/.git/circus/model/1/sentinel
EOF

# 3. Run the agent in a named, attachable pane.
circus launch --attempt model/1 --prompt prompt.md -- my-agent-cli

# 4. Verify it yourself, then record the decision.
cargo test > /tmp/v.txt; echo "{\"command\":[\"cargo\",\"test\"],\"exit_code\":$?,\"output_path\":\"/tmp/v.txt\"}" > v.json
circus accept --attempt model/1 --verifier-record v.json --evidence theory:spec-001/q42

# 5. Merge.
circus merge --attempt model/1 --into main
```

Watch a running attempt with `tmux attach -t circus-model-1`.

## Usage

Circus has four commands and no daemon. Each one reads a run record, does one
thing, writes the record back, and prints it.

| Command | What it does |
|---|---|
| `prepare` | One worktree and one branch, from the integration ref |
| `launch` | One named tmux pane, running your driver through withdone |
| `accept` | Records a decision from your verifier output and your evidence |
| `merge` | Serialises one merge into the ref the attempt came from |

Three things Circus will not do, by design:

- **An exit code never means accepted.** A driver that exits 0 has told you
  about transport, not about correctness. `accept` requires a verifier record
  and at least one evidence reference before it records `accepted`.
- **It never touches Elephant.** No identity is created, no invite is parsed,
  and the `elephant` binary is never invoked. A worker that contributes
  independently already holds its own identity.
- **It never deletes an attempt.** A failed or rejected attempt keeps its
  worktree, branch, transcript, and record until you remove them.

Full guides: [docs/circus/](docs/circus/index.md).

## Architecture

Circus composes Git, tmux, and withdone. It implements none of them.

```
  ┌────────────┐  prepare   ┌──────────────────┐
  │ lead agent │ ─────────▶ │ Git worktree     │
  │            │  launch    └──────────────────┘
  │            │ ──────┐
  │            │       ▼
  │            │  ┌──────────┐  ┌──────────┐  ┌───────────┐
  │            │  │ tmux pane│─▶│ withdone │─▶│ agent CLI │
  │            │  └──────────┘  └──────────┘  └───────────┘
  │            │  accept · merge · run record
  └────────────┘
```

The source follows a purity boundary:

- `src/core/` — deterministic and side-effect free. Input recognisers, path
  derivation, the run-record codec, the acceptance predicate.
- `src/shell/` — every effect. Git, tmux, withdone, the filesystem, the lock.

Dependencies point inward. `core` never imports `shell`, and
`tests/purity.rs` fails the build if that changes.

Design decisions live in the specification: composition over reimplementation
(ADR-001), evidence before acceptance (ADR-002), no automatic peer enrolment
(ADR-003), the driver contract as a governed spike (ADR-004), why merge
coordination sits here (ADR-005), and the attempt identity and state root
(ADR-006). See [specs/SPEC-001-circus-agent-harness.md](specs/SPEC-001-circus-agent-harness.md).

## API Reference

- CLI: [docs/circus/reference/cli.md](docs/circus/reference/cli.md)
- Run record schema: [docs/circus/reference/run-record.md](docs/circus/reference/run-record.md)

Contracts CON-001 through CON-007 in the specification are the normative
source; the reference pages restate them for a reader at work.

## Development

Prerequisites: Rust 1.89 or later, Git, tmux, and
[withdone](https://files.anuna.io/withdone/) on `PATH`.

```sh
cargo test            # 129 tests: unit, spec suite, purity, traceability
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The test suite runs against real Git, real tmux, and the real withdone. Each
test isolates its tmux server through `TMUX_TMPDIR`, so a run never disturbs
your own session.

`tests/spec.rs` carries one test per TEST entry in the specification, named for
it. `tests/traceability.rs` checks mechanically that every requirement atom
reaches a test and that every test attributes an atom, so the traceability
claim is decided rather than asserted.

Specification hygiene:

```sh
zetl check --dead-links --fail-on error
```
