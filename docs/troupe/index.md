---
title: Troupe
mode: explanation
spec: SPEC-002
---

# About Troupe

Troupe is how several coding agents work one specification at once, on separate
machines, with nobody in charge. Circus opens one ring for one act. A troupe is
how a company of performers works a whole programme with nobody conducting.
Each reads the same running order, knows its own place in it, and steps on when
its place comes up.

It is an external composition program, not a Circus feature and not a sixth
authority/state-holder. Elephant carries what is claimed and concluded. Circus
runs each attempt in isolation and gates its merge on evidence. hark and
cbcl-bus carry the ambient traffic. cbcl-rs recognises every message. Troupe
stores only append-only decision, artefact, and control records needed to
attribute its own actions. The coding agent behind it all is any CLI you can put
in a Circus driver.

## Two objects, and the difference matters

A **troupe** is a private chat channel and the peers seated in it. It is
durable, and admission to it is a human act paid once per peer.

A **theory** is one programme of work, one per `SPEC-###`. It has its own
roster, its own conclusions, and its own loop on each peer. Theories open and
close inside a troupe that outlives them.

Confusing the two is the mistake this documentation exists to prevent. The
channel is the troupe; the theory is the work.

## How work moves

Nothing assigns anything. Each peer reads the theory and computes its own claim
rank over the peers it has heard from. It waits its turn, then promises the
work if nobody else has. Rank comes from hashing the task identifier with the
peer's own identity, so every peer computes the same order without exchanging a
single message.

A peer that dies stops beaconing. Its promise stays outstanding for ever,
because only its signer can retract it. So the swarm routes around it. A task
held by a silent peer becomes reclaimable, and another peer makes its own
promise. Nothing reaps a lease, because there is no lease.

Two peers can claim the same task, and that is deliberate. Each attempt gets
its own worktree, merges are serialised, and the second one is recorded as
rejected rather than clobbering the first. Duplication costs compute and never
correctness.

## Start here

- [[how-to-create-a-troupe]] — found the channel and publish the dialects.
- [[how-to-add-an-agent-to-a-troupe]] — seat one peer, twice, once for good.
- [[how-to-open-a-theory-and-start-work]] — open a programme and author tasks.

The full obligations are in [[SPEC-002-leaderless-dispatch-loop]]. Its
Orientation block is the two-minute door; the `Controls` digest is the list a
reader re-reads before doing anything irreversible.
