# Circus drivers

A driver is any executable that runs one agent CLI. Circus has already opened
the worktree, minted the sentinel, allocated the terminal, and wrapped the
process in withdone, so a driver is usually one line.

These four are examples, not a dependency. Nothing in the Circus binary knows
they exist, and none of them is installed by `cargo install`. Copy the ones you
want:

```sh
cp drivers/circus-driver-* ~/.local/bin/
```

## The contract

Circus starts the driver with:

| | |
|---|---|
| Working directory | The attempt worktree |
| stdin, stdout, stderr | A real terminal — the tmux pane, on all three |
| `CIRCUS_PROMPT` | Absolute path to a byte-identical copy of the caller's prompt |
| `CIRCUS_SENTINEL` | The path the agent writes to signal completion |
| `CIRCUS_ATTEMPT` | The `task/attempt` handle |
| `CIRCUS_WORKTREE` | The worktree, same as the working directory |
| `TMUX_PANE` | The pane, for a driver that needs to type into itself |
| argv | Whatever the caller put after `--`, minus the driver name |

A driver runs the agent in the foreground and returns when it does. It does not
write the sentinel — that is the agent's job, and the instruction telling the
agent to do it comes from `circus instruction`.

## Using one

```sh
circus prepare --task model --integration main
{ cat task.md; circus instruction --attempt model/1; } > prompt.md
circus launch --attempt model/1 --prompt prompt.md -- circus-driver-codex
```

Arguments after the driver name reach the agent CLI:

```sh
circus launch --attempt model/1 --prompt prompt.md -- circus-driver-codex -m o3
```

## The four

| Driver | Agent | Billing | Notes |
|---|---|---|---|
| `circus-driver-codex` | `codex exec` | Codex subscription | Verified against codex-cli 0.146.1 |
| `circus-driver-opencode` | `opencode run` | Provider key | Verified against opencode 1.14.48 |
| `circus-driver-claude` | `claude --print` | Anthropic API | One-shot, non-interactive |
| `circus-driver-claude-tui` | `claude` TUI | Claude subscription | **Experimental — does not work yet, see below** |

Each passes the agent its "yes to everything" flag, without which the agent
stops at its first approval prompt and the attempt burns its whole budget
waiting for a keypress nobody will provide.

## Writing your own

```sh
#!/bin/sh
set -eu
exec my-agent --auto-approve "$@" "$(cat "$CIRCUS_PROMPT")"
```

That is the whole of it for any agent that takes its prompt on argv. An agent
that reads a file instead takes `"$CIRCUS_PROMPT"` directly, and one that reads
stdin takes `< "$CIRCUS_PROMPT"`.

## The TUI case — experimental, and currently blocked

`circus-driver-claude-tui` does not work. The diagnosis is precise enough to be
worth stating, because it is not the first place anyone looks.

**Keystroke injection is solved.** tmux can type into a pane it owns, which the
driver names through `$TMUX_PANE`:

```sh
tmux load-buffer -b "$buffer" "$CIRCUS_PROMPT"
tmux paste-buffer -p -d -b "$buffer" -t "$TMUX_PANE"
tmux send-keys -t "$TMUX_PANE" Enter
```

`paste-buffer -p` is a real bracketed paste, which is the input form a TUI
accepts, and the prompt is read straight from the file so nothing is quoted
through argv. No `expect` needed for this part.

**Job control is what blocks it.** withdone enables job control and backgrounds
the child, so the agent lands in a process group that is not the terminal's
foreground group. A full-screen agent reads the terminal as it starts, receives
`SIGTTIN`, and stops — permanently and silently. `ps` shows state `T`, the pane
stays blank, and the transcript is zero bytes.

Non-interactive agents never read the terminal, so `codex exec`,
`opencode run`, and `claude --print` are all unaffected. This is a TUI-only
failure.

It also corrects an assumption worth naming: the `expect` in the withdone
recipe is not only there to allocate a pseudo-terminal. It allocates a *second*
one, and the agent is the foreground process of that pty, so `SIGTTIN` never
fires. Removing `expect` because tmux already supplies a terminal removes the
part that was doing the real work.

The fix belongs upstream in withdone — hand the terminal to the child's process
group, or do not enable job control. Until then, use `circus-driver-claude` for
a non-interactive Claude.

A second obstacle sits behind the first. Claude asks whether it trusts the
directory, once per directory, and `--dangerously-skip-permissions` does not
cover that prompt. Every Circus attempt is a fresh worktree, so it fires every
time. The driver sends an Enter to answer it, which is untested past the stop.

One heuristic remains even after both are fixed: the settle delay. A TUI
redraws several times after entering the alt-screen, and a paste that lands
before it has settled is lost. `CIRCUS_TUI_SETTLE` sets it, defaulting to 4
seconds.

If your terminal is left in an odd state after a TUI attempt, run `reset`. The
agent is SIGKILLed by withdone and does not get to restore the modes it set.

## What belongs here, and what does not

These scripts hold provider knowledge on purpose, and they hold it *outside*
the binary. `SPEC-001-circus-agent-harness#REQ-007.b` forbids Circus itself
from carrying provider-specific prompt injection, and the boundary is
checkable: delete this directory and Circus behaves identically for any
caller-supplied driver.

Circus does not discover these by name. `-- circus-driver-codex` is an ordinary
`PATH` lookup, which is the plugin system Unix already provides.

Related: the [withdone recipes](https://git.anuna.io/anuna-research/withdone)
solve the same problem one layer down, for callers who are not using Circus.
They also mint the sentinel, allocate the PTY, and run withdone, because
nothing else is doing it for them.
