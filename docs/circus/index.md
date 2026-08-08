---
title: Circus
mode: explanation
spec: SPEC-001
---

# About Circus

Circus is a local harness for running coding-agent CLIs. It gives each attempt
its own Git worktree, hosts it in a named tmux pane, and learns that the work
finished because the worker said so rather than because a process ended.

It exists because three problems recur whenever a lead agent drives another
agent on one machine. Agents editing one working tree overwrite each other.
An agent's exit code says nothing about whether its work is correct. And an
agent that "looks done" is a guess until something explicit says otherwise.

Circus solves exactly those three, and stops. It is not a planner, a scheduler,
a daemon, a remote service, or an agent framework.

## Where to go next

- [Run your first attempt](tutorial/index.md) — the whole loop, once, start to
  finish.
- [How to recover a rejected attempt](how-to/how-to-recover-a-rejected-attempt.md)
  — what to do when the verifier says no.
- [How to resolve a merge conflict](how-to/how-to-resolve-a-merge-conflict.md)
  — Circus refuses the merge and leaves the work alone; you take it from there.
- [CLI reference](reference/cli.md) — every command, argument, and exit code.
- [Run record reference](reference/run-record.md) — every field Circus writes.
- [About the evidence gate](explanation/about-the-evidence-gate.md) — why an
  exit code cannot accept work, and what Circus does and does not guarantee.
- [About explicit completion](explanation/about-explicit-completion.md) — why
  there is a sentinel file at all.

## What Circus composes

| Program | Role |
|---|---|
| Git worktrees | Isolation. One checkout and one branch per attempt |
| tmux | A visible, attachable host for the agent process |
| withdone | The explicit completion channel |
| Elephant | Evidence and coordination — used by you, never by Circus |

Circus invokes the first three as external programs and records every
invocation. It never invokes the fourth.

## Conventions

Circus follows the [Command Line Interface Guidelines](https://clig.dev/).
Stdout carries data and stderr carries messaging, every command takes `-q`,
`-v`, and `--no-color`, colour respects `NO_COLOR`, and `circus merge` takes
`--dry-run`. Exit codes distinguish a bad argument from a failed tool from a
recorded "no".
