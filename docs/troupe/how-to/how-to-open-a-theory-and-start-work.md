---
title: How to open a theory and start work
mode: how-to
spec: SPEC-002
---

# How to open a theory and start work

This guide covers opening one programme of work in an existing troupe, for a
reader with an approved `SPEC-###` and a decomposed plan.

One theory per specification. A troupe runs several at once, one `troupe run`
per theory per peer — [[SPEC-002-leaderless-dispatch-loop#ADR-013]].

## Before you start

You need a specification whose requirements, contracts, and tests are approved.
A task that cannot be completed on independently observable evidence is a
planning defect, not a task. Every task needs four things before it becomes
work: a statement, a `TEST-###`-derived acceptance criterion, a readiness rule,
and that rule's governing source.

## Steps

1. Create the theory, on any peer's machine.

   ```sh
   elephant theory create spec-014
   ```

2. Seed this specification's vocabulary and rules, before anyone else joins.

   ```sh
   troupe seed --config ~/.config/troupe/spec-014.toml --dry-run
   troupe seed --config ~/.config/troupe/spec-014.toml
   ```

   Seeding is idempotent, so a late run repairs a theory. It cannot un-coin a
   predicate a peer already invented, which is why it comes before the joins.

3. Author each task. A task identifier is an argument, never a name suffix.

   ```sh
   elephant assert '(given (task models))' -t spec-014
   elephant assert '(given (task-description models "Add the model layer"))' -t spec-014
   elephant assert '(given (task-acceptance models "TEST-042: model contract checks pass"))' -t spec-014
   elephant assert '(given (no-deps models))' -t spec-014
   elephant assert '(normally r-ready-models (and (task models) (no-deps models)) (ready models))' -t spec-014
   elephant assert '(meta r-ready-models (source "SPEC-014-user-authentication#REQ-042"))' -t spec-014
   ```

   A task with a prerequisite names it in the readiness rule instead of
   `no-deps`:

   ```sh
   elephant assert '(normally r-ready-api (and (task api) (completed models)) (ready api))' -t spec-014
   ```

4. Check that the theory offers the work rather than withholding it.

   ```sh
   elephant next -t spec-014 --json
   ```

   A withheld candidate names its own gap — a missing description, acceptance,
   readiness rule, or source. Repair the gap; do not work around it.

5. Let the other peers in with a human at each terminal. A current theory member
   runs `elephant theory invite`; the joiner runs `elephant theory join`; the
   humans pass the SPAKE2 phrase out of band. Do not send the phrase through an
   agent, a transcript, a room frame, a Troupe record, or a CBCL message.

   ```sh
   # human-operated terminals only; never run by Troupe
   elephant theory invite <spec-id>
   elephant theory join <spec-id>

   # afterwards, on the joiner's peer
   troupe status --config ~/.config/troupe/spec-014.toml
   ```

   The command exits 0 only once this peer is a roster member, its policy is
   compatible, and it is fit to claim.

6. Confirm every peer reached the same conclusions.

   ```sh
   elephant closure fingerprint -t spec-014
   ```

   The digest MUST agree across peers. A mismatch means sync has not converged,
   or a peer applied a local trust policy the others did not.

7. Start the loop on each peer.

   ```sh
   troupe run --config ~/.config/troupe/spec-014.toml
   ```

   Under launchd or a systemd user service, supervise this command. It is a
   foreground process and never backgrounds itself.

## Result

Every peer beacons, reads the theory, computes its own rank, and claims what it
ranks first for. Watch the work from any terminal:

```sh
elephant status -t spec-014
elephant commitments -t spec-014
troupe decisions --since 2026-08-10T00:00:00Z
```

## How to drive one attempt by hand

Skip the loop entirely and run a single pass:

```sh
troupe once --config ~/.config/troupe/spec-014.toml --task models
```

One decision record on stdout, one exit code, and no process to stop
afterwards.

## How to stop the troupe

Send a halt into the channel from any member:

```sh
hark emit '(swarm-halt :id "deploy-20260810-1" :by "@you" :reason "deploying the hub")'
hark emit '(swarm-resume :halt "deploy-20260810-1" :by "@you")'
```

A halt withholds every new claim. It does not terminate an attempt already
running, because killing an attempt mid-write leaves a worktree nobody can
read. Troupe journals the halt locally, so it remains in force across a restart;
only the same wire-verified signer can resume that named halt. It is still
best-effort: a peer partitioned from the hub never sees it.
