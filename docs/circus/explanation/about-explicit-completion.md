---
title: About explicit completion
mode: explanation
spec: SPEC-001
---

# About explicit completion

Why there is a sentinel file, and why Circus will not launch without seeing its
path in your prompt.

## Knowing when an agent has finished

A coding agent is not a build. It prints, pauses, thinks, prints again, and
sometimes sits idle for minutes in the middle of useful work. Three ways of
guessing when it has finished all fail:

- **Wait for the process to exit.** Many agent CLIs are interactive and never
  exit on their own. Those that do often exit long after their useful work,
  or immediately after printing a plan they never carried out.
- **Watch for silence.** A timeout on output turns a thinking agent into a
  killed one, and the threshold that avoids it is long enough to waste hours.
- **Parse the output.** Now you have a provider-specific parser at a trust
  boundary, and it breaks whenever the model's phrasing drifts.

Each replaces the worker's knowledge of its own state with the harness's guess.

## The sentinel

Circus mints one fresh path per attempt and hands it to `withdone`, which
watches for it. The worker writes an integer there when, and only when, the
work is done and its evidence is recorded. withdone sees the write and ends the
process group.

The channel is explicit and cheap. One `echo 0 > $CIRCUS_SENTINEL` is within
reach of any agent that can run a shell command, and it carries the one bit no
external observer can infer: *I consider this finished.*

## Why the prompt must name the path

Circus refuses to launch a prompt that does not contain the sentinel path
literally. Without that check the common failure is silent: an agent that was
never told about the sentinel runs to its own conclusion, exits, and the
attempt is recorded as `child-exit` with nothing to distinguish it from a
crash.

The check is a literal substring search and nothing more. That restraint is the
point. Circus does not template the prompt, does not append instructions, and
does not know which provider you are running. Composing the instruction that
tells the worker *when* to write is your job; verifying that one path made it
into the file is Circus's.

## What the record tells you afterwards

withdone's exit code is the sentinel value on one path and the child's own code
on the other, and afterwards the two are indistinguishable. Circus therefore
wraps the driver in a script that records the child's own exit. Its absence is
what identifies the sentinel path, because withdone terminates the group before
the wrapper can write.

The result is a `completion_method` you can act on:

| Value | What happened | What it usually means |
|---|---|---|
| `sentinel` | The worker signalled | The agent believes it finished |
| `child-exit` | The driver returned first | It gave up, crashed, or never saw the instruction |

A `child-exit` attempt is worth reading the prompt for before you blame the
agent.

## Cleanup, and who does it

withdone ends the process group when the sentinel appears. tmux sends SIGHUP to
the same group when the pane dies. Circus does neither — it measures, and
records `process_group_residue`. A process that survives both reapers within
10 s makes the attempt `failed`, because a harness that leaves stray processes
behind is worse than one that says so.

Decisions: ADR-001. Requirements: REQ-003, NFR-002. Signals: OBS-003.
