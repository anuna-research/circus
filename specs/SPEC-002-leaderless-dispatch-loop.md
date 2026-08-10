---
id: SPEC-002
title: Troupe — Leaderless Dispatch Loop
status: draft
version: 0.2.0
tier: 2
audience: agent, human
author: Anuna Research (drafted with Claude Opus 5)
last-updated: 2026-08-10
owner-repo: circus
affects-repos: elephant, hark, cbcl-bus, cbcl-rs
review-gate: not-approved (Tier 2 — cross-model adversarial review outstanding)
---

# SPEC-002: Troupe — Leaderless Dispatch Loop

The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT,
RECOMMENDED, MAY, and OPTIONAL in this document are to be interpreted as
described in BCP 14 (RFC 2119, RFC 8174) when, and only when, they appear in
all capitals.

## Orientation

Intent: Let several coding agents work one specification at once, on separate
machines, with no coordinator — so that what is claimed, running, done, and
abandoned is derived from signed evidence rather than announced by a supervisor.

Metaphor: a troupe. [[concepts/circus|Circus]] opens one ring for one act. A
troupe is how a company of performers works a whole programme with nobody
conducting: each performer reads the same running order, knows their own place
in it, and steps on when their place comes up.

Structure:

```
  cbcl-bus ──▶ hark ──────┐  recognised CBCL, R1–R5   (CON-003)
  signed wire  daemon     │
                          ▼
  Elephant theory ──────▶ ┌───────────────────────────────┐
  signed · defeasible     │  PURE CORE                    │
  (CON-002)               │ elect(corpus, peers, self,   │  CON-001
                          │       clock)                  │
                          │  rank · filter · hold · skip  │
  Circus run record ────▶ └──────────────┬────────────────┘
  attempt evidence                       │ one decision
  (CON-004)                              ▼
                          ┌───────────────────────────────┐
  LLM CLI driver ───────▶ │  EFFECTFUL SHELL              │
  opaque executable       │  beacon · promise · spawn ·   │
  (CON-004)               │  verify · accept · assert ·   │
                          │  merge                        │
                          └───────────────────────────────┘
    arrows point inward → the core imports nothing and calls nothing
```

