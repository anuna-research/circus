# Sentinel file

A sentinel file is a unique, initially absent path minted by Circus for one
attempt. The worker writes the promised success value only after recording its
evidence and completing the requested local work.

The path lies outside the attempt worktree, under the state root of
[[SPEC-001-circus-agent-harness#ADR-006]]. A sentinel inside the worktree
appears as an untracked file in the worker's diff, and reaches the merge.
[[SPEC-001-circus-agent-harness#REQ-003]].b forbids it.

Circus recognises the sentinel path in the prompt file as a literal substring
and nothing more. It never composes the instruction that tells the worker when
to write — [[SPEC-001-circus-agent-harness#REQ-003]].d.
