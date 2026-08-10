# Happy Paths: Peer Agent

Every path below belongs to the [[users/peer-agent/user|peer agent]] and is
governed by [[SPEC-002-leaderless-dispatch-loop]]. Each peer elects and starts
its own work; no other peer grants or allocates it. A different peer can later
review one exact attempt, but that review never grants the original claim.

## Happy Path: Claim and complete one task

Preconditions:

- The peer holds a theory identity and is a roster member.
- The peer holds a wire key and has joined the swarm channel.
- The [[concepts/elephant|Elephant]] daemon is running with sync enabled.
- The [[concepts/hark|hark]] daemon is running and the channel handle is active.
- At least one task is `ready`, documented, and unpromised.

Steps:

1. The peer emits a beacon → every live peer records it within one beacon
   period.
2. The peer reads `elephant next --json` → one documented candidate, with its
   `explain`, `describe`, and `promise` token arrays.
3. The peer computes its claim rank over the live peer set → an integer from
   zero to the live-peer count minus one.
4. The peer waits `rank × contention-window` → no other peer has promised the
   candidate in that interval.
5. The peer re-reads the theory and the channel → the candidate is still
   unpromised and unannounced.
6. The peer acquires its host-local capacity slot → no other local Troupe loop
   can promise a concurrent default attempt.
7. The peer writes and flushes a decision record carrying a fresh attempt
   identifier and policy fingerprint → the intended claim is attributable.
8. The peer announces that decision and attempt on the channel → every live peer
   sees the advisory intent within one second or ignores it on expiry.
9. The peer promises the task for that exact attempt → every live peer sees the
   promise within one sync interval.
10. The peer composes a prompt from the candidate's description, its acceptance
   criterion, and `circus instruction` → a prompt carrying the literal sentinel
   path.
11. The peer runs `circus spawn` with a driver → an isolated worktree, a branch,
   and a named pane.
12. The coding agent finishes and writes the sentinel → Circus records its
    terminal attempt outcome, not task completion.
13. The peer runs the acceptance verifier in the attempt worktree → a command,
    an exit code, and a preserved output file.
14. The peer records the exact Git tree, runs `circus accept` with that verifier
    record → Circus records
    `accepted`.
15. The peer asserts verifier evidence for that attempt and tree → the theory
    derives nothing yet, because review is still owed.
16. A different peer reviews that same tree and asserts its approval → the
    theory derives `(merge-authorized A)`.
17. The peer runs `circus merge` for A → the accepted branch lands on the
    integration ref, serialised against every other peer's merge.
18. The peer records Circus's merge result and asserts merge success → the
    theory derives `(completed X)`.

Postconditions: the task is `completed` in the theory, and the branch is merged.
The attempt worktree and its evidence remain on disk, and the peer's commitment
reads `fulfilled`.

Failure modes:

- **A peer promised first.** Step 5 finds an outstanding commitment. The peer
  drops the candidate and returns to step 2. Nothing is asserted and nothing is
  wasted.
