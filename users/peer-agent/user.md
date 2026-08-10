# User: Peer Agent

Role: one member of a [[concepts/leaderless-swarm|leaderless swarm]] working a
single [[concepts/elephant|Elephant]] theory. It holds no authority over any
other member and receives none. It decides what to attempt, attempts it in
isolation, and contributes signed evidence back to the theory.

A peer is not a smaller [[users/lead-agent/user|lead agent]]. The lead agent
decomposes work and hands attempts to workers it selected. A peer has nobody to
hand work to and nobody to receive it from. Every question the lead answered by
deciding, a peer answers by reading the theory and computing over its own
identity.

Goals:

- Find work nobody is already doing, without asking permission and without a
  coordinator to ask.
- Start an attempt within seconds of the work becoming available.
- Avoid duplicating a live peer's attempt, and pick up a dead peer's abandoned
  attempt without a human noticing it stalled.
- Keep a human in the room informed, and stop when that human says stop.
- Contribute evidence that a peer who did not run the attempt can check.

Constraints:

- Technical proficiency: high. It reads JSON, drives Git directly, and reads
  [[concepts/cbcl|CBCL]] S-expressions that another program has already
  recognised for it.
- Environment: one developer machine among several. Each machine runs one peer.
  There is no shared filesystem, no shared database, and no shared process.
- Identity: it holds its own [[concepts/did-crdt|did:crdt]] identity in the
  theory and its own Ed25519 wire key on the [[concepts/cbcl-bus|bus]]. It never
  holds another peer's, and it cannot enrol one — the
  [[concepts/spake2|SPAKE2]] join ceremony blocks on a human.
- Authority: it cannot assign work, revoke another peer's promise, or retract
  another peer's statement. Elephant makes all three cryptographically
  impossible, not merely discouraged.
- Trust: it treats a coding agent's own report of success as transport, never as
  verification — [[SPEC-001-circus-agent-harness#REQ-004]]. It treats an
  external event as a discovery, never as evidence —
  [[SPEC-002-leaderless-dispatch-loop#REQ-011]].
- Partition: it expects the bus to drop and the theory to stop syncing, and it
  expects both to happen without warning.

Daily workflow:

1. Beacon its liveness into the swarm channel, and read the beacons of others.
2. Read the theory for available work, and for work a silent peer abandoned.
3. Compute its own claim rank over the live peer set, and wait its turn.
4. Promise the work, then run a coding-agent CLI in an isolated
   [[concepts/git-worktree|Git worktree]] through [[concepts/circus|Circus]].
5. Run the acceptance verifier itself, and record what the verifier produced.
6. Assert the evidence into the theory, and merge only what the verifier passed.
7. Review a peer's attempt when the theory derives that a review is owed.

Detail: [[users/peer-agent/happy-paths]] ·
[[SPEC-002-leaderless-dispatch-loop#User Profile]]
