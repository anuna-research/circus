---
title: How to recover a rejected attempt
mode: how-to
spec: SPEC-001
---

# How to recover a rejected attempt

This guide covers what to do after `circus accept` records `rejected`, for a
reader who already runs the prepare-launch-accept loop.

Circus never deletes an attempt. Everything the worker produced is still on
disk, and the point of this guide is to read it before you spend another agent
run.

## Before you start

You need the attempt handle, such as `model/1`. If you have lost it, list the
attempts of a task:

```sh
ls "$(git rev-parse --git-common-dir)/circus/model"
```

## Steps

1. Read why the decision went the way it did.

   ```sh
   cat "$(git rev-parse --git-common-dir)/circus/model/1/record.json"
   ```

   The `verifier` field names the command that ran and its exit code. The
   `output_path` field points at what it printed.

2. Read what the agent did, in its own words.

   ```sh
   less "$(git rev-parse --git-common-dir)/circus/model/1/transcript.log"
   ```

3. Check how the attempt ended.

   WHEN `completion_method` is `child-exit`, the agent stopped without writing
   the sentinel. It gave up, crashed, or never understood the instruction.
   Re-read the prompt at `.../model/1/prompt` before blaming the agent.

4. Inspect the work itself.

   ```sh
   cd "$(git rev-parse --git-common-dir)/../circus-model-1"
   git log --oneline
   git diff main...HEAD
   ```

5. WHEN the work is salvageable, fix it in place and commit.

   ```sh
   git commit -am "address the verifier failure"
   ```

6. Re-run your verifier and record the new result.

   ```sh
   cargo test > /tmp/v.txt 2>&1
   printf '{"command":["cargo","test"],"exit_code":%s,"output_path":"/tmp/v.txt"}' $? > /tmp/v.json
   ```

7. WHEN the work is not salvageable, open a new ring instead. Attempts are
   immutable once launched, and a rejected one is evidence rather than a
   workspace.

   ```sh
   circus prepare --task model --integration main   # → model/2
   ```

8. Remove the old attempt only when you no longer need it.

   ```sh
   git worktree remove circus-model-1
   git branch -D circus/model/1
   rm -r "$(git rev-parse --git-common-dir)/circus/model/1"
   ```

## When the launch itself failed

WHEN `circus launch` exits 70, tmux or withdone failed before the attempt
began. The message names what went wrong. One cause worth knowing: tmux keeps
its socket under `TMUX_TMPDIR`, and a deep value there exceeds the platform's
limit on socket paths.

The attempt stays `prepared` in this case, so fix the environment and launch
the same attempt again. No new ring is needed.

## Result

You have either a corrected attempt with a fresh verifier record, or a new
attempt number and an old one kept for reference. A rejected attempt stays
inspectable until you remove it explicitly.

Note that `circus accept` cannot be re-run on the same attempt. Acceptance is
recorded once per attempt, by design: a decision that can be retried until it
comes out right is not a gate. Prepare a new attempt instead.
