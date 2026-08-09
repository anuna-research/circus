# Circus

Circus runs a coding-agent CLI in an isolated Git worktree, tells you when it
actually finished, and refuses to call the result good on your behalf.

It is a small ringmaster. It opens a ring for each act, records the result, and
never decides whether the act was good.

## Install

```sh
curl https://files.anuna.io/circus/install.sh | sh
```

The installer detects your platform, downloads the matching prebuilt binary,
checks it against the published SHA-256, and installs it to `~/.local/bin`. No
Rust toolchain is involved. Set `CIRCUS_INSTALL_DIR` to install elsewhere.

Prebuilt binaries cover `darwin-arm64`, `darwin-x64`, `linux-x64`, and
`linux-arm64`. The flat prefix <https://files.anuna.io/circus/> is the latest
release. Every release also keeps an immutable copy at its own tag, such as
<https://files.anuna.io/circus/v0.1.1/>, and
<https://files.anuna.io/circus/version.json> names the current one.

Circus invokes Git, tmux, and withdone rather than reimplementing them, so all
three must be on `PATH`. The installer checks and names what is missing; it
does not install them, because a harness that quietly pulls in three
dependencies is not composing them. Git and tmux come from your package
manager. withdone is one POSIX shell script:

```sh
mkdir -p ~/.local/bin
curl -fsSL https://git.anuna.io/anuna-research/withdone/raw/branch/main/withdone \
    -o ~/.local/bin/withdone && chmod +x ~/.local/bin/withdone
```

`withdone --version` MUST print `withdone 3.0.1` or later. Earlier versions
reap nothing where `/bin/sh` is dash — Debian and Ubuntu among them — so an
attempt whose agent leaves a subprocess behind is failed by NFR-002 for
survivors withdone was supposed to have killed.

The example drivers are published alongside the binaries and are not installed
for you — see [Drivers](#drivers) for why:

```sh
curl -fsSL https://files.anuna.io/circus/drivers/circus-driver-codex \
    -o ~/.local/bin/circus-driver-codex \
    && chmod +x ~/.local/bin/circus-driver-codex
```

To build from source instead, see [Development](#development).

## Quick Start

```sh
# 1. Open a ring.
circus prepare --task model --integration main
# → prints a run record naming the attempt, its worktree, and its sentinel path

# 2. Build a prompt. Circus supplies the completion instruction; you supply
#    the task. Never assemble the sentinel path by hand.
echo "Add the model layer described in the issue. Run the tests." > task.md
{ cat task.md; circus instruction --attempt model/1; } > prompt.md

# 3. Run the agent in a named, attachable pane.
circus launch --attempt model/1 --prompt prompt.md -- circus-driver-codex

# 4. Verify it yourself, then record the decision.
cargo test > /tmp/v.txt; echo "{\"command\":[\"cargo\",\"test\"],\"exit_code\":$?,\"output_path\":\"/tmp/v.txt\"}" > v.json
circus accept --attempt model/1 --verifier-record v.json --evidence theory:spec-001/q42

# 5. Merge.
circus merge --attempt model/1 --into main
```

Or all of steps 1–3 at once, with the prompt composed for you:

```sh
circus spawn --task model --integration main --task-file task.md -- circus-driver-codex
```

While an attempt runs, from any other terminal:

```sh
circus status --attempt model/1              # is it alive, how long, how far
circus logs   --attempt model/1 -f --plain   # what the agent is saying
tmux attach   -t circus-model-1              # watch it live, interactively
```

## Usage

Circus has four commands and no daemon. Each one reads a run record, does one
thing, writes the record back, and prints it.

| Command | What it does |
|---|---|
| `prepare` | One worktree and one branch, from the integration ref |
| `launch` | One named tmux pane, running your driver through withdone |
| `accept` | Records a decision from your verifier output and your evidence |
| `merge` | Serialises one merge into the ref the attempt came from |
| `instruction` | Prints the completion instruction a prompt needs |
| `spawn` | `prepare` and `launch` in one, composing the prompt for you |
| `status` | What is true of an attempt now, recorded and live |
| `logs` | What the agent itself printed |
| `history` | Every attempt of a task, and what came of each |

Every command takes `-q`/`--quiet`, `-v`/`--verbose`, and `--no-color`.
`circus merge` takes `-n`/`--dry-run`. stdout carries the run record as JSON and
nothing else; progress, hints, and errors go to stderr. The CLI follows the
[Command Line Interface Guidelines](https://clig.dev/).

Three things Circus will not do, by design:

- **An exit code never means accepted.** A driver that exits 0 has told you
  about transport, not about correctness. `accept` requires a verifier record
  and at least one evidence reference before it records `accepted`.
- **It never touches Elephant.** No identity is created, no invite is parsed,
  and the `elephant` binary is never invoked. A worker that contributes
  independently already holds its own identity.
- **It never deletes an attempt.** A failed or rejected attempt keeps its
  worktree, branch, transcript, and record until you remove them.

## Drivers

A driver is any executable that runs one agent CLI. Circus has already opened
the worktree, minted the sentinel, allocated the terminal, and wrapped the
process in withdone, so a driver is usually one line:

```sh
#!/bin/sh
exec codex exec --dangerously-bypass-approvals-and-sandbox "$(cat "$CIRCUS_PROMPT")"
```

Four examples live in [`drivers/`](drivers/README.md). They are not installed
and the binary does not know they exist — `-- circus-driver-codex` is an
ordinary `PATH` lookup, which is the plugin system Unix already provides.

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

Prerequisites: Rust 1.89 or later, plus the Git, tmux, and
[withdone](https://git.anuna.io/anuna-research/withdone) that
[Install](#install) covers. The version floor on withdone applies here too.

To build and install from a checkout:

```sh
cargo install --path .
```

```sh
cargo test            # 217 tests: unit, spec suite, purity, traceability
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

Or through the Makefile:

```sh
make check        # tests + fmt + clippy
make mutants      # mutation-test the pure core
make spec         # zetl link check + controlled-language check
make docs-lint    # the same check across README, docs/, and drivers/
make dist         # stage dist/circus-<os>-<arch> + .sha256 for this platform
```

## Releasing

```sh
./release.sh 0.2.0
```

Bumps the version, runs the whole gate, commits, tags, and pushes. The tag
triggers `.forgejo/workflows/release.yaml`, which cross-compiles four binaries and
publishes them with `scripts/install.sh` and the example drivers to
<https://files.anuna.io/circus/>.

The gate runs here rather than in the release pipeline for a reason: the suite
drives real Git, tmux, and withdone, and the pipeline cross-compiles from Linux
containers to four targets, none of which can run a tmux pane. `.forgejo/workflows/ci.yaml`
runs it on every push; `release.sh` refuses to tag without it passing locally.

## License

Apache 2.0 — see [LICENSE](LICENSE).
