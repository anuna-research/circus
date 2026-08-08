# Happy Path: Run One Task Attempt

Preconditions:

- The lead holds an Elephant identity and an approved theory with one ready
  goal.
- The chosen worker already completed the enrolment ceremony and holds a
  distinct identity. Circus plays no part in that — see
  [[SPEC-001-circus-agent-harness#REQ-006]].
- Git, [[concepts/tmux|tmux]], and [[concepts/withdone|withdone]] are on `PATH`.
- The integration branch exists locally.

Steps:

1. Query the theory for a ready goal → `elephant status --json` names one
   literal with no unmet dependency.
2. Prepare an attempt → `circus prepare --task model --integration
   circus/spec-001` prints a run record carrying `attempt_id`, `worktree_path`,
   `branch`, and `sentinel_path`
   ([[SPEC-001-circus-agent-harness#CON-001]]).
3. Write the prompt file, embedding the printed `sentinel_path` literally →
   the file exists and contains that path verbatim. Instruct the worker to
   write the sentinel only after it asserts its evidence.
4. Launch the driver → `circus launch --attempt model/1 --prompt ./p.md --
   <driver>` starts one named pane and returns when the sentinel is written
   ([[SPEC-001-circus-agent-harness#CON-002]]).
5. Read the worker's evidence → `elephant explain <literal>` and
   `elephant describe <literal> --json` show a signed assertion by the worker's
   identity.
6. Run the verifier and record its result → a JSON file with `command`,
   `exit_code`, and `output_path`
   ([[SPEC-001-circus-agent-harness#CON-003]]).
7. Accept the attempt → `circus accept --attempt model/1 --verifier-record
   ./v.json --evidence <ref>` exits 0 and records `decision: accepted`.
8. Merge → `circus merge --attempt model/1 --into circus/spec-001` creates one
   merge commit ([[SPEC-001-circus-agent-harness#CON-004]]).

Postconditions:

- The integration branch carries exactly one new merge commit.
- The run record holds the verifier command, its exit code, every evidence
  reference, the decision, and the merge result.
- The attempt worktree, branch, and transcript log still exist. Removal is the
  operator's action — [[SPEC-001-circus-agent-harness#NFR-001]].

Failure modes:

| Situation | Expected response | Recovery |
|---|---|---|
| Prompt omits the sentinel path | Exit 64, no pane, no driver process ([[SPEC-001-circus-agent-harness#TEST-004]]) | Re-read `sentinel_path` from the run record and rewrite the prompt |
| tmux, withdone, or Git absent | Exit 127 naming the program ([[SPEC-001-circus-agent-harness#CON-006]]) | Install the program; the attempt stays `prepared` and relaunches |
| Driver exits without writing the sentinel | Attempt completes with `completion_method: child-exit` and is not accepted | Read the transcript log, then prepare attempt 2 |
| Worker's process group outlives the 10 s deadline | `state: failed`, non-zero `process_group_residue` ([[SPEC-001-circus-agent-harness#NFR-002]]) | Inspect the residue by hand; the worktree and log are preserved |
| Verifier fails | Exit 1, `decision: rejected`, attempt preserved ([[SPEC-001-circus-agent-harness#TEST-007]]) | Read the verifier output, then prepare the next attempt |
| Evidence reference omitted | Exit 64, no decision recorded ([[SPEC-001-circus-agent-harness#TEST-021]]) | Supply the reference; Circus never resolves it, so the lead checks it first |
| Verifier record malformed | Exit 64, no field read, no decision ([[SPEC-001-circus-agent-harness#TEST-023]]) | Regenerate the record against the CON-003 schema |
| Relaunch of a completed attempt | Exit 64, no second pane ([[SPEC-001-circus-agent-harness#TEST-027]]) | Prepare a new attempt; attempts are immutable once launched |
| Merge target differs from the recorded ref | Exit 64, no Git merge ([[SPEC-001-circus-agent-harness#TEST-026]]) | Merge into the ref the attempt was prepared from |
| Merge conflicts | Exit 1, integration ref unchanged ([[SPEC-001-circus-agent-harness#TEST-010]]) | Resolve in the attempt worktree by hand; Circus never rebases |
| Two attempts merge at once | Both succeed, serialised by the advisory lock ([[SPEC-001-circus-agent-harness#TEST-024]]) | None needed |