- **Two peers promised.** Both attempts run. Both are isolated. The first merge
  lands; the second fails its precondition and is recorded as a rejected merge.
  The duplication stays visible in the corpus, per
  [[SPEC-002-leaderless-dispatch-loop#ADR-004]].
- **The verifier fails.** Step 14 records `rejected`. The peer asserts
  `(failed A)` and leaves the worktree for inspection. It does not assert
  completion, and it does not merge.
- **The merge fails.** The peer records the failed result for A. It does not
  assert merge success, so the theory does not complete X; a later attempt must
  earn its own verifier and review evidence.
- **The coding agent claims success without evidence.** The sentinel proves the
  process ended, never that the work is correct. Step 11 decides, and step 11
  alone.
- **The coding agent hangs.** Circus's cleanup deadline
  ([[SPEC-001-circus-agent-harness#NFR-002]]) records residue as a failed
  attempt. The peer asserts `(failed X)` and stops beaconing that task.

## Happy Path: Reclaim a silent peer's task

Preconditions: a task carries an outstanding promise, and its promising peer has
emitted no beacon for longer than the silence threshold.

Steps:

1. The peer observes the owner of attempt A silent → it asserts
   `(attempt-silent A T)` with its local observation time.
2. The theory derives `(reclaimable X A)` → the task reappears as available work
   through a path `elephant next` cannot offer.
3. The peer computes its rank over the live peer set, which no longer contains
   the silent peer → the ranks redistribute with no message exchanged.
4. Steps 4 onward of the first path run unchanged.

Postconditions: the task is attempted by a live peer under a new attempt
identifier. The silent peer's promise
stays outstanding and visible in the corpus for ever, because nobody but its
signer can retract it.

Failure modes:

- **The silent peer was merely partitioned.** It returns, finds its task
  completed by another peer, and discards its own attempt. Its own work is not
  lost — the worktree stays on disk — and no correctness property depended on it
  being alone.
- **The silence was a bus outage, not a death.** Every peer looks silent to
  every other. The degraded mode of
  [[SPEC-002-leaderless-dispatch-loop#REQ-008]] applies: the peer falls back to
  the theory roster as its live set and stops asserting silence.

## Happy Path: A human stops the swarm

Preconditions: a human member is present in the swarm channel.

Steps:

1. The human sends a halt into the channel → the hub fans it to every peer.
2. Each peer recognises it as a valid halt in the swarm dialect → the peer
   writes the named, signed halt to its durable control journal.
3. Each peer stops claiming new work → attempts already running continue to
   their own terminal state.
4. Each peer reports what it is still holding → the human sees exactly what is
   in flight.

Postconditions: no new claim is made, including after a peer restarts. Every
in-flight attempt reaches its own terminal state and its evidence is preserved.

Failure modes:

- **A peer is partitioned from the bus.** It never sees the halt and keeps
  claiming. The halt is best-effort by construction, and
  [[SPEC-002-leaderless-dispatch-loop#REQ-010]] says so rather than promising
  a guarantee the transport cannot make.
- **The halt is read as an amendment.** A channel message stops work. It never
  changes an obligation. See
  [[SPEC-002-leaderless-dispatch-loop#Amendment Channels]].

## Synthetic user findings

Simulation metadata: model Claude Opus 5, profile
[[users/peer-agent/user|peer agent]], paths above, 2026-08-09. The synthetic user
worked from the profile and the draft requirements, with no implementation
available.

### Finding: The dead peer's promise is unretractable

- **Step:** Reclaim path, step 2.
- **Category:** Gap
- **Description:** `elephant next` withholds a task that carries an outstanding
  `(completed X)` commitment, and `elephant retract` accepts only the signer's
  own statement. A peer that dies mid-attempt therefore removes its task from
  every peer's `next` output permanently.
- **User impact:** The swarm silently stops making progress on that task. No
  error surfaces, because an empty `next` is successful idle.
- **Proposed resolution:** Specify a second discovery path that does not run
  through `next`, keyed on observed silence rather than on commitment state.
- **Trace:** created [[SPEC-002-leaderless-dispatch-loop#REQ-004]] and
  [[SPEC-002-leaderless-dispatch-loop#CON-002]].

### Finding: The contention window cannot be sized to the theory

- **Step:** First path, steps 4 to 7.
- **Category:** Ambiguity
- **Description:** A promise reaches other peers only after an Elephant sync
  interval, which defaults to 30 s. Sizing the contention window to that
  interval makes the last-ranked peer of a swarm of six wait more than two
  minutes before it claims anything.
- **User impact:** The peer appears idle while work is available, and the
  operator concludes the swarm is broken.
- **Proposed resolution:** Size the window to the channel's latency, not the
  theory's. Announce the intent on the bus, and keep the promise as the
  authoritative record.
- **Trace:** created [[SPEC-002-leaderless-dispatch-loop#ADR-003]] and
  [[SPEC-002-leaderless-dispatch-loop#NFR-003]].

### Finding: A webhook can forge evidence

- **Step:** Not in any path; discovered while walking the channel's inputs.
- **Category:** Gap
- **Description:** The bus mints an external webhook into a CBCL message and
  fans it into the channel. A peer that converts such a message into an evidence
  literal lets any party holding a webhook secret drive the theory to
  `completed`.
- **User impact:** A merge lands on evidence nobody verified.
- **Proposed resolution:** Prohibit the conversion. An external event becomes a
  discovery literal and never an evidence literal.
- **Trace:** created [[SPEC-002-leaderless-dispatch-loop#REQ-011]].

### Finding: The peer reviews its own attempt

- **Step:** First path, step 14.
- **Category:** Sequencing issue
- **Description:** Nothing in the first draft stopped the claiming peer from
  asserting its own review approval. That assertion satisfies the completion
  rule while defeating the purpose of Principle 12.
- **User impact:** A single peer merges unreviewed work while the theory reports
  it as verified.
- **Proposed resolution:** Carry the reviewer in the literal and unify it against
  the attempting peer, so self-review is unrepresentable rather than merely
  discouraged.
- **Trace:** created [[SPEC-002-leaderless-dispatch-loop#REQ-007]] and
  [[SPEC-002-leaderless-dispatch-loop#CON-002]].

### Finding: A swarm of one is indistinguishable from a stalled swarm

- **Step:** First path, step 3.
- **Category:** Error path gap
- **Description:** A peer whose live set contains only itself always ranks zero
  and always claims immediately. That is correct behaviour and also exactly what
  a peer with a broken beacon reader does.
- **User impact:** A misconfigured peer works alone and duplicates everything,
  and nothing distinguishes it from a legitimately solitary peer.
- **Proposed resolution:** Make the observed live-peer count an observability
  signal rather than an inference.
- **Trace:** created [[SPEC-002-leaderless-dispatch-loop#OBS-002]].