Decisions: [[SPEC-002-leaderless-dispatch-loop#ADR-001]] two coordination
planes · [[SPEC-002-leaderless-dispatch-loop#ADR-002]] a composition, not a
component · [[SPEC-002-leaderless-dispatch-loop#ADR-003]] the window is sized
to the channel · [[SPEC-002-leaderless-dispatch-loop#ADR-004]] duplication is
survivable, not prevented ·
[[SPEC-002-leaderless-dispatch-loop#ADR-005]] rendezvous hashing over random
backoff · [[SPEC-002-leaderless-dispatch-loop#ADR-006]] the coding agent is not
a theory member · [[SPEC-002-leaderless-dispatch-loop#ADR-007]] flat gate
literals · [[SPEC-002-leaderless-dispatch-loop#ADR-008]] where this
specification lives · [[SPEC-002-leaderless-dispatch-loop#ADR-009]] one
foreground process, one pass per invocation ·
[[SPEC-002-leaderless-dispatch-loop#ADR-010]] initialisation is asymmetric,
operation is not · [[SPEC-002-leaderless-dispatch-loop#ADR-011]] the troupe is
the channel, a theory is one programme ·
[[SPEC-002-leaderless-dispatch-loop#ADR-012]] theory admission remains human ·
[[SPEC-002-leaderless-dispatch-loop#ADR-013]] per-theory loops share local
capacity · [[SPEC-002-leaderless-dispatch-loop#ADR-014]] an attempt, not a task,
is the evidence subject.

Load-bearing: [[SPEC-002-leaderless-dispatch-loop#REQ-001]] uniform procedure ·
[[SPEC-002-leaderless-dispatch-loop#REQ-002]] local contention ordering ·
[[SPEC-002-leaderless-dispatch-loop#REQ-004]] reclamation ·
[[SPEC-002-leaderless-dispatch-loop#REQ-006]] worker containment ·
[[SPEC-002-leaderless-dispatch-loop#CON-001]] the election function every other
requirement reads.

Controls:

- [[SPEC-002-leaderless-dispatch-loop#REQ-001]].c a claim SHALL NOT depend on
  another peer's grant, lease, or acknowledgement.
- [[SPEC-002-leaderless-dispatch-loop#REQ-004]].c a peer SHALL NOT retract or
  defeat another peer's commitment.
- [[SPEC-002-leaderless-dispatch-loop#REQ-005]].b no CBCL parser, tokeniser, or
  S-expression reader in the loop.
- [[SPEC-002-leaderless-dispatch-loop#REQ-006]].a no theory identity, wire key,
  or channel handle reaches a coding agent.
- [[SPEC-002-leaderless-dispatch-loop#REQ-006]].b no literal derived from a
  coding agent's own report of its work.
- [[SPEC-002-leaderless-dispatch-loop#REQ-007]].b a peer SHALL NOT approve its
  own attempt.
- [[SPEC-002-leaderless-dispatch-loop#REQ-008]].c theory unreachable ⇒ no new
  claims.
- [[SPEC-002-leaderless-dispatch-loop#REQ-010]] a halt withholds every new claim
  until a resume arrives.
- [[SPEC-002-leaderless-dispatch-loop#REQ-011]].b no evidence literal from an
  untrusted-external message.
- [[SPEC-002-leaderless-dispatch-loop#REQ-013]].b no claim acted on before its
  decision record is written.
- [[SPEC-002-leaderless-dispatch-loop#REQ-014]].d no command reimplements what a
  composed program already exposes.
- [[SPEC-002-leaderless-dispatch-loop#REQ-015]].b no decision reads which peer
  founded the theory.
- [[SPEC-002-leaderless-dispatch-loop#REQ-016]].c `seed` writes no task,
  promise, or evidence literal.
- [[SPEC-002-leaderless-dispatch-loop#REQ-017]].b a beacon from off *this*
  theory's roster never enters its live peer set.
- [[SPEC-002-leaderless-dispatch-loop#REQ-017]].d Troupe admits no member to the
  channel.
- [[SPEC-002-leaderless-dispatch-loop#REQ-017]].e Troupe removes no member from
  either plane.
- [[SPEC-002-leaderless-dispatch-loop#REQ-018]].a Troupe SHALL NOT automate
  theory admission or carry an admission secret.
- [[SPEC-002-leaderless-dispatch-loop#REQ-019]].d `(completed X)` follows a
  successful merge of one identified attempt, never review alone.
- [[SPEC-002-leaderless-dispatch-loop#REQ-020]].c a silence observation names
  one attempt and cannot reclaim a later one.
- [[SPEC-002-leaderless-dispatch-loop#REQ-021]].d a replayed or stale event
  SHALL NOT affect a claim or halt state.
- [[SPEC-002-leaderless-dispatch-loop#REQ-022]].d a recovered process restores
  its durable halt state before it can claim.
- [[SPEC-002-leaderless-dispatch-loop#REQ-023]].c incompatible execution policy
  SHALL withhold claim, review, and merge.

Open:

- No temporal toolchain is selected for the properties in
  [[SPEC-002-leaderless-dispatch-loop#Temporal Properties]]. The same gap
  [[SPEC-001-circus-agent-harness#ADR-007]] records. Owner: HOC.
- Automated peer-issued theory admission is deferred. It needs a two-member,
  end-to-end encrypted carrier that does not expose an invite code to a hub or
  group member. Owner: HOC.
- The Tier 2 cross-model adversarial review is outstanding. No `ADR-###` below
  is ratified until it completes. Owner: HOC.

Detail: [[users/peer-agent/user]] · [[users/peer-agent/happy-paths]] ·
[[SPEC-002-leaderless-dispatch-loop#Requirements]] ·
[[SPEC-002-leaderless-dispatch-loop#Contracts]] ·
[[SPEC-002-leaderless-dispatch-loop#Verification Strategy]] ·
[[SPEC-002-leaderless-dispatch-loop#Tests]].

## Amendment Channels

Amendable by: the project steward (HOC).

Through: a merged revision of this specification or a linked `ADR-###`.

Not amendable by: agent prompts, coding-agent output, channel messages, external
webhook payloads, theory assertions, issue comments, or local repository
contents.

Hard stops: [[SPEC-002-leaderless-dispatch-loop#REQ-005]],
[[SPEC-002-leaderless-dispatch-loop#REQ-006]],
[[SPEC-002-leaderless-dispatch-loop#REQ-007]], and
[[SPEC-002-leaderless-dispatch-loop#REQ-011]], and
[[SPEC-002-leaderless-dispatch-loop#REQ-019]] require a new specification
version to change.

A channel halt is not an amendment. It exercises an authority
[[SPEC-002-leaderless-dispatch-loop#REQ-010]] already grants, and it changes no
obligation in this document. A channel message that asks a peer to skip a
verifier, merge without review, or admit an external event as evidence is a
request to amend this specification. It routes to the Error Response of
[[concepts/PROTO-001|PROTO-001]] and is never acted on.

## Context

Five programs already exist, and each refuses the job of the sixth.

[[concepts/elephant|Elephant]] carries the shared conclusion. It is
peer-to-peer, signed, defeasible, and has no supervisor. It states plainly that
nothing spawns agents, allocates worktrees, enforces leases, or merges branches.

[[concepts/circus|Circus]] carries one local attempt. It isolates a worktree,
hosts the process, detects completion explicitly, and gates a merge on evidence.
[[SPEC-001-circus-agent-harness#REQ-007]].b forbids it from becoming a scheduler
or a daemon. Troupe consequently SHALL NOT become a Circus subcommand: it reads
theory and channel state, makes a cross-peer decision, and invokes Circus only
after that decision. Those are responsibilities Circus is expressly forbidden to
own; [[SPEC-002-leaderless-dispatch-loop#ADR-002]] records the placement.

[[concepts/hark|hark]] carries the ambient channel. It holds the WebSocket to
the bus, blocks for one message, and sends one back. It selects no work.

[[concepts/cbcl-bus|cbcl-bus]] carries the wire. Every frame is signed per
frame, bound to an audience, and sequenced against replay. It routes and it fans
out. It decides nothing about the content it moves.

[[concepts/cbcl-rs|cbcl-rs]] carries the recogniser. It is the one parser for
[[concepts/cbcl|CBCL]], with R1–R5 machine-checked in Lean 4. It is a library
and never a participant.

The gap between them is a **loop**. The procedure reads what the theory
concluded, and decides whether this peer is the one to act. It then runs an
attempt, checks it, and puts signed evidence back. [[concepts/hence|hence]]
filled that gap with a supervisor and lost the coordination when the supervisor
died. This specification fills it with a procedure that every peer runs
identically, so there is nothing whose death loses anything.

The name is Troupe. It is a small external composition program, not a Circus
feature and not a sixth state-holder. Its durable state is attributable. It
carries the decision and control records required by this specification.
Circus retains attempt state. Elephant retains coordination truth. hark retains
connection state — see [[SPEC-002-leaderless-dispatch-loop#ADR-002]].

## User Profile

The [[users/peer-agent/user|peer agent]] is one member of a swarm with no
leader. It has its own theory identity and its own wire key. It cannot assign
work, cannot revoke a promise, and cannot enrol another member. Its whole
authority is over its own machine.

## Happy Path

Recorded in full, with failure modes and synthetic-user findings, at
[[users/peer-agent/happy-paths]]. In outline:

1. The peer beacons, and reads the beacons of others.
2. The peer reads the theory for available and for reclaimable work.
3. The peer computes its own rank and waits its turn.
4. The peer acquires local capacity, writes its decision record, announces
   intent, then promises one identified attempt.
5. The peer runs a coding agent through Circus, in isolation.
6. The peer runs the acceptance verifier itself.
7. The peer asserts attempt-scoped verifier evidence, merges only the exact
   attempt a different peer approved, and records completion only after Circus
   reports that merge successful.

## Requirements

Each requirement is decomposed into lettered atoms. One atom carries one BCP 14
keyword, so a failure attributes to a single obligation.

### REQ-001: Uniform Peer Procedure

- **REQ-001.a** — Every peer SHALL execute the same dispatch procedure.
- **REQ-001.b** — The procedure SHALL NOT accept a parameter that distinguishes
  one peer's role from another's.
- **REQ-001.c** — A peer's claim SHALL NOT depend on a grant, lease, or
  acknowledgement from another peer.
- **REQ-001.d** — The cross-peer election decision SHALL be derived from the
  theory, the channel, and the peer's own identity alone. A local capacity
  refusal MAY convert an elected claim into `Skip(capacity)` without changing
  any other peer's election.

This is the requirement the whole design exists to satisfy, and it is the one
most easily lost by accident. Three things reintroduce the leader by a side
door. A configuration key named
`role`. A peer that seeds the theory, and therefore knows something others do
not. A message whose absence blocks a claim. A per-host capacity slot is not a
counterexample: it cannot alter another peer's rank, grant work, or survive the
process that owns it.
[[SPEC-002-leaderless-dispatch-loop#TEST-002]] and
[[SPEC-002-leaderless-dispatch-loop#TEST-003]] exist to catch that.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-001]]
- [[SPEC-002-leaderless-dispatch-loop#CON-007]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-001]] (a — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-002]] (b — negative-input)
- [[SPEC-002-leaderless-dispatch-loop#TEST-003]] (c — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#OBS-001]]

### REQ-002: Local Contention Ordering

- **REQ-002.a** — A peer SHALL compute a claim rank per candidate task, from the
  task identifier, its own identity, and the live peer set.
- **REQ-002.b** — The rank SHALL be the peer's position in the descending order
  of `H(task-id ‖ peer-did)` over the live peer set, counting from zero.
- **REQ-002.c** — Rank computation SHALL exchange no message.
- **REQ-002.d** — A peer SHALL delay its claim by `rank × contention-window`.
- **REQ-002.e** — Immediately before promising, a peer SHALL re-read the theory
  and the channel.
- **REQ-002.f** — A peer SHALL NOT promise a candidate that carries an
  outstanding commitment or an unexpired intent from another peer.

`H` is SHA-256, already a dependency of every program in the composition.
[[concepts/rendezvous-hashing|Rendezvous hashing]] gives three properties, and
needs no message exchange for any of them. Every peer computes the same order.
The order redistributes minimally when a peer leaves. Different tasks give
different peers rank zero. See [[SPEC-002-leaderless-dispatch-loop#ADR-005]].

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-001]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-004]] (a, b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-005]] (b — property: agreement)
- [[SPEC-002-leaderless-dispatch-loop#TEST-006]] (b — property: minimal
  disruption)
- [[SPEC-002-leaderless-dispatch-loop#TEST-007]] (c — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-008]] (f — negative-input)
- [[SPEC-002-leaderless-dispatch-loop#OBS-001]]
- [[SPEC-002-leaderless-dispatch-loop#OBS-003]]

### REQ-003: Liveness Beacon

- **REQ-003.a** — A peer SHALL emit a beacon into the swarm channel at least
  once per beacon period.
- **REQ-003.b** — A beacon SHALL carry the peer's theory DID, outstanding
  attempt identifiers, process instance, sequence number, and policy
  fingerprint.
- **REQ-003.c** — A peer SHALL treat a peer whose last beacon is older than the
  silence threshold at local receipt time as silent.
- **REQ-003.d** — A peer SHALL continue beaconing while an attempt runs.

REQ-003.d is what separates a working peer from a dead one. A peer that stopped
beaconing because it was busy gets reclaimed out from under itself.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-003]]
- [[SPEC-002-leaderless-dispatch-loop#NFR-002]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-009]] (a, d — positive, temporal)
- [[SPEC-002-leaderless-dispatch-loop#TEST-010]] (c — positive)
- [[SPEC-002-leaderless-dispatch-loop#OBS-002]]

### REQ-004: Reclamation of an Abandoned Attempt

- **REQ-004.a** — WHEN a peer observes the owner of attempt A silent, the peer
  SHALL assert `(attempt-silent A T)`, where T is its local observation time.
- **REQ-004.b** — A peer SHALL discover reclaimable work through the
  `(reclaimable X A)` literal, naming both task X and abandoned attempt A.
- **REQ-004.c** — A peer SHALL NOT retract, defeat, or otherwise remove another
  peer's commitment.
- **REQ-004.d** — A reclaiming peer SHALL make its own, new attempt B on task X.
- **REQ-004.e** — A peer SHALL NOT assert `(attempt-silent A T)` while the
  channel is unreachable.
- **REQ-004.f** — A silence observation for A SHALL NOT make any later attempt
  B reclaimable.

The second discovery path exists because the first one cannot be repaired.
`elephant next` withholds a task carrying an outstanding `(completed X)`
commitment, and `elephant retract` accepts only the signer's own statement. A
peer that dies mid-attempt therefore hides its task from every peer's `next`
output for ever. No living peer has the authority to release it. Reclamation
routes around the withholding rather than trying to undo it. The dead peer's
promise stays outstanding and visible, which is the honest record of what
happened. A permanent silence fact names the abandoned attempt. A bare
peer-level silence fact makes every future task that peer attempts reclaimable.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-002]]
- [[SPEC-002-leaderless-dispatch-loop#NFR-006]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-011]] (a, b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-012]] (c — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-013]] (e — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-014]] (d, f — scope-invariant)
- [[SPEC-002-leaderless-dispatch-loop#OBS-004]]

### REQ-005: Recognition Before Action

- **REQ-005.a** — A peer SHALL act on a channel message only after hark reported
  it recognised under R1–R5.
- **REQ-005.b** — A peer SHALL NOT implement a CBCL parser, tokeniser, or
  S-expression reader.
- **REQ-005.c** — A peer SHALL invoke Elephant using the token arrays
  `elephant next --json` returns.
- **REQ-005.d** — A peer SHALL discard a channel message whose swarm-dialect
  shape it cannot resolve.

REQ-005.b is Constitutional Principle 14's one-recogniser-per-language rule
applied to a composition. [[concepts/cbcl-rs|cbcl-rs]] is that recogniser, its
R1–R5 invariants are machine-checked, and hark already runs it at both
boundaries. A second reader in the loop is a parser-differential bug waiting
for an attacker. REQ-005.c closes the same hole on the other input. The
returned token arrays are argument vectors, so no quoting is reconstructed and
no shell metacharacter is interpreted.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-003]]
- [[SPEC-002-leaderless-dispatch-loop#CON-005]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-015]] (a — negative-input)
- [[SPEC-002-leaderless-dispatch-loop#TEST-016]] (b — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-017]] (c — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-018]] (d — negative-input)

### REQ-006: Worker Containment

- **REQ-006.a** — A peer SHALL NOT pass its theory identity, its wire key, or
  its channel handle to a coding agent.
- **REQ-006.b** — A peer SHALL NOT assert a literal derived from a coding
  agent's own report of its work.
- **REQ-006.c** — A peer SHALL assert only literals its own verifier run or its
  own inspection established.
- **REQ-006.d** — A peer SHALL run every coding agent inside a Circus attempt
  worktree.

A coding agent is an untrusted producer of a diff. It is not a member of
anything. The theory is end-to-end encrypted, and its roster is
cryptographically closed. An agent holding a peer's identity therefore signs
statements the whole swarm treats as that peer's own. Circus already refuses to
carry an Elephant identity ([[SPEC-001-circus-agent-harness#REQ-006]]); this
requirement refuses to hand one to the process Circus starts. See
[[SPEC-002-leaderless-dispatch-loop#ADR-006]].

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-004]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-019]] (a — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-020]] (b — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-021]] (c — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-022]] (a — scope-invariant)
- [[SPEC-002-leaderless-dispatch-loop#OBS-007]]

### REQ-007: Review Separation

- **REQ-007.a** — A peer SHALL assert `(attempted X A P)` naming task X,
  attempt A, and itself when it claims.
- **REQ-007.b** — A peer SHALL NOT assert `(review-approved A P D)` where P
  names itself and D is the reviewed tree digest.
- **REQ-007.c** — The theory SHALL defeat `(verified A)` when the reviewer and
  the attempter of A unify.
- **REQ-007.d** — A peer SHALL NOT merge attempt A unless `(merge-authorized A)`
  is derivable.
- **REQ-007.e** — A reviewer SHALL bind its approval to the exact tree digest
  Troupe recorded for A from the named Circus worktree.

REQ-007.c is what makes REQ-007.b more than an honour system. The reviewer is
carried in the literal, and unified against the attempting peer. Self-review is
therefore unrepresentable as a path to `verified`, rather than merely
forbidden. The attempt and tree digest prevent an approval of one duplicate
worktree being used to merge another. The rule is in
[[SPEC-002-leaderless-dispatch-loop#CON-002]].

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-002]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-023]] (a, c, e — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-024]] (c — negative-output)
- [[SPEC-002-leaderless-dispatch-loop#TEST-025]] (b — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-026]] (d — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#OBS-005]]

### REQ-008: Degraded Modes

- **REQ-008.a** — WHEN the channel is unreachable, a peer SHALL use the theory
  roster as its live peer set.
- **REQ-008.b** — WHEN the channel is unreachable, a peer SHALL widen its
  contention window to the theory's sync interval.
- **REQ-008.c** — WHEN the theory is unreachable, a peer SHALL withhold every
  new claim.
- **REQ-008.d** — A peer SHALL record every entry into and exit from a degraded
  mode.

The two planes fail differently and the loop degrades differently for each. The
channel is advisory, so losing it costs speed. The roster still names every
member, and a wider window restores safety at the price of latency. The theory
is authoritative, so losing it costs the right to claim at all. A peer that
cannot read commitments cannot know it is alone.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-001]]
- [[SPEC-002-leaderless-dispatch-loop#NFR-003]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-027]] (a, b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-028]] (c — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-029]] (d — positive)
- [[SPEC-002-leaderless-dispatch-loop#OBS-006]]

### REQ-009: Bounded Work In Flight

- **REQ-009.a** — A peer SHALL hold at most `max-in-flight` outstanding promises
  at once.
- **REQ-009.b** — WHEN a peer is at its limit, the peer SHALL skip the claim and
  continue beaconing.
- **REQ-009.c** — `max-in-flight` SHALL default to 1.

The default is 1 because of the dominant profile in
[[users/peer-agent/happy-paths]]. That profile is one developer machine,
running one coding agent against one worktree. A second concurrent attempt
competes for the same CPU, the same rate limit, and the same operator's
attention. A peer with
spare capacity raises it; the common path does not configure it.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-007]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-030]] (a, b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-031]] (a — negative-output)
- [[SPEC-002-leaderless-dispatch-loop#OBS-001]]

### REQ-010: Human Halt

WHEN a halt H arrives on the swarm channel from a channel member, the peer SHALL
withhold every new claim. The peer SHALL record H durably with its wire-verified
signer, and report its in-flight tasks to the channel WITHIN one beacon period.
New claims SHALL remain withheld until a resume naming H arrives from that same
signer. A halt is troupe-wide: it applies to every theory carried by its channel.

- **REQ-010.a** — A halt SHALL NOT terminate an in-flight attempt.
- **REQ-010.b** — A peer SHALL record H and its signer in its durable control
  journal before it next elects work.
- **REQ-010.c** — A resume SHALL name H and carry the same wire-verified signer.
- **REQ-010.d** — A peer SHALL restore its durable control journal before it
  issues a claim after restart.
- **REQ-010.e** — A halt or resume replay SHALL NOT change the control state.

A halt stops the swarm taking on more. It does not kill work already running,
because killing an attempt mid-write is how a worktree becomes evidence nobody
can read. REQ-010.b exists because a halt is an authority claim. An authority
claim with no attributable signer is a denial-of-service vector, on a channel
anyone in the room can write to. A transient channel event cannot safely mean
"until resume" after a process restart, so the control journal is the small
durable state this procedure needs. It records control only; it never becomes a
source of work, evidence, or merge authority.

The halt is best-effort. A partitioned peer never receives it and keeps
claiming. This specification states that rather than promising a guarantee the
transport cannot make.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-003]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-032]] (positive: withheld, reported,
  and journalled)
- [[SPEC-002-leaderless-dispatch-loop#TEST-033]] (prohibited-action: no claim
  after halt or restart)
- [[SPEC-002-leaderless-dispatch-loop#TEST-034]] (a — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-035]] (b — positive)
- [[SPEC-002-leaderless-dispatch-loop#OBS-008]]

### REQ-011: External Event Admission

- **REQ-011.a** — A peer SHALL admit a message tagged untrusted-external only as
  a discovery literal.
- **REQ-011.b** — A peer SHALL NOT assert an evidence literal derived from a
  message tagged untrusted-external.
- **REQ-011.c** — A peer SHALL record the source tag of every external message
  it admits.

[[concepts/cbcl-bus|cbcl-bus]] mints an external webhook into a CBCL message on
the source's behalf and marks it untrusted-external. That marking is the whole
control, and it is only a control if the loop honours it. A CI webhook able to
assert `(ci-green X)` lets anyone holding the webhook secret drive the
theory to `completed` and a merge to land. A CI webhook that asserts
`(discovered ci-reported-green-X)` prompts a peer to run the verifier itself,
which is what evidence means here.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-005]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-036]] (a — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-037]] (b — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-038]] (b — scope-invariant)
- [[SPEC-002-leaderless-dispatch-loop#TEST-039]] (c — positive)
- [[SPEC-002-leaderless-dispatch-loop#OBS-009]]

### REQ-012: Driver Neutrality

- **REQ-012.a** — A peer SHALL invoke the coding agent through a Circus driver
  named by configuration.
- **REQ-012.b** — A peer SHALL NOT embed provider-specific prompt text, flags,
  or environment variables.
- **REQ-012.c** — A peer SHALL compose the prompt from the candidate's
  description, its acceptance criterion, and `circus instruction` output.

This mirrors [[SPEC-001-circus-agent-harness#REQ-007]].a one layer out. Circus
keeps provider-specific invocation in the driver, and the loop keeps it there
too. So "any LLM CLI agent" stays a property of the design, rather than a list
that needs maintaining.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-004]]
- [[SPEC-002-leaderless-dispatch-loop#CON-007]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-040]] (a, c — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-041]] (b — prohibited-action)

### REQ-013: Externalised Decision Record

- **REQ-013.a** — Before promising, a peer SHALL write a durable decision
  record. It carries the candidate, proposed attempt, its rank, the live peer
  set, policy fingerprint, and the theory read that justified the claim.
- **REQ-013.b** — A peer SHALL NOT act on a candidate whose decision record it
  has not written.
- **REQ-013.c** — A peer SHALL cite the decision record identifier in the
  evidence it asserts.

This is the countermeasure PROTO-001's Externalised Verification Record
prescribes, applied to the one decision this loop exists to make. A claim
justified only inside an agent's reasoning is unattributable. The measured
failure mode is an agent that retrieves every fact it needs, and then argues
itself out of the finding. Writing the record before the promise makes the
ordering checkable from the artefacts rather than from a narration.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-006]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-042]] (a, c — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-043]] (b — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-044]] (a — temporal ordering)
- [[SPEC-002-leaderless-dispatch-loop#OBS-001]]

### REQ-014: Operator Feedback and Composability

- **REQ-014.a** — A peer SHALL write its primary output to stdout, and nothing
  else to stdout.
- **REQ-014.b** — A peer SHALL write every human-facing message to stderr.
- **REQ-014.c** — A peer SHALL produce the same output regardless of whether
  stdout is a terminal, a pipe, or a file.
- **REQ-014.d** — A peer SHALL NOT reimplement a capability one of the five
  composed programs already exposes on the command line.
- **REQ-014.e** — A peer SHALL exit non-zero WHEN it is not fit to claim work.

REQ-014.c is Constitutional Principle 2's calling-context rule, and it is the
one an operator notices. A loop whose event stream reformats itself for a human
watcher produces two different traces from one run. The temporal properties are
stated over the trace.

REQ-014.d is the Simplicity Ladder as an obligation rather than a habit. A human
halts the swarm with `hark emit`, reads an attempt with `circus logs`, and asks
the theory a question with `elephant explain`. Each already exists, each is
already recognised, and a wrapper around it is a second surface to keep
correct.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-008]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-057]] (a, b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-058]] (c — positive, three contexts)
- [[SPEC-002-leaderless-dispatch-loop#TEST-059]] (d — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-060]] (e — negative-output)

### REQ-015: Founding Confers No Operating Authority

- **REQ-015.a** — The dispatch loop SHALL treat the founding peer exactly as it
  treats every other peer.
- **REQ-015.b** — A peer SHALL NOT read, derive, or act on which peer founded
  the theory.
- **REQ-015.c** — A peer SHALL NOT hold a configuration value that names a
  founder, a coordinator, or a primary.

[[SPEC-002-leaderless-dispatch-loop#REQ-001]] governs the loop. This
requirement governs the seam between the loop and the ceremony that precedes
it, which is where a leader most plausibly re-enters. The founder is first in
time and nothing else. It cannot assign work, revoke a promise, or retract
another peer's statement. Elephant makes all three impossible rather than
discouraged.

One asymmetry does survive, and it is named rather than denied. The **steward**
— the member holding the theory — can `elephant theory remove` another member,
which rotates the corpus key. That is a membership power, not a dispatch power.
It changes who is in the room and never who does the work, so no requirement in
this specification reads it. See
[[SPEC-002-leaderless-dispatch-loop#ADR-010]].

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-009]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-067]] (a — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-068]] (b — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-069]] (c — negative-input)

### REQ-016: Seeding Precedes Admission and Repeats Safely

- **REQ-016.a** — The vocabulary and rules of
  [[SPEC-002-leaderless-dispatch-loop#CON-002]] SHALL be present in the theory
  before any peer beyond the founder joins.
- **REQ-016.b** — `seed` SHALL write only the families and rules the theory does
  not already carry.
- **REQ-016.c** — `seed` SHALL NOT write a task, a promise, or an evidence
  literal.
- **REQ-016.d** — A peer SHALL refuse to claim WHEN the theory is missing a rule
  its own decision depends on.

REQ-016.b matters because a theory is append-only. Re-running a naive seed does
not overwrite a rule. It adds a second copy, and a reader cannot tell an
intentional duplicate from a repeated ceremony. REQ-016.d is the safety net for
REQ-016.a. A peer that joined a half-seeded theory refuses rather than
improvises. That routes to the Error Response of
[[concepts/PROTO-001|PROTO-001]], instead of coining a private vocabulary
nobody listens for.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-002]]
- [[SPEC-002-leaderless-dispatch-loop#CON-009]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-070]] (b — scope-invariant)
- [[SPEC-002-leaderless-dispatch-loop#TEST-071]] (c — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-072]] (d — negative-output)

### REQ-017: Roster Correspondence

- **REQ-017.a** — A theory's roster SHALL be a subset of the channel roster.
- **REQ-017.b** — A peer SHALL exclude from a theory's live peer set every
  beacon whose DID is absent from **that theory's** roster.
- **REQ-017.c** — A peer SHALL report a DID observed under more than one wire
  key, or a wire key observed carrying more than one DID.
- **REQ-017.d** — Troupe SHALL NOT admit a member to the channel.
- **REQ-017.e** — Troupe SHALL NOT remove a member from either plane.

REQ-017.b is the atom that carries weight, and its absence was a defect. A rank
is a position in an ordering over the live peer set
([[SPEC-002-leaderless-dispatch-loop#REQ-002]].b), so **anyone able to beacon
can move everyone's rank**. One channel carries every theory the troupe works,
so most beacons a peer sees are for theories it is not in. The filter is that
theory's roster, and it is cryptographically closed.

The subset is deliberate rather than sloppy. The channel is the troupe and
outlives any one theory; a theory is one work programme with its own roster —
see [[SPEC-002-leaderless-dispatch-loop#ADR-011]]. Equality forces every peer
into every theory, which is the thing a subset exists to avoid.

REQ-017.c is the countermeasure to a sockpuppet. The self-review defeater of
[[SPEC-002-leaderless-dispatch-loop#REQ-007]] unifies on the reviewer's symbol.
A party holding two DIDs therefore attempts as one and approves as the other,
and Constitutional Principle 12 collapses. Every beacon is signed by a wire key
and names a DID, so the binding between them is observable. A one-to-many
binding in either direction is a finding rather than a curiosity.

REQ-017.d and .e keep membership outside dispatch. Both channel and theory
admission remain human ceremonies under [[SPEC-002-leaderless-dispatch-loop#REQ-018]].
Removal stays with the steward on both planes, because it rotates a corpus key.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-001]]
- [[SPEC-002-leaderless-dispatch-loop#CON-009]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-076]] (b — negative-input)
- [[SPEC-002-leaderless-dispatch-loop#TEST-077]] (b — scope-invariant)
- [[SPEC-002-leaderless-dispatch-loop#TEST-078]] (c — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-079]] (d, e — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#OBS-002]]
- [[SPEC-002-leaderless-dispatch-loop#OBS-010]]

### REQ-018: Peer-Issued Theory Admission

- **REQ-018.a** — A peer SHALL admit a requester to a theory only WHEN that
  requester is already a channel member.
- **REQ-018.b** — Exactly one peer SHALL answer a given `join-request`, chosen
  by the ranking of [[SPEC-002-leaderless-dispatch-loop#REQ-002]].b over the
  requester's DID.
- **REQ-018.c** — A peer SHALL send a `join-grant` only on the routed
  transport, addressed to the requester.
- **REQ-018.d** — A peer SHALL NOT write an invite code into a channel frame, a
  decision record, a journal record, a log line, or a theory assertion.
- **REQ-018.e** — An invite code SHALL carry a time-to-live of at most 120 s.
- **REQ-018.f** — A peer SHALL NOT admit a requester to the channel, and SHALL
  NOT remove a member from either plane.

Channel membership is the trust root, and theory membership decides whose
signed approvals a programme believes. REQ-018.a keeps the first human and
automates only the second, so the boundary that matters is still crossed by a
person.

REQ-018.b reuses the election the loop already performs, so admission needs no
coordinator and no new mechanism. `elephant theory invite` blocks until one
joiner completes the exchange. N peers answering one request therefore leave
N − 1 processes hanging until their codes expire.

REQ-018.c and .e together bound an exposure they cannot remove. Neither carrier
in this composition is clean. The encrypted room is group-visible, so a code
posted there lets a current member mint a second DID and defeat
[[SPEC-002-leaderless-dispatch-loop#REQ-007]]. The routed transport is
point-to-point but signed rather than encrypted, so its hub reads the frame.
The routed transport is chosen because its exposure has bounds. A single-use
code, a 120 s window, and the troupe's own deployment. The room's exposure has
none.
[[SPEC-002-leaderless-dispatch-loop#ADR-012]] records the trade in full, and
[[SPEC-002-leaderless-dispatch-loop#REQ-017]].c is the detection that survives
a failure of all three bounds.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-003]]
- [[SPEC-002-leaderless-dispatch-loop#CON-009]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-080]] (a — negative-input)
- [[SPEC-002-leaderless-dispatch-loop#TEST-081]] (b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-082]] (c — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-083]] (d — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#TEST-099]] (d — scope-invariant)
- [[SPEC-002-leaderless-dispatch-loop#TEST-100]] (e — negative-input)
- [[SPEC-002-leaderless-dispatch-loop#TEST-101]] (f — prohibited-action)
- [[SPEC-002-leaderless-dispatch-loop#OBS-011]]

### REQ-019: Attempt-Scoped Completion

- **REQ-019.a** — An attempt identifier SHALL be the pair `(theory, circus
  attempt-id)` and SHALL be unique within the theory.
- **REQ-019.b** — Every attempt, verifier, review, merge-authorisation, and
  merge-success literal SHALL name the same attempt identifier.
- **REQ-019.c** — A verifier result and review approval SHALL carry the Git tree
  digest Troupe recorded for that attempt's named Circus worktree.
- **REQ-019.d** — The theory SHALL derive `(completed X)` only from a successful
  merge of an identified attempt of X.
- **REQ-019.e** — A failed or rejected merge SHALL NOT derive `(completed X)` or
  authorise a different attempt.

The task is a planning object; an attempt is the immutable artefact that was
actually verified, reviewed, and merged. Treating them as the same object lets
an approval of one duplicate worktree authorise another, or lets a merge
conflict report a task complete. [[SPEC-002-leaderless-dispatch-loop#CON-002]]
keeps the theory's lifecycle at the attempt boundary and derives task completion
only at the final integration transition.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-002]]
- [[SPEC-002-leaderless-dispatch-loop#CON-004]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-088]] (a, b, c — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-089]] (d, e — negative-output)

### REQ-020: Replay-Safe Coordination Events

- **REQ-020.a** — Every `claim-intent` SHALL carry a decision identifier, an
  attempt identifier, a policy fingerprint, and an expiry.
- **REQ-020.b** — Every beacon SHALL carry a peer-instance identifier and a
  monotonically increasing sequence number.
- **REQ-020.c** — A peer SHALL use local receipt time to determine event
  freshness. The peer SHALL process a duplicate identifier idempotently. The
  peer SHALL discard an expired or superseded event.
- **REQ-020.d** — An event that fails those checks SHALL NOT affect rank,
  eligibility, or control state.

The wire's signature and sequence prevent frame forgery and replay at the bus
boundary. They do not define the application lifetime of a claim intent after a
peer restarts or a delayed frame arrives. The application fields here make the
`unexpired intent` condition of [[SPEC-002-leaderless-dispatch-loop#REQ-002]].f
decidable without trusting another peer's clock.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-003]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-090]] (a, b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-091]] (c, d — negative-input)

### REQ-021: Compatible Execution Policy

- **REQ-021.a** — Every peer SHALL compute a policy fingerprint from the theory
  identifier, specification revision, dialect revision, repository identity,
  integration ref, and verifier argument vector.
- **REQ-021.b** — A beacon and decision record SHALL carry that fingerprint.
- **REQ-021.c** — A peer observing a roster member with a different fingerprint
  SHALL report the mismatch and SHALL withhold claim, review, and merge.

The verifier and integration ref are otherwise merely local configuration, even
though they decide what evidence can complete shared work. A policy fingerprint
makes that hidden disagreement visible before a permissive verifier or a
different integration branch can produce an incomparable approval.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-006]]
- [[SPEC-002-leaderless-dispatch-loop#CON-007]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-092]] (a, b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-093]] (c — prohibited-action)

### REQ-022: Host Capacity Across Theories

- **REQ-022.a** — A peer SHALL acquire one local execution-capacity slot before
  it promises an attempt.
- **REQ-022.b** — Slots SHALL be shared by all Troupe processes on the same
  execution host, not scoped to one theory.
- **REQ-022.c** — A peer that cannot acquire a slot SHALL return `Skip(capacity)`
  and SHALL NOT promise.
- **REQ-022.d** — A slot SHALL be released when Circus reaches a terminal
  attempt state or its owning process exits.

`max-in-flight` remains a per-peer upper bound, but a peer can participate in
several theories. A capacity slot prevents three per-theory loops from quietly
running three coding agents on one developer machine. It is a local native
advisory lock, not a cross-peer scheduler and not a Circus responsibility.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#CON-007]]
- [[SPEC-002-leaderless-dispatch-loop#CON-008]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-094]] (a, b, d — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-095]] (c — prohibited-action)

### REQ-023: Review Assurance Boundary

- **REQ-023.a** — This specification SHALL treat a distinct reviewer DID as a
  distinct credential, not proof of a distinct human, model family, operator, or
  organisation.
- **REQ-023.b** — A peer SHALL report, but SHALL NOT silently repair, a DID to
  wire-key binding anomaly.
- **REQ-023.c** — A programme requiring cross-model or organisationally
  independent review SHALL establish that assurance during human admission,
  outside Troupe's election procedure.

The self-review defeater proves credential separation. It cannot prove cognitive
or organisational independence when one operator legitimately controls multiple
keys. Troupe is leaderless dispatch under governed membership; it is not
Byzantine consensus or leaderless membership governance.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#REQ-017]]
- [[SPEC-002-leaderless-dispatch-loop#TEST-078]] (b — positive)
- [[SPEC-002-leaderless-dispatch-loop#TEST-096]] (a, c — review)

## Non-Functional Requirements

### NFR-001: Claim Latency

Claim latency is measured from a candidate becoming visible to the rank-zero
peer, until that peer's promise is written. It SHALL be ≤ 2 s UNDER a live peer
set of ≤ 8 WITH 95th percentile.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#TEST-045]]
- [[SPEC-002-leaderless-dispatch-loop#OBS-001]]

### NFR-002: Beacon Period and Silence Threshold

Beacon period SHALL be 15 s WITH jitter of ±20 %. Silence threshold SHALL be
45 s, which is three beacon periods at the widest jitter.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#TEST-009]]
- [[SPEC-002-leaderless-dispatch-loop#OBS-002]]

### NFR-003: Contention Window

Contention window SHALL be 1 s UNDER a reachable channel, and SHALL widen to the
theory's configured sync interval UNDER an unreachable channel.

The 1 s figure is sized to the channel round trip, not the theory. A promise
reaches other peers only after an Elephant sync interval, whose default is 30
s. A window sized to that makes the rank-three peer of a six-peer swarm wait
more than two minutes. See [[SPEC-002-leaderless-dispatch-loop#ADR-003]].

Trace:

- [[SPEC-002-leaderless-dispatch-loop#TEST-046]]
- [[SPEC-002-leaderless-dispatch-loop#OBS-003]]

### NFR-004: Duplicate Claim Rate

A duplicate claim is one issued against a task another peer already holds live.
Duplicate claims SHALL be ≤ 1 % of all claims UNDER a reachable channel and a
converged theory WITH a 24 h measurement window.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#TEST-047]]
- [[SPEC-002-leaderless-dispatch-loop#OBS-003]]

### NFR-005: Loop Overhead

Wall-clock time spent in the loop's own commands SHALL be ≤ 5 % of an attempt's
total wall clock WITH median over 20 attempts.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#TEST-048]]

### NFR-006: Reclaim Latency

Elapsed time from the silence threshold expiring until a live peer promises the
reclaimable task SHALL be ≤ 90 s WITH 95th percentile. Measure only while the
channel and theory are reachable, no halt is active, and a compatible peer has
capacity. A halt, a partition, or no eligible capacity is an explicit
withholding condition, not a latency failure.

Trace:

- [[SPEC-002-leaderless-dispatch-loop#TEST-049]]
- [[SPEC-002-leaderless-dispatch-loop#OBS-004]]

## Contracts

### CON-001: The Election Function

Interface: `elect(corpus, peers, self, clock) -> Decision`

This is the pure core. It is a deterministic function of its arguments. It
performs no I/O, reads no clock, and calls no program. The evaluation time is a
parameter, exactly as it is for Elephant's own closure.

Pre-conditions:

- `corpus` is the set of conclusions and commitments read from one theory read.
- `peers` is the observed live peer set, each entry a DID, a last-beacon time,
  and the policy fingerprint the shell has already found compatible.
- `self` is this peer's DID, and `self ∈ peers`.
- `clock` is the evaluation time.

Post-conditions, one clause per implemented requirement:

- `Decision` is exactly one of `Claim(task, rank, window)`, `Skip(reason)`, or
  `Idle` — [[SPEC-002-leaderless-dispatch-loop#REQ-001]].d
- `rank` is the position of `self` in the descending order of
  `H(task ‖ peer)` over `peers` — [[SPEC-002-leaderless-dispatch-loop#REQ-002]].b
- `Claim` is returned only when the task carries no outstanding commitment and
  no unexpired intent — [[SPEC-002-leaderless-dispatch-loop#REQ-002]].f
- `Claim` is returned only when `self` holds fewer than `max-in-flight`
  outstanding promises — [[SPEC-002-leaderless-dispatch-loop#REQ-009]].a
- `Skip(halted)` is returned while a halt is recorded and no later resume is —
  [[SPEC-002-leaderless-dispatch-loop#REQ-010]]
- `Skip(theory-unreachable)` is returned when the corpus read failed —
  [[SPEC-002-leaderless-dispatch-loop#REQ-008]].c
- The same arguments SHALL produce the same `Decision` on every peer and on
  every replay.

The shell acquires the local capacity slot after `Claim` and before the promise.
Failure to acquire it yields `Skip(capacity)` without re-running or altering the
pure election. Compatibility filtering likewise occurs before the `PeerSet`
crosses this boundary. These local refusals preserve leaderless election: they
cannot grant work, change a remote rank, or authorise a merge.

Error model: the function is total. An unreadable corpus is a `Skip`, never an
exception. There is no error return, because there is no I/O to fail.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-001]],
[[SPEC-002-leaderless-dispatch-loop#REQ-002]],
[[SPEC-002-leaderless-dispatch-loop#REQ-008]],
[[SPEC-002-leaderless-dispatch-loop#REQ-009]],
[[SPEC-002-leaderless-dispatch-loop#REQ-010]],
[[SPEC-002-leaderless-dispatch-loop#REQ-021]],
[[SPEC-002-leaderless-dispatch-loop#REQ-022]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-004]],
[[SPEC-002-leaderless-dispatch-loop#TEST-005]],
[[SPEC-002-leaderless-dispatch-loop#TEST-006]],
[[SPEC-002-leaderless-dispatch-loop#TEST-030]],
[[SPEC-002-leaderless-dispatch-loop#TEST-050]].

### CON-002: The Swarm Theory Vocabulary

Interface: one Elephant theory per governing `SPEC-###`.

Input grammar: SPL, as recognised by Elephant. This contract declares which
families the loop coins, and which rules consume them. Every coined family has a
consumer, and every rule carries a `source`.

Reserved built-ins used as-is, never redefined: `task`, `ready`, `completed`,
`verified`, `failed`, `blocked-by`, `discovered`, `finding`, `partial`.

Coined families, declared once per theory:

```bash
elephant define task-description/2 --arg id:symbol --arg text:symbol --kind state \
    --desc "Work statement for task ?id" -t <theory>
elephant define task-acceptance/2  --arg id:symbol --arg text:symbol --kind state \
    --desc "TEST-derived completion criterion for task ?id" -t <theory>
elephant define attempted/3 --arg task:symbol --arg attempt:symbol --arg peer:symbol --kind state \
    --desc "Peer ?peer claimed attempt ?attempt of task ?task" -t <theory>
elephant define verifier-passed/2 --arg attempt:symbol --arg tree:symbol --kind evidence \
    --desc "Verifier passed for ?attempt at Git tree ?tree; output is preserved" -t <theory>
elephant define review-approved/3 --arg attempt:symbol --arg peer:symbol --arg tree:symbol --kind evidence \
    --desc "Peer ?peer reviewed ?attempt at Git tree ?tree" -t <theory>
elephant define merge-authorized/1 --arg attempt:symbol --kind state \
    --desc "One exact verified attempt may be submitted to Circus merge" -t <theory>
elephant define merge-succeeded/3 --arg task:symbol --arg attempt:symbol --arg commit:symbol --kind evidence \
    --desc "Circus merged ?attempt of ?task as integration commit ?commit" -t <theory>
elephant define attempt-silent/2 --arg attempt:symbol --arg observed-at:symbol --kind evidence \
    --desc "The owner of ?attempt missed the liveness threshold at ?observed-at" -t <theory>
elephant define reclaimable/2 --arg task:symbol --arg attempt:symbol --kind state \
    --desc "Task ?task has an abandoned attempt ?attempt that another peer may replace" -t <theory>
elephant define needs-agent/1 --arg role:symbol --kind state \
    --desc "The theory derives that a peer in role ?role is missing" -t <theory>
```

The last two are easy to miss, because both appear only in rule *heads* and a
rule head runs without a declaration. An undeclared family is one no joining
peer can discover. `elephant vocab` shows it with no description, which is
visible debt in the same sense as a dead link.

Rules, each carrying its source:

```lisp
; verification and review bind the same immutable attempt and Git tree
(normally r-verified
  (and (attempted ?x ?a ?p) (verifier-passed ?a ?d) (review-approved ?a ?r ?d))
  (verified ?a))
(except   d-self-review
  (and (attempted ?x ?a ?r) (review-approved ?a ?r ?d))
  (not (verified ?a)))
(meta r-verified    (source "SPEC-002-leaderless-dispatch-loop#REQ-007"))
(meta d-self-review (source "SPEC-002-leaderless-dispatch-loop#REQ-007"))

; verification authorises one merge; completion follows the recorded merge only
(normally r-merge-authorized (verified ?a) (merge-authorized ?a))
(normally r-completed
  (and (merge-authorized ?a) (merge-succeeded ?x ?a ?m))
  (completed ?x))
(meta r-merge-authorized (source "SPEC-002-leaderless-dispatch-loop#REQ-007"))
(meta r-completed        (source "SPEC-002-leaderless-dispatch-loop#REQ-019"))

; only the silenced attempt is reclaimable; later attempts remain independent
(normally r-reclaimable
  (and (attempted ?x ?a ?p) (attempt-silent ?a ?t))
  (reclaimable ?x ?a))
(except   d-reclaim-done
  (and (completed ?x) (reclaimable ?x ?a))
  (not (reclaimable ?x ?a)))
(meta r-reclaimable   (source "SPEC-002-leaderless-dispatch-loop#REQ-004"))
(meta d-reclaim-done  (source "SPEC-002-leaderless-dispatch-loop#REQ-004"))

; a Tier 1–2 artefact with no reviewer is a staffing gap the theory derives
(normally r-needs-reviewer (attempted ?x ?a ?p) (needs-agent reviewer))
(except   d-staffed (review-approved ?a ?r ?d) (not (needs-agent reviewer)))
(meta r-needs-reviewer (source "PROTO-001#Multi-Model Cognitive Diversity"))
```

Pre-conditions: the theory exists, the peer is a roster member, and the daemon
runs with `ELEPHANT_SYNC_INTERVAL` set. Without the interval there is no sync at
all, and every peer reasons alone while believing it reasons together.

Post-conditions: `(completed X)` is derivable only through a successful
`(merge-succeeded X A M)` of a `(merge-authorized A)` attempt. `(verified A)` is
derivable only where verifier and reviewer agree on the same Git tree, and that
reviewer is not the attempter of A.

Error model: an unprovable literal is a conclusion, not a failure. A peer reads
`why-not` and `describe --json`, and asserts exactly what is missing.

**Verified against elephant 0.1.7 on 2026-08-10.** The ten definitions and
eight rules above were loaded verbatim into a temporary live theory, and the
conclusions were read back:

```console
# attempts: models/m1 (peer-a, reviewed by peer-b), models/m2 (duplicate,
# unverified), api/a1 (peer-a reviews its own), ui/u1 (owner went silent)
$ elephant status -t s2v2
 +d  verified m1                 # reviewer peer-b, attempter peer-a, same tree
 -D  verified a1                 # reviewer is the attempter → defeated
 -D  merge-authorized a1
 +d  merge-authorized m1
 +d  reclaimable ui u1           # only the silenced attempt
                                 # no `verified m2` — a duplicate inherits nothing
                                 # no `completed models` — review is not integration

$ elephant assert '(given (merge-succeeded models m1 c0ffee))' -t s2v2
$ elephant status -t s2v2
 +d  completed models            # …and only now

# a replacement attempt u2, verified by a different peer and merged
$ elephant status -t s2v2
 +d  completed ui
 -D  reclaimable ui u1           # the abandoned attempt retires
```

Self-review is defeated by unification on the shared `?r`, with nobody asserting
that a review was self-reviewed. `elephant vocab` reports
`task-description/2` and `task-acceptance/2` as `[detached]`, which is correct:
their consumer is `elephant next`, not a rule. The remaining declared families
each have a rule consumer or are a rule head that the shell reads.

**Failure model.** Reclamation assumes crash-stop or partitioned peers, not
Byzantine consensus. One local, signed observation is enough to route around a
possibly dead attempt because duplicate work is safe by
[[SPEC-002-leaderless-dispatch-loop#ADR-004]]. A two-distinct-signer quorum is
not claimed: Elephant does not expose signer identity as an SPL term. A programme
requiring Byzantine liveness needs a different evidence representation and a new
specification, not an unenforceable procedural promise.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-004]],
[[SPEC-002-leaderless-dispatch-loop#REQ-006]],
[[SPEC-002-leaderless-dispatch-loop#REQ-007]],
[[SPEC-002-leaderless-dispatch-loop#REQ-019]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-011]],
[[SPEC-002-leaderless-dispatch-loop#TEST-023]],
[[SPEC-002-leaderless-dispatch-loop#TEST-024]],
[[SPEC-002-leaderless-dispatch-loop#TEST-088]],
[[SPEC-002-leaderless-dispatch-loop#TEST-089]],
[[SPEC-002-leaderless-dispatch-loop#TEST-051]].

### CON-003: The Swarm CBCL Dialects

Interface: two CBCL dialects, published through `hark dialect publish` and
installed by every peer.

Input grammar: [[concepts/cbcl|CBCL]], the deterministic context-free language
[[concepts/cbcl-rs|cbcl-rs]] recognises. A dialect definition is itself a CBCL
message, so the same deterministic pushdown automaton that reads ordinary
traffic reads the definition. This contract adds no grammar of its own, and that
is the point: [[SPEC-002-leaderless-dispatch-loop#REQ-005]].b forbids a second
reader.

**`cbcl-usdd-work` — reused, not written.** cbcl-rs already ships a dialect for
leaderless USDD work coordination. Its seven performatives carry the bounded
work-attempt trail: `work-offer`, `work-promise-notice`, `work-progress`,
`work-evidence`, `work-blocked`, `work-review-request`, and
`work-review-result`. Its own comments already state the discipline this specification depends on.
The shared theory is authoritative, a promise notice is not a promise, and a
review result is not a completion assertion. The
composition-first check of Constitutional Principle 15 found it, and this
contract adopts it unchanged.

**`cbcl-swarm-liveness` — the four notices the work trail cannot carry.**
Definition at `specs/dialects/swarm-liveness.cbcl`.

| Performative | Required keys | Meaning |
|---|---|---|
| `peer-beacon` | `:theory`, `:peer`, `:instance`, `:seq`, `:holds`, `:policy` | this peer instance is alive and compatible, and holds these attempts |
| `claim-intent` | `:theory`, `:task`, `:attempt`, `:peer`, `:rank`, `:decision`, `:policy`, `:expires` | this peer may claim one identified attempt; advisory only |
| `swarm-halt` | `:id`, `:by`, `:reason` | troupe-wide withholding control |
| `swarm-resume` | `:halt`, `:by` | releases one identified halt |
| `join-request` | `:theory`, `:peer`, `:wire`, `:decision` | a channel member asks to be admitted to one theory |
| `join-grant` | `:theory`, `:peer`, `:code`, `:decision`, `:expires` | the admitting peer hands over a SPAKE2 invite code |

The first two messages name `:theory`, because one channel carries the traffic
of every theory the troupe is working — see
[[SPEC-002-leaderless-dispatch-loop#ADR-011]]. A `claim-intent` expiry is a
duration no greater than two contention windows. Its expiry instant is local
receipt time plus that duration; a peer never trusts a remote wall clock for
freshness. `:instance` changes on process start and `:seq` increases within an
instance. Troupe carries no admission performative and no secret.

The R5 causal protocol is
`begin → (any peer-beacon claim-intent swarm-halt)`, with
`swarm-halt → swarm-resume`. A beacon repeats every period, so it cannot sit on
a causal chain with its predecessor without making the protocol cyclic. Each
beacon and each intent is therefore its own bounded thread, and only the halt
exchange is a genuine two-step chain.

Constraints both dialects carry:

- An R5 `(shape …)` clause per performative, naming exactly the required keys.
- An R5 `(protocol …)` clause that is acyclic and fully reachable from `begin`.
- An R2 resource bound no larger than the smallest bound any peer configures.
- No redefinition of a core performative, which R3 enforces regardless.

Pre-conditions: both dialects are installed in the peer's local registry before
the first message flows. `hark init --dialect cbcl-usdd-work --dialect
cbcl-swarm-liveness` issues the queries that install them.

Post-conditions: every message the loop sends and receives has passed R1–R5 in
hark's daemon. An outbound violation surfaces as `shape_violation` or
`causal_violation`. An inbound violation is dropped before it reaches `recv`.

Error model: hark exits 8 for a validation failure, 10 for a timeout, and 3 for
a daemon that is not running. A peer treats exit 3 and
exit 10 as the unreachable-channel condition of
[[SPEC-002-leaderless-dispatch-loop#REQ-008]].a.

**Verified.** Both dialects pass R1, R2, R3, and R5:

```console
$ cargo run -q -p cbcl-cli -- verify < dialects/usdd-work.cbcl
dialect 'cbcl-usdd-work' passed all safety checks (R1, R2, R3, R5)
  performatives: 7   resource bounds: depth=8, expansion=1024, time=30ms

$ cargo run -q -p cbcl-cli -- verify < specs/dialects/swarm-liveness.cbcl
dialect 'cbcl-swarm-liveness' passed all safety checks (R1, R2, R3, R5)
  performatives: 6   resource bounds: depth=8, expansion=1024, time=30ms
```

The redirection is load-bearing. `cbcl-cli verify` reads a dialect only from
stdin. Its documented `[INPUT]` argument is parsed as the dialect text itself,
so passing a file name reports
`dialect definition must be a list`.
Recorded against cbcl-rs as `BUG-004`, with the README command it contradicts.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-003]],
[[SPEC-002-leaderless-dispatch-loop#REQ-005]],
[[SPEC-002-leaderless-dispatch-loop#REQ-010]],
[[SPEC-002-leaderless-dispatch-loop#REQ-020]],
[[SPEC-002-leaderless-dispatch-loop#REQ-021]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-018]],
[[SPEC-002-leaderless-dispatch-loop#TEST-032]],
[[SPEC-002-leaderless-dispatch-loop#TEST-052]].

### CON-004: The Attempt Invocation

Interface: the Circus commands the loop composes, in order.

Troupe identifies its immutable attempt as `(theory, circus attempt-id)`. Circus
owns the second component and its run record. Troupe records the theory,
decision identifier, Git tree digest, and policy fingerprint that bind that
local attempt to shared evidence.

```bash
circus spawn  --task <X> --integration <ref> --task-file <prompt> -- <driver>
circus status --attempt <X>/<n> --json
circus accept --attempt <X>/<n> --verifier-record <file> --evidence <ref>
circus merge  --attempt <X>/<n> --into <ref>
```

Pre-conditions:

- The prompt was composed from `task-description`, `task-acceptance`, and
  `circus instruction` output — [[SPEC-002-leaderless-dispatch-loop#REQ-012]].c
- The driver is an executable on `PATH`, named by configuration, and its content
  is opaque to the loop — [[SPEC-002-leaderless-dispatch-loop#REQ-012]].a
- The process environment carries no theory identity, wire key, or channel
  handle — [[SPEC-002-leaderless-dispatch-loop#REQ-006]].a

Post-conditions:

- `accept` is called only after the loop ran the acceptance verifier itself,
  preserved its output, and recorded the attempt's Git tree digest —
  [[SPEC-002-leaderless-dispatch-loop#REQ-006]].c
- `merge` is called only when `(merge-authorized A)` is derivable for the exact
  Troupe attempt A — [[SPEC-002-leaderless-dispatch-loop#REQ-007]].d
- `(merge-succeeded X A M)` is asserted only after Circus records a successful
  merge of A and reports integration commit M —
  [[SPEC-002-leaderless-dispatch-loop#REQ-019]].d
- No literal is asserted from the coding agent's own transcript —
  [[SPEC-002-leaderless-dispatch-loop#REQ-006]].b

Error model: Circus records `failed` for residue past its cleanup deadline and
`rejected` for a non-zero verifier. Both are terminal outcomes the loop asserts
as `(failed A)`; neither is retried inside the same attempt. A rejected or
failed merge never completes task X and never authorises another attempt.

The evidence reference passed to `accept` is opaque text Circus never resolves.
The loop passes `theory:<theory>/<attempt>` and the decision record identifier
from [[SPEC-002-leaderless-dispatch-loop#CON-006]]. An auditor reading the run
record reaches the conclusion, its reasoning, and the exact worktree it
concerns.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-006]],
[[SPEC-002-leaderless-dispatch-loop#REQ-007]],
[[SPEC-002-leaderless-dispatch-loop#REQ-012]],
[[SPEC-002-leaderless-dispatch-loop#REQ-019]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-019]],
[[SPEC-002-leaderless-dispatch-loop#TEST-021]],
[[SPEC-002-leaderless-dispatch-loop#TEST-040]].

### CON-005: External Event Admission

Interface: the untrusted-external boundary between the bus and the theory.

Input grammar: CBCL, recognised by cbcl-rs before the message reaches the loop.
The provenance tag is the discriminator, and it is set by the bus at the webhook
door, never by the sender.

Pre-conditions: the message arrived through `hark recv`, carries the bus's
untrusted-external provenance tag, and passed R1–R5.

Post-conditions:

- The admitted literal is a member of the `discovered` family, parameterised by
  the event — [[SPEC-002-leaderless-dispatch-loop#REQ-011]].a
- No literal in the `verified`, `verifier-passed`, `review-approved`,
  `merge-authorized`, `merge-succeeded`, or `completed` families is asserted —
  [[SPEC-002-leaderless-dispatch-loop#REQ-011]].b
- The source tag is recorded alongside the admitted literal —
  [[SPEC-002-leaderless-dispatch-loop#REQ-011]].c

Error model: an untagged message from an unknown source is discarded and
counted. A message whose provenance tag the peer does not recognise is discarded
and counted. Neither is repaired, normalised, or guessed at.

This is the one contract in this specification whose failure is a security
failure rather than a coordination failure. It escalates to Tier 1 for review
on that ground.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-011]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-036]],
[[SPEC-002-leaderless-dispatch-loop#TEST-037]],
[[SPEC-002-leaderless-dispatch-loop#TEST-038]].

### CON-006: The Peer Decision Record

Interface: one JSON object per claim decision, appended to a durable local log
before the promise is issued.

Input grammar: JSON, with this schema. Every field is REQUIRED.

```json
{
  "decision_id": "string, ULID",
  "at": "string, RFC3339",
  "peer": "string, did:crdt:…",
  "task": "string",
  "attempt": "string, <theory>/<circus-attempt-id>, or null for skip or idle",
  "rank": "integer, ≥ 0",
  "live_peers": ["string, did:crdt:…"],
  "corpus_fingerprint": "string, sha256:…",
  "policy_fingerprint": "string, sha256:…",
  "capacity_domain": "string",
  "commitments_seen": [{"task": "string", "peer": "string", "state": "string"}],
  "outcome": "claim | skip | idle",
  "reason": "string"
}
```

Pre-conditions: the record is written and flushed before the `elephant promise`
process starts.

Post-conditions:

- `corpus_fingerprint` is the output of `elephant closure fingerprint` for the
  read that justified the decision.
- `decision_id` appears in the evidence reference passed to `circus accept` —
  [[SPEC-002-leaderless-dispatch-loop#REQ-013]].c
- A claim record's `attempt` identifies the one future Circus attempt its intent,
  verification, review, and merge evidence can name —
  [[SPEC-002-leaderless-dispatch-loop#REQ-019]].a
- `policy_fingerprint` is the one announced in the peer's beacon —
  [[SPEC-002-leaderless-dispatch-loop#REQ-021]].b
- The record is never rewritten. A superseded decision is a new record.

Error model: a write failure is a hard stop. A peer that cannot write its
decision record SHALL NOT claim, because the claim is then unattributable.

The `corpus_fingerprint` field is what makes a disputed claim resolvable after
the fact. Two peers that claimed the same task can be shown to have read different
closures. That distinguishes a partition from a defect in the election
function.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-013]],
[[SPEC-002-leaderless-dispatch-loop#REQ-019]],
[[SPEC-002-leaderless-dispatch-loop#REQ-021]],
[[SPEC-002-leaderless-dispatch-loop#REQ-022]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-042]],
[[SPEC-002-leaderless-dispatch-loop#TEST-043]],
[[SPEC-002-leaderless-dispatch-loop#TEST-044]].

### CON-007: Peer Configuration

Interface: configuration recognised at start-up, before any other action.

Input grammar: a flat key-value set. Each key is set independently per deploy.
There is no environment name, no profile, and no inherited group.

| Key | Type | Default | Constraint |
|---|---|---|---|
| `theory` | string | none | REQUIRED; an Elephant theory id or alias |
| `channel` | string | none | REQUIRED; a `@handle` the peer has joined |
| `repository_id` | string | none | REQUIRED; stable identity of the repository the integration ref belongs to |
| `driver` | string | none | REQUIRED; an executable resolvable on `PATH` |
| `integration_ref` | string | none | REQUIRED; an existing Git ref |
| `verifier` | string list | none | REQUIRED; an argument vector, not a shell string |
| `beacon_period` | duration | `15s` | ≥ 1 s, ≤ silence threshold ÷ 3 |
| `silence_threshold` | duration | `45s` | ≥ 3 × beacon period |
| `contention_window` | duration | `1s` | ≥ channel round-trip p95 |
| `max_in_flight` | integer | `1` | ≥ 1 |

Pre-conditions: none. This runs first.

Post-conditions:

- Every REQUIRED key is present and every constraint holds, or the peer exits
  non-zero without contacting the theory or the channel.
- No credential appears in this configuration. The theory identity lives in
  `ELEPHANT_HOME` and the wire key in hark's config directory, each owned by the
  program that created it.
- `policy_fingerprint` is the SHA-256 digest of the canonical tuple `(theory,
  SPEC-002 version, swarm-liveness dialect digest, repository_id,
  integration_ref, verifier)`. It is computed, never accepted as configuration.
- Every Troupe process on the same OS user and execution host acquires its
  capacity slot from one native advisory lock path.
- That path derives from the OS user and execution host.
- The operating system releases the lock if its process exits.

Error model: fail closed. A missing key, an unresolvable driver, an unparseable
duration, a missing repository identity, or a violated constraint each exits
non-zero with the offending key named. No default is invented for a REQUIRED
key.

`verifier` is an argument vector rather than a shell string, for the reason
[[SPEC-002-leaderless-dispatch-loop#REQ-005]].c gives. A string that becomes a
command is a parser at a trust boundary, and this composition has agreed not to
write one.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-009]],
[[SPEC-002-leaderless-dispatch-loop#REQ-012]],
[[SPEC-002-leaderless-dispatch-loop#REQ-021]],
[[SPEC-002-leaderless-dispatch-loop#REQ-022]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-053]],
[[SPEC-002-leaderless-dispatch-loop#TEST-054]].

### CON-008: The Peer Command-Line Interface

Interface: five commands and no daemon. Each one reads the configuration, does
one thing, and prints one kind of output.

| Command | What it does |
|---|---|
| `seed` | Writes this specification's vocabulary and rules into a theory, once |
| `run` | Runs the loop in the foreground until stopped |
| `once` | Runs exactly one pass of the loop, then exits |
| `status` | What this peer sees now — live peers, ranks, in-flight, mode |
| `decisions` | Reads back this peer's decision records |

```
troupe seed      [--config <path>] [--dry-run]
troupe run       [--config <path>]
troupe once      [--config <path>] [--task <id>]
troupe status    [--config <path>]
troupe decisions [--since <RFC3339>] [--task <id>]
```

Every command takes `-q`/`--quiet`, `-v`/`--verbose`, and `--no-color`.

There is no `--json` flag, and its absence is the point.
[[SPEC-002-leaderless-dispatch-loop#REQ-014]].a and .c together mean stdout is
always the machine-readable form, and `-q`/`-v` govern stderr alone. A flag
switching stdout between a human form and a machine form is the implicit
context-sensitivity Constitutional Principle 2 treats as a code smell. The flag
makes it explicit without making it correct. Circus reached the same shape from
the same rule.

Pre-conditions:

- Configuration resolves and every constraint of
  [[SPEC-002-leaderless-dispatch-loop#CON-007]] holds. Otherwise the command
  exits 3 before contacting the theory or the channel.
- `run` and `once` additionally require writable decision and control-journal
  paths. [[SPEC-002-leaderless-dispatch-loop#REQ-013]].b forbids claiming
  without the former. [[SPEC-002-leaderless-dispatch-loop#REQ-010]] requires
  the latter before restart recovery.

Post-conditions:

- **`seed`** declares every coined family of
  [[SPEC-002-leaderless-dispatch-loop#CON-002]] and asserts its rules, then
  publishes the four-performative `cbcl-swarm-liveness` dialect. It is idempotent: it reads
  `elephant vocab` and `hark dialect list` first, and writes only what is
  absent. It seeds vocabulary and rules, never tasks. A task comes from the governing
  specification's planning ledger, which is a Phase 2 activity this loop does
  not perform. `--dry-run` prints what it writes and writes nothing.
- **`run`** writes the [[SPEC-002-leaderless-dispatch-loop#Observability]] event
  stream to stdout, one JSON object per line, unbuffered. That stream is the
  primary output, so the `LOG` factor and
  [[SPEC-002-leaderless-dispatch-loop#REQ-014]].a agree rather than compete. The
  peer routes and rotates none of it. It stores only the append-only decision
  records and durable control journal this specification requires.
- **`once`** runs exactly one SENSE and ELECT sequence. For a claim, it runs
  local CAPACITY, CLAIM, RUN, VERIFY, and TELL. A later peer performs review and
  the authorised merge. It prints one decision record to stdout and exits.
  `--task` restricts the candidate set to one task for a human-driven attempt.
- **`status`** prints one JSON object. It carries the resolved configuration and
  the observed live peer set with last-beacon ages. It carries this peer's rank
  per current candidate, in-flight promises, policy fingerprint, local capacity
  status, degraded mode, and recovered halt state. It exits non-zero WHEN the
  peer is not fit to claim —
  [[SPEC-002-leaderless-dispatch-loop#REQ-014]].e.
- **`decisions`** prints the matching
  [[SPEC-002-leaderless-dispatch-loop#CON-006]] records, oldest first, one per
  line. It never writes one.
- `status` and `decisions` are read-only. Neither promises, asserts, emits, nor
  starts an attempt.

Error model — a stable exit-code table, in the shape hark already uses:

| Code | Meaning |
| ---: | --- |
| 0 | success, including a pass that claimed nothing |
| 2 | usage error |
| 3 | configuration invalid or incomplete |
| 4 | theory unreachable |
| 5 | channel unreachable, for `status` only |
| 6 | driver not resolvable on `PATH` |
| 7 | decision record not writable |

Idle and halted are **not** error codes. An empty candidate set is successful
idle, and a halt is the peer obeying an instruction. Both exit 0 and report the
outcome on stdout. An exit code meaning "nothing was wrong and nothing
happened" is indistinguishable from one meaning "nothing happened because
something was wrong".

Signals, per the `DIS` disposability factor:

- The first `SIGINT` or `SIGTERM` stops new claims. Any in-flight attempt runs
  to its own terminal state, its evidence is preserved, and the peer then exits
  0. This is the halt behaviour of
  [[SPEC-002-leaderless-dispatch-loop#REQ-010]], reached from the operator's
  side.
- The second `SIGINT` or `SIGTERM` exits immediately. The Circus attempt keeps
  running, which owes nothing to the peer: Circus records its own attempt, and
  withdone reaps its own process group. A peer that exits mid-attempt therefore
  leaves no lease to reap and no cleanup owed to another component.

Environment passed to the driver: `CIRCUS_PROMPT` and `CIRCUS_SENTINEL`, which
Circus itself sets, and nothing else this peer adds.
[[SPEC-002-leaderless-dispatch-loop#REQ-006]].a makes that omission a
prohibition rather than an oversight. TEST-019 checks it.

What is deliberately absent, per
[[SPEC-002-leaderless-dispatch-loop#REQ-014]].d:

| Absent | Use instead |
|---|---|
| a halt or resume verb | `hark emit`, which already signs and validates the frame |
| a log or attach verb | `circus logs`, `circus status`, `tmux attach` |
| a theory query verb | `elephant explain`, `describe --json`, `status` |
| a merge or accept verb | `circus accept`, `circus merge` |
| a membership or invite verb | human, out-of-band admission per [[SPEC-002-leaderless-dispatch-loop#REQ-018]] |
| a daemon, socket, or control API | nothing needs one — see [[SPEC-002-leaderless-dispatch-loop#ADR-009]] |

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-013]],
[[SPEC-002-leaderless-dispatch-loop#REQ-014]],
[[SPEC-002-leaderless-dispatch-loop#REQ-018]],
[[SPEC-002-leaderless-dispatch-loop#REQ-022]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-057]],
[[SPEC-002-leaderless-dispatch-loop#TEST-058]],
[[SPEC-002-leaderless-dispatch-loop#TEST-059]],
[[SPEC-002-leaderless-dispatch-loop#TEST-060]],
[[SPEC-002-leaderless-dispatch-loop#TEST-061]],
[[SPEC-002-leaderless-dispatch-loop#TEST-062]].

### CON-009: Swarm Initialisation

Interface: an ordered ceremony in three stages. Every other contract in this
specification states its pre-conditions as though a swarm already exists. This
one is how a swarm comes to exist.

The ceremony is **asymmetric, and the asymmetry does not survive it** — see
[[SPEC-002-leaderless-dispatch-loop#ADR-010]].

#### Stage A — found the swarm (once, by one founder)

```bash
# 1. the channel — created in the web client, not from a CLI.
#    Open the hub, use the create-channel control, name it, set it PRIVATE.

# 2. an identity, and the theory this programme of work reasons in
elephant id create --name <founder>
elephant theory create <spec-id>

# 3. this specification's vocabulary and rules, before anyone joins
troupe seed --config <path>          # idempotent; --dry-run to inspect first

# 4. take a seat in the channel
hark join @<channel> --as @<founder> --speak cbcl-usdd-work,cbcl-swarm-liveness
```

Step 1 is a browser action and has no command line. The hub's chat app owns
channel creation, and `hark join` joins a channel that already exists.
Visibility is set once at creation and matters:
[[SPEC-002-leaderless-dispatch-loop#ADR-011]] requires a private channel.

Stage A ends with a theory that knows how to conclude `verified`, `completed`,
and `reclaimable`. It also ends with a published dialect, and a channel holding
one member.

Ordering is load-bearing, and step 2 is the step that is skipped. A joiner
inherits the vocabulary it finds. A joiner arriving before the vocabulary
exists coins its own. A peer asserting
`build-ok` past a rule listening for `verifier-passed` is the silent
coordination failure Elephant's own vocabulary specification was written to
prevent. `troupe seed` is idempotent so that
running it late repairs the theory, but it cannot un-coin what a peer already
invented.

#### Stage B — admit each peer (once per peer, needs a human at each end)

Two admissions, both human, both paid **once per peer for the troupe's whole
life**. Theory admission is not here: under
[[SPEC-002-leaderless-dispatch-loop#ADR-012]] the peers perform it themselves,
and [[SPEC-002-leaderless-dispatch-loop#REQ-018]] governs it.

```bash
# 1. a seat in the channel. A member mints a pairing phrase from the web
#    client's add-agent control and hands it over out of band.
hark pair <id>-word-word
#    Or, where the joiner already holds a channel capability:
hark join @<channel> --as @<peer> --speak cbcl-usdd-work,cbcl-swarm-liveness

# 2. an enrolment on the routed transport, which carries the invite grant.
#    An admin mints a one-time mnemonic; the agent redeems it over SPAKE2.
curl -s -X POST https://<hub>:8443/admin/v1/mnemonic \
  -H "Authorization: Bearer $ADMIN_JWT" -H 'Content-Type: application/json' \
  -d '{"agent_id":"<peer>","ttl_seconds":900,"capabilities":["agent"]}'

# 3. the peer's own theory identity, created locally and needing nobody
elephant id create --name <peer>
```

Step 2 is the cost [[SPEC-002-leaderless-dispatch-loop#ADR-012]] pays to keep an
invite code off the chat fan-out. The routed transport is the only
point-to-point carrier the composition has, and reaching it needs its own
admin-gated enrolment. A troupe that never opens a second theory does not need
it. One that does pays it once per peer.

Both admissions are [[concepts/spake2|SPAKE2]] ceremonies whose secret travels
out of band. Both therefore **block on a human**, which is the operational cost
of [[SPEC-002-leaderless-dispatch-loop#ADR-006]]: a troupe cannot seat itself.

An invite code that passes through an agent transcript is spent. A transcript
is a log, and the code is a password valid for its whole TTL. Use a short
`--ttl`. Let the joiner type the code on the TTY rather than passing it in
`argv`, where it lands in `ps` and shell history. Check the result with
`elephant theory members`.

#### Stage C — make each peer ready (per peer, no human)

```bash
export ELEPHANT_SYNC_INTERVAL=30     # unset means no sync at all
elephant daemon start
hark daemon start
troupe status                        # exits 0 only when this peer is fit
```

Pre-conditions: none for Stage A. Stage B requires Stage A complete. Stage C
requires Stage B complete for this peer.

Post-conditions, checkable in one command each:

| Check | Command | Expected |
|---|---|---|
| The theory knows the vocabulary | `elephant vocab -t <spec-id>` | 10 coined families, each with a description |
| The rules derive | `elephant status -t <spec-id>` | `r-verified`, `d-self-review`, `r-reclaimable` present |
| Peers agree on the closure | `elephant closure fingerprint -t <spec-id>` | the same digest on every peer |
| The roster is who you expect | `elephant theory members -t <spec-id>` | no unexpected DID |
| The dialects are installed | `hark dialect list` | `cbcl-usdd-work`, `cbcl-swarm-liveness` |
| The peer is fit to claim | `troupe status` | exit 0 |

Error model:

- `ELEPHANT_SYNC_INTERVAL` unset is the failure that looks like success. Every
  peer runs, every peer reasons, and no peer sees another's assertions. The
  `closure fingerprint` row above is the check that catches it, and
  `troupe status` exits non-zero WHEN the peer's daemon reports no sync
  interval.
- An unexpected DID in the roster is removed with `elephant theory remove`,
  which MLS-removes the member and rotates the corpus key. It is not tidied
  later.
- A failed `hark join` on an encrypted channel fails closed rather than sending
  plaintext.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-015]],
[[SPEC-002-leaderless-dispatch-loop#REQ-016]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-067]],
[[SPEC-002-leaderless-dispatch-loop#TEST-068]],
[[SPEC-002-leaderless-dispatch-loop#TEST-069]],
[[SPEC-002-leaderless-dispatch-loop#TEST-070]],
[[SPEC-002-leaderless-dispatch-loop#TEST-071]].

### CON-010: Durable Attempt and Control Journal

Interface: two append-only JSON Lines journals local to a peer: an
`attempts.jsonl` ledger and a `control.jsonl` journal. Neither is a source of
shared truth. The former makes the exact evidence record behind a later Elephant
assertion auditable; the latter restores a locally received halt after restart.

Input grammar: each line is one JSON object, recognised in full before the
record is used. An attempt-evidence record has these REQUIRED fields:

```json
{
  "record_id": "ULID",
  "kind": "attempt-evidence | merge-result",
  "at": "RFC3339",
  "decision_id": "ULID",
  "theory": "string",
  "task": "string",
  "circus_attempt": "string",
  "policy_fingerprint": "sha256 string",
  "tree": "Git tree object id",
  "verifier_record": "path or digest",
  "merge_commit": "Git commit object id or null"
}
```

A control record has these REQUIRED fields:

```json
{
  "record_id": "ULID",
  "kind": "halt | resume",
  "at": "RFC3339",
  "halt_id": "ULID",
  "signer": "wire-verified channel member",
  "reason": "string or null",
  "source_frame": "signed CBCL frame identifier"
}
```

Pre-conditions: Troupe accepts only a R1–R5-recognised control frame, and it
writes and flushes its corresponding control record before the next election.
The attempt-evidence record is written after the local verifier produces the
tree digest and before its `(verifier-passed A D)` assertion. The merge-result
record is written after `circus merge` returns and before a
`(merge-succeeded X A M)` assertion.

Post-conditions:

- Replaying `control.jsonl` from the beginning reconstructs the active halt set.
  A resume changes only the halt it names and only with the matching signer.
- A duplicate `source_frame` changes neither journal.
- Every verifier, review, and merge assertion has one matching attempt-evidence
  or merge-result record with the same theory, attempt, tree, and policy.
- Neither journal changes the Elephant corpus, Circus run record, or Hark state.

Error model: an unparseable, truncated, duplicate, or unwritable journal record
is a hard stop for claim or control-state transition. A peer can still preserve
an already running Circus attempt, but it SHALL NOT issue a new claim or merge.

Implements: [[SPEC-002-leaderless-dispatch-loop#REQ-007]],
[[SPEC-002-leaderless-dispatch-loop#REQ-010]],
[[SPEC-002-leaderless-dispatch-loop#REQ-019]],
[[SPEC-002-leaderless-dispatch-loop#REQ-020]],
[[SPEC-002-leaderless-dispatch-loop#REQ-021]].

Verified by: [[SPEC-002-leaderless-dispatch-loop#TEST-088]],
[[SPEC-002-leaderless-dispatch-loop#TEST-090]],
[[SPEC-002-leaderless-dispatch-loop#TEST-097]],
[[SPEC-002-leaderless-dispatch-loop#TEST-098]].

## Purity Boundary Map

### Pure Core (no I/O, no shared state, deterministic)

- `elect(corpus, peers, self, clock)`: the whole decision of
  [[SPEC-002-leaderless-dispatch-loop#CON-001]].
- `rank(task, self, peers)`: the rendezvous ordering.
- `live(peers, clock, threshold)`: the live-set filter.
- `halted(control_journal)`: whether a durable, unmatched halt is in force.

### Effectful Shell (orchestrates I/O, calls pure core)

- `read_theory()`: runs `elephant next --json`, `status --json`,
  `commitments`, `closure fingerprint`.
- `read_channel()`: runs `hark recv` with a bounded timeout.
- `beacon()`, `announce_intent()`: run `hark emit`.
- `record_decision()`: writes [[SPEC-002-leaderless-dispatch-loop#CON-006]].
- `record_control()`, `record_artifact()`: write
  [[SPEC-002-leaderless-dispatch-loop#CON-010]].
- `acquire_capacity()`: takes the native local advisory lock of
  [[SPEC-002-leaderless-dispatch-loop#REQ-022]].
- `run_attempt()`: runs the Circus sequence of
  [[SPEC-002-leaderless-dispatch-loop#CON-004]].
- `verify()`: runs the configured verifier argument vector.
- `tell_theory()`: runs `elephant promise` and `elephant assert`.

### Boundary Contracts (data types crossing the boundary)

- `Corpus`: shell → core. Conclusions, commitments, and a fingerprint.
- `PeerSet`: shell → core. DIDs with last-beacon times.
- `ControlState`: shell → core. The replayed active halt set.
- `Decision`: core → shell. Exactly one of `Claim`, `Skip`, `Idle`.

### Dependency Rule

Dependencies point inward: shell → core. The core MUST NOT import from the
shell, read a clock, or invoke a program.

### Enforcement

Module visibility plus an import lint, matching the boundary gates the sibling
repositories already run. The core is what the property tests of
[[SPEC-002-leaderless-dispatch-loop#TEST-005]] and
[[SPEC-002-leaderless-dispatch-loop#TEST-006]] run against. It is also the
model any falsification of the temporal properties searches over.

## Temporal Properties

Notation: STL over declared signals. The toolchain is not selected, and that gap
is an Open item. Signals are `OBS-###` identifiers, so the same formula gates the
suite in Phase 3 and the production trace in Phase 4.

```
signal claim_issued        = OBS-001
signal beacon_emitted      = OBS-002
signal halt_recorded       = OBS-008
signal resume_recorded     = OBS-008
signal decision_written    = OBS-001.record
signal capacity_available  = OBS-012
signal policy_compatible   = OBS-013
signal theory_reachable    = OBS-006
signal channel_reachable   = OBS-006
param  beacon_period       = 15s      # NFR-002, dominant profile
param  silence_threshold   = 45s      # NFR-002
param  claim_window        = 2s       # NFR-001

# REQ-003.a  a live peer beacons within every period
REQ-003.a  always eventually[0s, 15s] beacon_emitted

# REQ-010    a halt withholds every later claim until its matching resume
REQ-010.a  halted    = active(halt_recorded, resume_recorded)
REQ-010    always (halted => not claim_issued)

# REQ-013.b  no claim without its record, and the record comes first
REQ-013.a  recorded  = past decision_written
REQ-013    always (claim_issued => recorded)

# NFR-006    a reclaimable task is claimed within the stated bound
NFR-006.a  reclaimable_seen = OBS-004
NFR-006    always ((reclaimable_seen and not halted and capacity_available and
                    policy_compatible and theory_reachable and channel_reachable)
                   => eventually[0s, 90s] claim_issued)
```

Each composite is built from named atoms. A failure then attributes to one atom,
and its repair stays scoped to it. `REQ-013` is worth exemplifying before
implementation starts. A property set permitting a claim with no prior record
is unsatisfiable against `CON-006`. Exemplification finds that without running
anything. NFR-006 is guarded. A human halt, partition, incompatible policy, or
exhausted capacity deliberately withholds progress.

## Architecture Decisions

### ADR-001: Two coordination planes, and which carries what

Status: proposed.

Context: the composition offers two ways for peers to reach each other. Elephant
syncs a signed corpus peer-to-peer. cbcl-bus fans signed frames through a hub. A
reviewer's first question is why both.

Decision: Elephant carries **claims** and cbcl-bus carries **events**. A claim is
permanent, signed, reasoned over, and has no delivery deadline. An event is
transient, timely, addressed, and has no obligation to survive.

Consequences: the split is not redundancy, because neither plane substitutes for
the other. Elephant has no liveness signal at all — a dead peer's promise stays
outstanding for ever, with nothing to distinguish "died" from "still working".
The bus has no defeasible reasoning and no replayable record an auditor can
check. The loop's contribution is the bridge.
[[SPEC-002-leaderless-dispatch-loop#REQ-004]] turns an observation made on the
fast plane into a signed literal on the slow one, and the theory concludes
reclamation from it.

Alternatives rejected: using Elephant alone forces a heartbeat into the corpus,
which appends a signed statement every 15 s per peer for ever. Using the bus
alone throws away exactly the property that makes acceptance auditable.

### ADR-002: A composition, not a component

Status: proposed.

Context: Constitutional Principle 15 and the Simplicity Ladder both ask whether
a new component is needed. Rung 4 asks whether existing components already
compose into the capability.

Decision: Troupe is a specification of a procedure. Its reference
implementation is one foreground program that shells out. It runs no daemon,
opens no socket, and stores only append-only decision, attempt-evidence, and
control journals whose local state a restart can replay.

Consequences: the theory holds coordination truth, Circus holds one attempt's
worktree and run record, and hark's daemon holds the connection. Troupe holds
only the externalised rationale, exact-evidence, and received-control records
needed to make its own actions attributable and restart-safe.

Troupe SHALL NOT be folded into Circus. Troupe reads Elephant and Hark, performs
cross-peer election, emits beacons and intents, and chooses whether to invoke
Circus. Circus owns neither theory nor channel access. It is prohibited from
becoming a scheduler or daemon by [[SPEC-001-circus-agent-harness#REQ-007]].b.
Adding Troupe as a Circus subcommand broadens every boundary this composition
preserves. An external composition program is the lowest sufficient rung.

The ladder repaid the check twice. cbcl-rs already ships `cbcl-usdd-work`, a
verified dialect for leaderless USDD coordination. The remaining wire vocabulary
is four liveness/control performatives —
[[SPEC-002-leaderless-dispatch-loop#CON-003]]. Elephant's reserved families
still carry `verified`, `completed`, `failed`, and discovery; Troupe coins only
the attempt-specific predicates the existing vocabulary cannot express.

Alternatives rejected: a resident swarm daemon needs its own supervision, its
own restart semantics, and its own answer to one question. That question is
what happens when the thing that coordinates dies, and this whole design exists
to avoid it.

### ADR-003: The contention window is sized to the channel

Status: proposed.

Context: a promise reaches another peer only after an Elephant sync interval,
whose default is 30 s. A contention window shorter than the propagation delay
lets two peers claim; a window longer than it makes low-ranked peers wait
minutes.

Decision: announce intent on the channel, where the round trip is sub-second, and
size the contention window to that. Keep the promise as the authoritative claim.

Consequences: the fast plane collapses the window from tens of seconds to one,
and the slow plane still decides who actually holds the task. A lost intent
message costs a duplicate attempt, which
[[SPEC-002-leaderless-dispatch-loop#ADR-004]] establishes is survivable. When the
channel is unreachable, [[SPEC-002-leaderless-dispatch-loop#REQ-008]].b widens the
window back to the sync interval, trading latency for the safety the intent
message was providing.

Alternatives rejected: lowering `ELEPHANT_SYNC_INTERVAL` to 1 s makes every
peer dial every roster peer every second. That is a cost paid on all traffic,
to fix one decision.

### ADR-004: Duplication is survivable, not prevented

Status: proposed.

Context: two peers can promise the same goal. Elephant states this is
deliberate. The corpus records the duplication as a fact for the group to
resolve, rather than hiding it behind whoever won a race.

Decision: do not attempt mutual exclusion. Make duplication rare with the
contention window, make it visible in the corpus, and make it harmless by
construction.

Consequences: harmlessness is not an assumption, it is a property of the
pieces. Each attempt runs in its own Git worktree, so two attempts never touch
each other's files. Circus serialises merges under an advisory lock, and
refuses a merge whose preconditions no longer hold. The second merge is
therefore recorded as rejected, rather than silently clobbering the first. The
cost of a duplicate is therefore wasted compute, never a corrupted integration
ref.

This reframes the liveness beacon. It is an efficiency mechanism, not a
correctness control. That distinction matters for review: a beacon defect
degrades throughput, and reviewers SHOULD NOT reason about it as though it
degraded safety.

Alternatives rejected: a distributed lock needs a lock service, which needs a
leader, which is the thing being removed.

### ADR-005: Rendezvous hashing over random backoff

Status: proposed.

Context: some rule must decide which peer waits longest, without exchanging a
message to decide it.

Decision: rank by `H(task-id ‖ peer-did)`, descending, and wait
`rank × window`.

Consequences: every peer computes the same order from data every peer already
has. Removing a peer changes the rank of a task only for the peers below the
removed one, so a death redistributes work without reshuffling it. Different
tasks give different peers rank zero, so load spreads without a balancer. The
function is pure, which is why it sits in
[[SPEC-002-leaderless-dispatch-loop#CON-001]] and why
[[SPEC-002-leaderless-dispatch-loop#TEST-005]] can be a property test rather than
an integration test.

Alternatives rejected: uniform random backoff spreads claims but gives no
agreement, so two peers draw similar delays often enough to matter at small peer
counts. Lowest-DID-wins gives agreement but sends every task to the same peer.

### ADR-006: The coding agent is not a theory member

Status: proposed.

Context: the theory-coordination pattern shows a worker promising its own goal
and asserting its own evidence, which makes attribution cryptographic. That needs
the worker to be a roster member.

Decision: the coding agent does not join the theory. The peer that dispatched it
is the signer, and it signs only what its own verifier established.

Consequences: the decision is forced rather than chosen. Joining requires the
SPAKE2 invite ceremony, and `elephant theory invite` blocks waiting for the
joiner. The invite code is deliberately kept off stdout, so it travels out of
band. A swarm cannot enrol a worker without a human, and a design that needs
one per attempt is not a design.

The consequence for review is favourable. The dispatching peer is the attempt's
author, so [[SPEC-002-leaderless-dispatch-loop#REQ-007]] forces a different peer
to approve it. Principle 12 is satisfied at the peer boundary rather than at the
worker boundary, and the signer DIDs are the evidence.

The consequence for trust is the cost: an agent's claim about its own work has no
standing at all, which is why
[[SPEC-002-leaderless-dispatch-loop#REQ-006]].b exists.

### ADR-007: Flat gate literals where agents interrogate them

Status: proposed.

Context: parameterised literals are RECOMMENDED because flat atoms are
vulnerable to the task-declaration squat. But `why-not` and `require`
under-report on parameterised goals. For `(verified req-042)` they answer that
no rule concludes it, which is wrong. They then abduce the goal itself rather
than its premises.

Decision: use parameterised literals throughout
[[SPEC-002-leaderless-dispatch-loop#CON-002]]. Read a rule's structure with
`describe --json` and `dag`, and do not route a peer's diagnosis through
`why-not` or `require`.

Consequences: peers keep the squat-immune, arity-checked form, and pay for it
by losing the pull-based agreement channel. That is affordable here, because
this contract fixes the vocabulary rather than discovering it at run time. A
peer never needs to ask the theory what a missing premise is called, because
[[SPEC-002-leaderless-dispatch-loop#CON-002]] already names every one.

This decision is revisited when Elephant's recorded `REQ-401` witness-join item
closes.

### ADR-008: Troupe is not part of Circus

Status: proposed.

Context: the loop composes five repositories and belongs to none of them.
Elephant refuses orchestration, Circus refuses scheduling, hark and cbcl-bus
carry transport, and cbcl-rs is a library. Circus is the nearest neighbour. Its
subject is local agent execution, and the loop calls four of its commands. So
the question is whether the loop is a part of it.

Decision: no. Troupe is a separate component, and it ships from its own
repository once it is implemented. Its specification lives in the Circus vault
until that repository exists, which is a transitional arrangement rather than
the answer.

Consequences. Four things decided this, and the first is Circus saying so
itself.

**Circus already prohibits what the loop is.**
[[SPEC-001-circus-agent-harness#REQ-007]].b forbids Circus from implementing a
scheduler, a persistent process, or a replacement for Elephant's theory
queries. The loop is the first, runs as the second, and reads the third.
Shipping it from the same repository under a different binary name satisfies
the letter and defeats the point. That is exactly the encrustation the
Specification Status Lifecycle guards against.

**Circus already states the pattern for adjacent executables.** A driver is an
ordinary executable found through `PATH`; Circus does not install one, and the
binary does not know they exist. Troupe stands in the same relation to Circus
that a driver does: it is found on `PATH`, it invokes the commands, and nothing
links. The plugin system Unix already provides is the composition-first answer
at rung 4.

**The family's own pattern is caller and callee in separate repositories.**
hark calls cbcl-rs; separate. Elephant calls cbcl-rs; separate. cbcl-bus
consolidated two hubs into one umbrella because they shared an admission spine.
Circus and Troupe share none. Their relationship is invocation, not a common
core.

**The dependency and release stories diverge.** Circus depends on three pure Rust crates, and on `git`, `tmux`, and `withdone`.
Its README frames that minimalism as the point. It says a harness quietly
pulling in three dependencies is not composing them. Troupe's integration tests
run against real Elephant, hark, and Circus. Constitutional Principle 5 asks
for realistic environments. Every failure mode here is what a mock removes: a
sync interval, a dropped socket, an advisory merge lock. Merging the
repositories either widens Circus's install contract, or gives its users a
half-working one. Their release trains diverge too. `SPEC-001` is `implemented`
at 0.1.1, and this specification is `draft` with a Tier 1 open item.

Circus stays useful alone, which is what most of its users need: one developer,
one agent, one worktree, no hub, no theory, no second machine. Bundling makes
that case carry this one's weight.

**Timing, and the trigger.** Creating a repository, a release pipeline, an installer, and a CI job for an
unimplemented draft is speculative scaffolding. The Simplicity Ladder's
discipline rules prohibit it. So the split
happens at implementation, not now, and the specification sits in the Circus
vault meanwhile. It places no obligation on the `circus` binary, and
[[SPEC-001-circus-agent-harness#REQ-007]].b stays intact.

// SIMPLIFY: specification hosted in the Circus vault — move it to the Troupe
// repository at the first implementation commit (trace:
// SPEC-002-leaderless-dispatch-loop#ADR-008)

This decision reverses on one condition. A reference implementation small
enough to need no release cadence of its own, and needing no Elephant or hark
integration test. Neither is likely. The Tests table already names integration
against all three as the default.

### ADR-009: One foreground process, one pass per invocation

Status: proposed.

Context: [[SPEC-002-leaderless-dispatch-loop#ADR-002]] says the loop runs no
daemon. A dispatch loop is nevertheless a loop, and something has to keep
running it. That reads as a contradiction until the word is pinned down.

Decision: `troupe run` is a **foreground process**, supervised by launchd or a
systemd user service, exactly as `hark daemon run` and `elephant daemon run`
already are. `troupe once` is the same loop body, executed exactly once.

Consequences: the distinction ADR-002 cares about is ownership, not lifetime. A
daemon owns a socket, serves clients, holds a singleton lock, and has other
programs depending on its being up. This process owns none of those. Nothing
connects to it, nothing waits on it, and killing it costs the swarm one worker.
Two things follow, and a control API costs both of them.

First, `once` makes the whole loop testable, because a single pass is a single
observable transition rather than a slice of a running process. Every temporal
property in [[SPEC-002-leaderless-dispatch-loop#Temporal Properties]] is stated
over a trace, and `once` produces the shortest trace that contains a decision.
`TEST-044` asserts that the decision record is written before the promise
process starts. That is two lines over one `once` invocation, and an
interleaving problem over a `run`.

Second, `once` is the manual override. An operator wanting one attempt on one
task runs `troupe once --task api`. Back come a decision record and an exit
code, with no supervisor, no socket, and no process to stop afterwards.

Alternatives rejected: a resident process with a control socket needs three
things of its own. Its own authentication, its own API version, and its own
answer to who is allowed to halt it. Those are three problems the swarm already
solved on a signed channel. A `cron`-only design with no `run` pays for a full
theory read and a dialect check on every tick. It also leaves the beacon of
[[SPEC-002-leaderless-dispatch-loop#REQ-003]] nowhere to live between ticks.

### ADR-010: Initialisation is asymmetric; operation is not

Status: proposed.

Context: a leaderless system still has to start. Someone runs `theory create`
first, someone seeds the vocabulary, and someone holds the invite open while a
joiner types a code. Each of those is a role exactly one peer plays. A reviewer
is right to ask whether
[[SPEC-002-leaderless-dispatch-loop#REQ-001]] survives contact with its own
bootstrap.

Decision: accept the asymmetry at t=0, and forbid it from persisting.
[[SPEC-002-leaderless-dispatch-loop#REQ-015]] makes founding unreadable to the
loop: no decision consults it, and no configuration key names it.

Consequences: the founder is first in time and nothing else. The properties
making the swarm leaderless are properties of Elephant, not of good behaviour.
Work is not assigned, a promise is not revocable by anyone but its signer, and
a statement is not forgeable. None of that changes because a particular peer
ran `theory create`.

One asymmetry genuinely persists, and the defect here is pretending otherwise.
The **steward** can remove a member and rotate the corpus key. That is
membership, not dispatch: it decides who is in the room, never who does the
work. The line is checkable, because no requirement in this specification reads
membership authority, and `TEST-068` asserts that no decision consults the
founder.

The second consequence is operational and unwelcome. Both planes admit a peer
through a SPAKE2 ceremony whose secret travels out of band, so **adding a peer
costs a human at each end**. A swarm cannot grow itself. That is the price of
[[SPEC-002-leaderless-dispatch-loop#ADR-006]], and of the property making
adversarial review checkable at all. An agent able to enrol another agent
manufactures its own reviewer, and Constitutional Principle 12 reverts to an
honour system.

Alternatives rejected: a shared bearer capability distributed with the
configuration lets peers self-enrol. It also makes every peer's identity
forgeable by anyone holding the file. An enrolment service is a coordinator,
restoring by the back door what this whole specification removes.

### ADR-011: The troupe is the channel; a theory is one work programme

Status: proposed.

Context: a swarm has two memberships — a channel roster and a theory roster —
and a reader reasonably asks which one *is* the swarm. An earlier revision of
this specification answered "the theory", which is wrong, and the error is
instructive. USDD binds **one theory per `SPEC-###`**, so a theory is the
lifetime of one specification. A set of machines running coding agents is not.
The same peers work a second specification, and a third, while the first is
still open.

Decision: the **troupe** is the channel membership — a durable set of peers. A
**theory** is one work programme with its own roster, and a peer belongs to
zero or more of them. The channel roster is therefore a *superset* of every
theory roster, never an equal.

```
   channel  @troupe          ← the durable set of peers; admission is human
   ├── theory spec-014       ← peers A B C   ┐
   ├── theory spec-021       ← peers A   C D │  each its own roster,
   └── theory spec-030       ← peers   B   D ┘  its own closure, its own loop
```

Consequences: three, and the third is the one that pays.

The **live peer set is per theory**, which is what
[[SPEC-002-leaderless-dispatch-loop#REQ-017]].b now says. One channel carries
every theory's beacons, so a peer discards most of what it hears. Filtering on
the wrong roster lets a peer working `spec-021` shift the ranks of a theory it
is not in.

The **theory stays authoritative for work**. That is what the earlier revision
got right, and it belonged to work rather than to identity. Claims live there,
and its roster is cryptographically closed. The loop already falls back to
`elephant theory members` as its live set when the channel is unreachable
([[SPEC-002-leaderless-dispatch-loop#REQ-008]].a). The channel is a transport
and a room, and it decides nothing about what is true.

The **cost of admission changes shape entirely**, and this is what makes
[[SPEC-002-leaderless-dispatch-loop#ADR-012]] worth adopting rather than merely
possible. Under a swarm-is-one-theory model, admitting a peer costs two
ceremonies. Under the real model it costs one more per programme the peer ever
works. A troupe of six machines across five specifications is thirty
human-mediated ceremonies, each needed at the right moment. That does not
scale, and a rule that does not scale is a rule that gets bypassed.

The channel MUST be private and end-to-end encrypted. A beacon names a DID and
the tasks it holds, and an intent names a task and a rank. A public room
therefore publishes the troupe's whole work allocation, across every theory, to
anyone who opens it. Encryption is not what makes the design correct, because
the roster filter of [[SPEC-002-leaderless-dispatch-loop#REQ-017]].b does that.
A fan-out room turns a coordination hint into a disclosure.

So the answer to "is the swarm a private channel?" is: the troupe is, and the
work is not.

### ADR-012: Peers exchange theory invites over the channel

Status: **accepted** by the steward, 2026-08-10, over a drafted reversal. The
conditions below are normative, and the residual risk is recorded rather than
resolved.

Context: [[SPEC-002-leaderless-dispatch-loop#ADR-011]] makes theory admission a
recurring cost rather than a one-off. A troupe working N specifications pays
two admission ceremonies plus N theory ceremonies per peer. Each of the N needs
a human at both ends, at the moment a new theory opens.

Decision: channel admission stays human and happens once per peer. Theory
admission is then carried out by the peers themselves. A peer holding a DID and
a channel seat emits `join-request`. The peer ranking first for that requester
answers with `join-grant` on the routed transport. The requester redeems the
code with `elephant theory join`.

The economy is (2 + N) ceremonies per peer down to 2. Those two are paid once
for the troupe's whole life, rather than once per programme of work.

**Neither carrier in this composition is clean, and this decision picks the
bounded one.** An earlier revision of this ADR concluded from the same facts
that admission stays human. Its reasoning is preserved here, because it is the
strongest case against what follows:

| Carrier | Who can read the code | Bounded by |
|---|---|---|
| encrypted chat room | every current channel member | nothing |
| routed transport | the hub | single use, 120 s, own deployment |

A code in the room lets a current member mint a **second DID**. A party holding
two DIDs attempts under one and approves under the other. That defeats the
self-review defeater of [[SPEC-002-leaderless-dispatch-loop#REQ-007]] and
reduces Constitutional Principle 12 to an honour system. A code on the routed
transport is read by the hub, which is the troupe's own deployment. It lives at
most 120 s, and is spent the moment the joiner completes the exchange.

Conditions, all of which MUST hold together —
[[SPEC-002-leaderless-dispatch-loop#REQ-018]] states them normatively:

1. The requester is already a channel member. Channel admission is the trust
   root, and it is the only thing standing between an outsider and every theory
   the troupe will ever open.
2. The code rides the routed transport, point-to-point, addressed to the
   requester. Never the chat fan-out.
3. Exactly one peer answers, chosen by the ranking the loop already computes.
4. The TTL is at most 120 s, and the requester redeems immediately.
5. The code is never written to a room frame, a decision record, a journal
   record, a log line, or a theory assertion.

Residual risk, stated plainly. The hub sees the code inside its TTL window, and
no condition above removes that.
[[SPEC-002-leaderless-dispatch-loop#REQ-017]].c is the detection that survives
it: a second DID under one wire key is observable from signed beacons.

Upgrade path: a human-free, two-member, end-to-end encrypted carrier closes the
residual. Its recipient binding MUST prevent both another channel member and
the hub from reading the frame. The grant moves there WHEN one exists. That move
needs its own security review, and it is not a configuration flag on this
dispatch loop.

Alternatives rejected: posting the code to the room, for the sockpuppet reason
above. Keeping theory admission human, which is correct on the security argument and
fails on the operational one. A per-theory ceremony at the moment each theory
opens is the friction that gets bypassed rather than followed.
Deriving a theory roster from channel membership automatically. Elephant has no
mechanism for it, and it makes theory authority depend on the hub's view of a
room. A bearer capability in configuration, which makes every peer's
identity forgeable by anyone holding the file.

### ADR-013: One loop process per theory

Status: proposed.

Context: [[SPEC-002-leaderless-dispatch-loop#ADR-011]] puts a peer in several
theories at once. Something has to say whether that is one process reasoning
over N theories, or N processes reasoning over one each.

Decision: one `troupe run` per theory. A peer in three theories runs three
processes, each with its own configuration, sharing one hark daemon, one
Elephant daemon, and one host-local execution-capacity lock.

Consequences: every contract keeps one `theory` in scope. None acquires a
cross-theory rank or coordination state. The native execution-capacity lock
applies only before a local promise. One developer host still runs at most one
default coding attempt even across several theories. A failed loop affects one
theory and releases its capacity lock. Its peers see the beacon stop; other
theories carry on. Liveness is per process, and the beacon names its theory.

Alternatives rejected: one process over N theories needs three new things. It
needs a cross-theory rank, a cross-theory coordination budget, and an answer to
which theory's merge wins for one integration ref. Each is a mechanism bought
for no requirement. A global process budget is also rejected. The native lock
is enough because capacity is local. Election stays leaderless and per theory.

### ADR-014: An attempt, not a task, is the evidence subject

Status: proposed.

Context: a task can have duplicate or replacement attempts. Circus isolates
those worktrees and records them separately. The preceding theory vocabulary
attached verifier, reviewer, silence, and completion evidence to the task or
peer alone. That lets one approval authorise another attempt. It also lets past
peer silence make future work reclaimable.

Decision: Troupe's lifecycle facts use `(theory, circus attempt-id)` as their
subject. The verifier and reviewer unify on that attempt's exact Git tree.
Verification authorises a merge. Only Circus's recorded successful merge derives
task completion. A liveness observation likewise names one abandoned attempt.

Consequences: duplicate work remains safe, but evidence cannot be confused. The
theory is not a premature mirror of Circus's accepted state. It records the
integration result, including a merge failure after review. This adds append-only
evidence records to Troupe. It does not expand Circus, which owns worktrees and
run records.

Alternatives rejected: task-scoped facts conflate duplicate attempts. Peer-scoped
silence persists across future attempts. Marking `completed` at verification
time is invalid because review is not integration.

## Verification Strategy

Selected techniques, by system characteristic:

| Characteristic | Technique | Scope |
|---|---|---|
| Pure decision core | Property-based testing + mutation testing | `elect`, `rank`, `live` |
| Election agreement across replicas | Property-based testing | `rank` |
| Theory rules | Example-based testing against a live theory | `CON-002` |
| CBCL dialect at a trust boundary | `cbcl-cli verify` + fuzzing | `CON-003` |
| External admission boundary | Fuzzing + prohibited-action tests | `CON-005` |
| Bounded-response obligations | Temporal monitoring over `OBS-###` traces | `REQ-003`, `REQ-010`, `REQ-013`, `NFR-006` |
| Configuration recognition | Negative-input testing | `CON-007` |
| Whole loop | Integration testing against real Elephant, hark, and Circus | all |

Integration tests run against real programs, not mocks. Constitutional
Principle 5 asks for realistic environments. Every failure mode this
specification cares about — a sync interval, a dropped socket, an advisory
merge lock — is precisely what a mock removes.

Review tier: **Tier 2** overall — this specification governs an irreversible
merge and defines a schema three repositories read.
[[SPEC-002-leaderless-dispatch-loop#CON-005]] escalates to **Tier 1**, because
its failure admits forged evidence. Tier 2 requires cross-model review plus human
review; Tier 1 requires cross-model adversarial review plus a human domain
expert. Neither has run. No `ADR-###` above is ratified until they do.

The comprehension gate is REQUIRED, because this specification is Tier 2 and
was synthesised by an AI agent. A fresh-context reader is given only the
Orientation block. That reader restates the intent, and predicts the behaviour
of a swarm of three when one peer dies. It then names the artefact deciding
whether a webhook can prove a test passed.

## Tests

Every test attributes its target requirement. Numbering follows the requirement
it verifies where the correspondence is one to one.

| ID | Validates | Type | What it checks |
|---|---|---|---|
| TEST-001 | REQ-001.a | positive | Two peers with different identities run byte-identical procedures |
| TEST-002 | REQ-001.b | negative-input | A configuration carrying a `role` key is rejected at start-up |
| TEST-003 | REQ-001.c | prohibited-action | No claim is issued in response to a message from another peer |
| TEST-004 | REQ-002.a,b | positive | `rank` returns the documented order for a known peer set |
| TEST-005 | REQ-002.b | property | For all task and peer sets, every peer computes the same order |
| TEST-006 | REQ-002.b | property | Removing one peer changes at most that peer's share of ranks |
| TEST-007 | REQ-002.c | prohibited-action | `rank` performs no I/O, asserted by the purity lint and a syscall trace |
| TEST-008 | REQ-002.f | negative-input | A candidate with an outstanding commitment yields `Skip`, never `Claim` |
| TEST-009 | REQ-003.a,d; NFR-002 | temporal | Beacons continue at the stated period throughout a 10 min attempt |
| TEST-010 | REQ-003.c | positive | A peer past the silence threshold is excluded from the live set |
| TEST-011 | REQ-004.a,b | positive | Silence of attempt A yields `(attempt-silent A T)` and `(reclaimable X A)` |
| TEST-012 | REQ-004.c | prohibited-action | No `elephant retract` is issued against another signer's statement |
| TEST-013 | REQ-004.e | prohibited-action | No `(attempt-silent A T)` is asserted while the channel is unreachable |
| TEST-014 | REQ-004.d,f | scope-invariant | Reclaiming creates one new attempt and a silence of A never reclaims later B |
| TEST-015 | REQ-005.a | negative-input | A message hark reports as unrecognised produces no action |
| TEST-016 | REQ-005.b | prohibited-action | The implementation contains no S-expression reader, asserted by source scan |
| TEST-017 | REQ-005.c | positive | Elephant is invoked with the returned token array, verbatim |
| TEST-018 | REQ-005.d | negative-input | A well-formed CBCL message of an unknown shape is discarded and counted |
| TEST-019 | REQ-006.a | prohibited-action | The driver's environment contains no theory identity, wire key, or handle |
| TEST-020 | REQ-006.b | prohibited-action | A driver transcript claiming success produces no assertion |
| TEST-021 | REQ-006.c | positive | Every asserted literal traces to a preserved verifier output file |
| TEST-022 | REQ-006.a | scope-invariant | The driver process reads no file under `ELEPHANT_HOME` or hark's config dir |
| TEST-023 | REQ-007.a,c,e | positive | A distinct reviewer of the exact attempt/tree yields `(verified A)` |
| TEST-024 | REQ-007.c | negative-output | Reviewer equal to attempter defeats `(verified A)` |
| TEST-025 | REQ-007.b | prohibited-action | A peer issues no self-naming `review-approved A P D` assertion |
| TEST-026 | REQ-007.d | prohibited-action | No merge is attempted while `(merge-authorized A)` is underivable |
| TEST-027 | REQ-008.a,b | positive | Channel loss switches the live set to the roster and widens the window |
| TEST-028 | REQ-008.c | prohibited-action | Theory loss produces no claim |
| TEST-029 | REQ-008.d | positive | Each mode transition writes one record |
| TEST-030 | REQ-009.a,b | positive | A peer at its limit skips and keeps beaconing |
| TEST-031 | REQ-009.a | negative-output | No decision returns `Claim` above the limit |
| TEST-032 | REQ-010.a,b | positive | A halt is journalled before election and in-flight tasks are reported within one period |
| TEST-033 | REQ-010.b,d | prohibited-action | No claim follows a halt, including after process restart |
| TEST-034 | REQ-010.a | prohibited-action | A halt sends no signal to a running attempt |
| TEST-035 | REQ-010.c,e | positive | Only the halting signer can resume the named halt and replay changes nothing |
| TEST-036 | REQ-011.a | positive | An untrusted-external message yields exactly one `discovered` literal |
| TEST-037 | REQ-011.b | prohibited-action | No evidence-family literal follows an untrusted-external message |
| TEST-038 | REQ-011.b | scope-invariant | The corpus gains exactly one statement per admitted external event |
| TEST-039 | REQ-011.c | positive | The source tag is recorded with the admitted literal |
| TEST-040 | REQ-012.a,c | positive | The prompt contains the description, the acceptance, and the sentinel path |
| TEST-041 | REQ-012.b | prohibited-action | The source contains no provider name, flag, or prompt template |
| TEST-042 | REQ-013.a,c | positive | A decision record is written and its id reaches the evidence reference |
| TEST-043 | REQ-013.b | prohibited-action | No promise is issued without a preceding record |
| TEST-044 | REQ-013.a | temporal | The record's write completes before the promise process starts |
| TEST-045 | NFR-001 | positive | Claim latency at the 95th percentile is within 2 s for eight peers |
| TEST-046 | NFR-003 | positive | The window matches the channel round trip and widens on channel loss |
| TEST-047 | NFR-004 | positive | Duplicate claims stay within 1 % over a 24 h soak |
| TEST-048 | NFR-005 | positive | Loop overhead stays within 5 % of attempt wall clock |
| TEST-049 | NFR-006 | temporal | A reclaimable task is claimed within 90 s only while all stated progress guards hold |
| TEST-050 | CON-001 | property | `elect` is total and returns exactly one variant for every input |
| TEST-051 | CON-002 | positive | Every coined family has a declaration and consumer, asserted by `elephant vocab` |
| TEST-052 | CON-003 | positive | `cbcl-cli verify` accepts both dialects, and R5 shape checks pass for four liveness performatives |
| TEST-055 | CON-003 | negative-input | A `peer-beacon` missing `:instance`, `:seq`, or `:policy` is refused as a `shape_violation` |
| TEST-056 | CON-003 | negative-input | A resume missing the named halt is refused as a `shape_violation` or `causal_violation` |
| TEST-057 | REQ-014.a,b | positive | stdout carries only the event stream or the record; every message is on stderr |
| TEST-058 | REQ-014.c | positive | `run` output is byte-identical under a tty, a pipe, and a file |
| TEST-059 | REQ-014.d; REQ-018.a | prohibited-action | No Troupe command invokes membership, halt/resume, merge, accept, or theory-query capabilities directly |
| TEST-060 | REQ-014.e | negative-output | `status` exits non-zero when the peer is unfit to claim |
| TEST-061 | CON-008 | positive | `once` performs exactly one pass and exits 0 on an empty candidate set |
| TEST-062 | CON-008 | prohibited-action | `status` and `decisions` issue no promise, assert, emit, or spawn |
| TEST-063 | CON-008 | positive | The first SIGTERM stops new claims and lets the in-flight attempt finish |
| TEST-064 | CON-008 | scope-invariant | The second SIGTERM leaves the Circus attempt running and its record intact |
| TEST-065 | CON-008 | negative-input | Each documented exit code is produced by its stated condition |
| TEST-066 | CON-008 | positive | `once --task <id>` restricts the candidate set to that one task |
| TEST-067 | REQ-015.a | positive | The founder and a later joiner produce identical decisions from identical inputs |
| TEST-068 | REQ-015.b | prohibited-action | No decision path reads the theory's creating signer |
| TEST-069 | REQ-015.c | negative-input | A configuration naming a founder, coordinator, or primary is rejected |
| TEST-070 | REQ-016.b | scope-invariant | A second `seed` adds exactly zero statements to the corpus |
| TEST-071 | REQ-016.c | prohibited-action | `seed` writes no task, promise, or evidence literal |
| TEST-072 | REQ-016.d | negative-output | A peer on a half-seeded theory returns `Skip`, never `Claim` |
| TEST-073 | CON-009; REQ-018.c | positive | Human Stage A then B then C leaves `troupe status` exiting 0 on every peer |
| TEST-074 | CON-009 | negative-output | With `ELEPHANT_SYNC_INTERVAL` unset, `troupe status` exits non-zero |
| TEST-075 | CON-009 | positive | Two peers seeded and joined agree on `closure fingerprint` |
| TEST-076 | REQ-017.b | negative-input | A beacon from a DID off the theory roster changes no peer's rank |
| TEST-077 | REQ-017.b | scope-invariant | The live peer set contains exactly the roster members heard from |
| TEST-078 | REQ-017.c | positive | A channel-roster divergence is reported once per transition |
| TEST-079 | REQ-017.d,e | prohibited-action | Troupe execution invokes no channel admission or membership-removal command |
| TEST-080 | REQ-018.a | negative-input | A `join-request` from a DID with no channel seat is refused |
| TEST-081 | REQ-018.b | positive | Exactly one peer of N answers a given `join-request` |
| TEST-082 | REQ-018.c | positive | The grant is sent on the routed transport, addressed to the requester |
| TEST-083 | REQ-018.d | prohibited-action | No code appears in a room frame, decision record, journal record, log line, or assertion |
| TEST-084 | REQ-020.a | positive | An intent includes one decision, attempt, policy, and bounded expiry |
| TEST-085 | REQ-021.b | positive | A beacon and decision record carry the same policy fingerprint |
| TEST-086 | REQ-022.a | positive | The capacity slot is acquired before the promise process starts |
| TEST-087 | REQ-017.b | positive | A beacon for another theory changes no rank in this theory |
| TEST-053 | CON-007 | negative-input | Each missing REQUIRED key, including `repository_id`, exits non-zero naming that key |
| TEST-054 | CON-007 | negative-input | A `silence_threshold` below three beacon periods is rejected |
| TEST-088 | REQ-019.a,b,c; CON-010 | positive | Attempt, verifier, review, and merge records bind one theory, Circus attempt, tree, and policy |
| TEST-089 | REQ-019.d,e | negative-output | Review alone and a rejected merge never derive `(completed X)` or authorise B |
| TEST-090 | REQ-020.a,b; CON-003 | positive | Intent expiry and beacon instance/sequence survive a sender restart |
| TEST-091 | REQ-020.c,d | negative-input | A duplicate, expired, or superseded frame cannot alter rank, eligibility, or control |
| TEST-092 | REQ-021.a,b | positive | Equivalent configurations derive the same policy fingerprint |
| TEST-093 | REQ-021.c | prohibited-action | A policy mismatch reports once and issues no claim, review, or merge |
| TEST-094 | REQ-022.a,b,d | positive | Two theory loops on one host share one capacity slot and release it on terminal exit |
| TEST-095 | REQ-022.c | prohibited-action | A capacity loser issues no promise or Circus spawn |
| TEST-096 | REQ-023.a,c | review | A fresh reviewer states the credential-independence limit and checks human admission evidence |
| TEST-097 | CON-010; REQ-010 | positive | Replaying the control journal restores precisely the unmatched signed halts |
| TEST-098 | CON-010 | negative-input | A malformed, duplicate, or unwritable durable record produces no claim or control transition |
| TEST-099 | REQ-018.d | scope-invariant | Admitting one peer adds exactly one roster entry and no other state |
| TEST-100 | REQ-018.e | negative-input | A grant with a TTL above 120 s is refused before it is sent |
| TEST-101 | REQ-018.f | prohibited-action | No peer admits to the channel, and none removes a member from either plane |

Mutation testing is REQUIRED on the pure core, because tests and implementation
are synthesised together and the Red Gate cannot be strictly enforced. The kill
rate threshold is 90 %, matching the sibling repositories.

Adversarial testing is REQUIRED before this specification leaves `draft`, and
it targets the specification rather than the code. The adversary works from the
requirements alone. Four places to look are named. A claim path that needs no
decision record. A message that reaches an assertion without passing hark. A
reviewer that is the attempter under a different spelling. A degraded mode that
claims work.

## Observability

| ID | Signal | Kind | Why |
|---|---|---|---|
| OBS-001 | `troupe.decision` — one event per decision, with outcome, task, attempt, rank, policy, and record id | log | Makes every claim attributable and every skip explicable |
| OBS-002 | `troupe.live_peers` — observed live peer count | metric | Distinguishes a solitary peer from a peer with a broken beacon reader |
| OBS-003 | `troupe.duplicate_claims` — claims that collided with a live claim | metric | The rate NFR-004 bounds |
| OBS-004 | `troupe.reclaims` — reclaimable attempts seen and replacement attempts claimed | metric | The latency NFR-006 bounds |
| OBS-005 | `troupe.reviews` — exact-attempt/tree reviews requested, given, and refused as self-review | metric | Shows Principle 12 operating, not merely specified |
| OBS-006 | `troupe.degraded` — one event per degraded-mode transition, with the plane | log | The only way an operator learns a plane is gone |
| OBS-007 | `troupe.attempts` — attempts started, verified, authorised, rejected, failed, merged | metric | The throughput the whole design exists to produce |
| OBS-008 | `troupe.halt` — one event per durable halt and matching resume, with id and signer | log | An authority claim with no record is not auditable |
| OBS-009 | `troupe.external_admitted` — external events admitted, by source tag | metric | Makes the trust boundary of CON-005 visible in production |
| OBS-010 | `troupe.identity_binding` — one event per DID-to-wire-key binding observed, and per violation | log | The sockpuppet detection of REQ-017.c is only a control if it is recorded |
| OBS-011 | `troupe.artifact` — one event per verified tree and merge result, with attempt and policy | log | Binds theory evidence to one immutable worktree and integration result |
| OBS-012 | `troupe.capacity` — local capacity acquired, withheld, and released, by execution host | metric | Shows that per-theory loops cannot overrun a developer host |
| OBS-013 | `troupe.policy_mismatch` — one event per incompatible roster peer or local policy | log | Makes an otherwise hidden verifier or integration-ref disagreement actionable |

Every signal is written unbuffered to stdout as a structured event. A peer routes
none of its own logs, rotates none of them, and stores none of them. Temporal
properties are evaluated against the trace those signals emit, which is why the
transport is not part of any signal's definition.

## Gate Evidence Record

```yaml
phase: 1
gates:
  - gate: "All requirements unambiguous, verifiable, atomic"
    mechanism: "reviewer: adversarial review, fresh context (Tier 2)"
    result: unverified
    evidence: "no mechanism available; escalated to HOC 2026-08-09"
  - gate: "Controls extracted as prohibitive or hold REQ"
    mechanism: "self-check against the Prohibitive and Hold Requirements template"
    result: pass
    evidence: "REQ-001.b/.c, REQ-004.c/.e, REQ-005.b, REQ-006.a/.b, REQ-007.b, REQ-008.c, REQ-011.b are prohibitive; REQ-010 is a hold"
  - gate: "No speculative requirements; each traces to a user goal or finding"
    mechanism: "trace audit against users/peer-agent/happy-paths findings"
    result: pass
    evidence: "REQ-004←Finding 1, NFR-003/ADR-003←Finding 2, REQ-011←Finding 3, REQ-007←Finding 4, OBS-002←Finding 5"
  - gate: "Synthetic user simulation run and findings converted"
    mechanism: "simulation record"
    result: pass
    evidence: "users/peer-agent/happy-paths.md#synthetic-user-findings — 5 findings, all converted"
phase: 2
gates:
  - gate: "Simplicity Ladder rung recorded per capability"
    mechanism: "ADR presence"
    result: pass
    evidence: "ADR-002 records rung 4/5; no capability settles at rung 6"
  - gate: "Capability placement justified"
    mechanism: "ADR presence"
    result: pass
    evidence: "ADR-002 and ADR-008"
  - gate: "Grammar declared for every CON accepting external input"
    mechanism: "cargo run -q -p cbcl-cli -- verify < specs/dialects/swarm-liveness.cbcl"
    result: pass
    evidence: "2026-08-10: cbcl-rs cbcl-cli passed R1,R2,R3,R5; 4 performatives, depth=8, expansion=1024, time=30ms"
  - gate: "Composition-first feasibility check performed per capability"
    mechanism: "search of sibling dialect registries before coining vocabulary"
    result: pass
    evidence: "cbcl-rs dialects/usdd-work.cbcl adopted unchanged for the work-attempt trail; only 4 new performatives coined"
  - gate: "Tests derived from requirements (REQ -> TEST)"
    mechanism: "pi audit over the Tests table"
    result: pass
    evidence: "2026-08-10 trace audit: every one of 23 REQs and 6 NFRs appears in the 98-row TEST table; v0.2 controls map through TEST-088 to TEST-098"
  - gate: "Orientation block present, Controls exhaustive"
    mechanism: "reviewer: comprehension gate + adversarial review"
    result: unverified
    evidence: "no mechanism available; escalated to HOC 2026-08-09"
  - gate: "Amendment Channels block present"
    mechanism: "presence check"
    result: pass
    evidence: "## Amendment Channels, with four Hard stops named"
  - gate: "CON-002 rules derive the specified conclusions on a live reasoner"
    mechanism: "elephant 0.1.7 — define + assert + status against temporary theory s2v2"
    result: pass
    evidence: "2026-08-10: 10 families, 8 rules loaded verbatim. verified m1 +d; verified a1 -D (self-review); no verified m2 (duplicate inherits nothing); completed models absent until merge-succeeded, then +d; reclaimable ui u1 +d then -D once a replacement attempt merged"
  - gate: "Controlled language: no lowercase BCP 14 keyword, no deprecated term, sentences within limit"
    mechanism: "tools/usdd-lint.sh over the spec, both user-profile pages, and all 9 concept pages"
    result: pass
    evidence: "2026-08-10: SPEC-002 0 errors, 23 sentence warnings; amended Troupe docs 0 errors; peer happy path 0 errors, 1 sentence warning"
  - gate: "Wikilink traceability; no dead links"
    mechanism: "zetl check --dead-links --fail-on error"
    result: pass
    evidence: "0 dead links, 0 syntax errors, exit 0; 9 concept pages authored to clear the backlog this spec created"
  - gate: "Every coined family has a consumer and a declaration"
    mechanism: "elephant vocab -t spec002check"
    result: pass
    evidence: "2026-08-10: temporary theory reports 10 coined families; task-description/2 and task-acceptance/2 are detached by design for elephant next, every other family has a rule consumer or shell reader"
  - gate: "Cross-model adversarial review (Tier 2; Tier 1 for CON-005)"
    mechanism: "reviewer: distinct model family, fresh context"
    result: unverified
    evidence: "not run; escalated to HOC 2026-08-09"
```

Phase 2 is **not complete**. No gate is `fail`, and three are `unverified`. This
specification stays `draft` until the Tier 2 cross-model adversarial review, the
Tier 1 review of [[SPEC-002-leaderless-dispatch-loop#CON-005]], and the
comprehension gate have run. Each `unverified` gate names HOC as its escalation
owner, per PROTO-001's rule that an `unverified` gate without an owner blocks the
phase outright.

## Changelog

<details>
<summary>Revision history</summary>

- 0.2.0 — amendments pending review. Troupe remains an external composition,
  not a Circus feature. Attempt-scoped evidence and merge-derived completion
  replace task/peer-scoped lifecycle facts. Reclamation is scoped to one
  attempt. Replay-safe fields, compatible policy, host capacity, and durable
  control/evidence journals are added. Theory admission is peer-issued over the
  routed transport under ADR-012, and credential identity's independence limit
  is recorded.
- 0.1.0 — first draft. Normative. Defines the leaderless dispatch loop composing
  Circus, Elephant, hark, cbcl-bus, cbcl-rs, and any coding-agent CLI. Not
  reviewed; not approved; no ADR ratified.
</details>
