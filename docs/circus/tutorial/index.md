---
title: Run your first attempt
mode: tutorial
spec: SPEC-001
---

# Run your first attempt

We are going to take one task through the whole loop. We will open a ring,
watch an agent work in it, verify the result ourselves, and merge it.

The agent in this lesson is a two-line shell script. Once you have seen the
shape, swap it for a real coding CLI and nothing else changes.

## Before you start

You need Git, tmux, withdone, and Circus on your `PATH`. Check them:

```sh
git --version && tmux -V && withdone --version && circus --version
```

WHEN `withdone` is missing, install it. It is one POSIX shell script:

```sh
curl -fsSL https://git.anuna.io/anuna-research/withdone/raw/branch/main/withdone \
    -o ~/.local/bin/withdone && chmod +x ~/.local/bin/withdone
```

Work in a scratch repository, not one you care about:

```sh
mkdir /tmp/ring && cd /tmp/ring
git init -b main .
echo "hello" > greeting.txt
git add -A && git commit -m "base"
```

## Step 1 — Open a ring

```sh
circus prepare --task greeter --integration main
```

Circus prints a run record. Three fields matter now:

```json
{
  "attempt_id": "greeter/1",
  "worktree_path": "/tmp/circus-greeter-1",
  "branch": "circus/greeter/1"
}
```

`greeter/1` is the handle every later command takes. The worktree is a sibling
of your checkout, not a directory inside it.

Notice that your own repository is untouched. `git status` in `/tmp/ring`
reports nothing.

## Step 2 — Find the sentinel path

Circus already printed it. Look at the stderr from step 1:

```
circus: prepared greeter/1
  worktree:  /tmp/circus-greeter-1
  branch:    circus/greeter/1
  the prompt must contain this path, exactly:
    /private/tmp/ring/.git/circus/greeter/1/sentinel
  next: circus launch --attempt greeter/1 --prompt <file> -- <driver>
```

It is in the record too, as `sentinel_path`. Copy it from one of those two
places rather than assembling it yourself. Notice `/private/tmp` above: on
macOS `/tmp` is a symlink, and a path you type by hand does not match the one
Circus will use.

The file does not exist yet. The worker creates it, once, when the work is
done.

## Step 3 — Write the prompt

The prompt MUST contain that path literally. Circus refuses to launch
otherwise, and it refuses before starting any process.

```sh
S=$(python3 -c "import json,sys;print(json.load(sys.stdin)['sentinel_path'])" \
     < /tmp/ring/.git/circus/greeter/1/record.json)
cat > /tmp/prompt.md <<EOF
Change greeting.txt to say "hello, ring".
When the change is made and you have recorded your evidence, run:
  echo 0 > $S
EOF
```

Notice that you wrote the instruction, not Circus. Circus reads this file only
to check that one path appears in it, and passes every byte through unchanged.

## Step 4 — Run the agent

Our agent is a script standing in for a real CLI:

```sh
cat > /tmp/agent.sh <<'EOF'
#!/bin/sh
echo "working in $PWD"
echo "hello, ring" > greeting.txt
git add -A && git commit -q -m "greet the ring"
echo 0 > "$CIRCUS_SENTINEL"
sleep 60
EOF
chmod +x /tmp/agent.sh

circus launch --attempt greeter/1 --prompt /tmp/prompt.md -- /tmp/agent.sh
```

The command blocks while the agent runs, and tells you how to watch it:

```
circus: circus-greeter-1 is running
  watch it:  tmux attach -t circus-greeter-1
  transcript: /tmp/ring/.git/circus/greeter/1/transcript.log
circus: running for 4s
```

Open that command in another terminal to look over the agent's shoulder.
Interrupting `circus launch` with Ctrl-C does not stop the agent — the pane
belongs to the tmux server, and it keeps working.

Notice the `sleep 60` at the end of the script, and notice that `launch`
returns long before a minute has passed. The sentinel ended the attempt, not
the process. That is the whole point of the sentinel.

When it returns, the record reports how it finished:

```sh
grep -o '"completion_method":[^,]*' /tmp/ring/.git/circus/greeter/1/record.json
```

The terminal prints `"completion_method": "sentinel"`.

## Step 5 — Verify it yourself

Circus has no opinion about whether the work is good. You form one:

```sh
cd /tmp/circus-greeter-1
cat greeting.txt        # prints: hello, ring
grep -q "hello, ring" greeting.txt > /tmp/verifier.txt 2>&1
echo "{\"command\":[\"grep\",\"-q\",\"hello, ring\",\"greeting.txt\"],\
\"exit_code\":$?,\"output_path\":\"/tmp/verifier.txt\"}" > /tmp/v.json
cd /tmp/ring
```

## Step 6 — Record the decision

```sh
circus accept --attempt greeter/1 \
  --verifier-record /tmp/v.json \
  --evidence theory:tutorial/q1
```

The record now carries `"decision": "accepted"`.

Try it without the evidence reference and Circus exits 64 instead. Both halves
are required: a passing verifier and at least one reference.

## Step 7 — Merge

```sh
circus merge --attempt greeter/1 --into main
cat greeting.txt
```

The terminal prints `hello, ring`. Your checkout was brought in line with the
merge commit, so `git status` is clean.

## What you have

One merge commit on `main`, and a complete record of how it got there:

```sh
cat /tmp/ring/.git/circus/greeter/1/record.json
```

The attempt's worktree, branch, and transcript are all still on disk. Circus
never removes them. When you are finished with this one:

```sh
git worktree remove /tmp/circus-greeter-1
git branch -D circus/greeter/1
```

Next: [How to recover a rejected attempt](../how-to/how-to-recover-a-rejected-attempt.md).
