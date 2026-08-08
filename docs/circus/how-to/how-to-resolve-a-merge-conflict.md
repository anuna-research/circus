---
title: How to resolve a merge conflict
mode: how-to
spec: SPEC-001
---

# How to resolve a merge conflict

This guide covers a `circus merge` that reported a conflict, for a reader
comfortable with Git's own merge tools.

Circus computes the merge in the object database and never checks anything out.
On a conflict it changes nothing at all: the integration ref stays where it
was, and the attempt worktree is exactly as the agent left it. The resolution
is yours, and you do it with Git.

## Before you start

Confirm what Circus recorded:

```sh
grep -o '"merge":{[^}]*}' "$(git rev-parse --git-common-dir)/circus/model/1/record.json"
```

The terminal prints `"result":"conflict"`. The command's stderr also named the
conflicting paths when it ran.

## Steps

1. Go to the attempt worktree.

   ```sh
   cd "$(git rev-parse --git-common-dir)/../circus-model-1"
   ```

2. Bring the integration branch into the attempt, rather than the other way
   round. This keeps the conflict inside the ring.

   ```sh
   git merge main
   ```

3. Resolve the conflicting files and commit the merge.

   ```sh
   git mergetool
   git commit
   ```

4. Re-run your verifier against the resolved tree, and record the result.

5. Prepare a fresh attempt and accept it, because the attempt you already
   accepted describes a tree that no longer exists.

   ```sh
   circus prepare --task model --integration main
   ```

   Note that acceptance is recorded once per attempt. A resolved conflict is
   new work, and new work gets a new ring.

6. Merge the new attempt.

   ```sh
   circus merge --attempt model/2 --into main
   ```

## A different failure that looks similar

WHEN Circus exits 64 with a message about uncommitted changes, this is not a
conflict. The branch you are merging into is checked out somewhere, and that
checkout is dirty. Circus refuses before moving the ref, because moving a ref
under a dirty checkout makes every added file look deleted.

Commit or stash the change and run the merge again:

```sh
git -C /path/to/that/checkout stash
circus merge --attempt model/1 --into main
```

## Result

The integration branch carries one merge commit, and no intermediate state was
left behind by the refused attempt. Every attempt involved is still on disk
with its record intact.
