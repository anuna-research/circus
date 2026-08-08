# User: Lead Agent

Role: the coordinating agent for one [[concepts/elephant|Elephant]] theory. It
owns task decomposition, selects an already-enrolled worker, and decides whether
independently verified evidence justifies acceptance.

Goals:

- Run an independent coding-agent CLI without sharing its own working tree.
- Know when a worker has finished, without polling output or guessing.
- Accept a change only on evidence the theory and a local verifier support.
- Keep a rejected attempt inspectable rather than lost.

Constraints:

- Technical proficiency: high. It reads JSON, drives Git directly, and attaches
  to a [[concepts/tmux|tmux]] pane when it needs to watch a worker.
- Environment: one developer machine. No daemon, no remote service, no
  scheduler.
- Identity: it holds its own Elephant identity. It never receives a worker's,
  and Circus never sees either — [[SPEC-001-circus-agent-harness#REQ-006]].
- Trust: it treats an agent exit code as transport, never as verification —
  [[SPEC-001-circus-agent-harness#REQ-004]].

Daily workflow:

1. Read Elephant readiness and outstanding commitments.
2. Decompose the ready goal into task attempts.
3. Prepare, launch, verify, and accept or reject each attempt through Circus.
4. Merge accepted work, and leave everything else on disk for review.

Detail: [[users/lead-agent/happy-paths]] ·
[[SPEC-001-circus-agent-harness#User Profile]]
