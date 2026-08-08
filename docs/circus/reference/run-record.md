---
title: Run record
mode: reference
spec: SPEC-001
---

# Run record

One JSON document per attempt, at
`<git-common-dir>/circus/<task>/<attempt>/record.json`. It is the only durable
state Circus owns, and every command's pre-condition is a predicate over it.

Circus writes it atomically and recognises it in full before reading any field.
A record that fails the schema exits 65 and causes no state transition.

## Fields

| Field | Type | Default | Constraint |
|---|---|---|---|
| `schema_version` | integer | — | Exactly `1` |
| `attempt_id` | string | — | `task/attempt` |
| `repository` | string | — | Required |
| `task` | string | — | Required |
| `attempt` | integer | — | `1..=9999` |
| `state` | enum | — | `prepared` `completed` `accepted` `rejected` `merged` `failed` |
| `integration_ref` | string | — | The ref the attempt was prepared from |
| `worktree_path` | string | — | Absolute |
| `branch` | string | — | `circus/<task>/<attempt>` |
| `pane_name` | string or null | `null` | Set at launch |
| `log_path` | string or null | `null` | Outside every worktree |
| `sentinel_path` | string or null | `null` | Set at prepare; outside every worktree |
| `sentinel_value` | integer or null | `null` | The integer the worker wrote |
| `transport_exit_code` | integer or null | `null` | withdone's exit code |
| `completion_method` | enum | `none` | `sentinel` `child-exit` `none` |
| `process_group_residue` | integer | `0` | Live processes at the deadline |
| `external_programs` | array | `[]` | Every program Circus invoked |
| `verifier` | object or null | `null` | `command`, `exit_code`, `output_path` |
| `evidence_refs` | array of string | `[]` | Recorded verbatim, never resolved |
| `decision` | enum or null | `null` | `accepted` `rejected` |
| `merge` | object or null | `null` | `result`, `commit` |
| `timestamps` | object | — | RFC 3339 with a `Z` offset |

No other property is accepted at any level.

## `state`

| Value | Reached by |
|---|---|
| `prepared` | `prepare` succeeded |
| `completed` | `launch` reached a terminal outcome with an empty process group |
| `failed` | `launch` found live processes at the cleanup deadline |
| `accepted` | `accept` recorded a passing verifier and at least one reference |
| `rejected` | `accept` recorded a failing verifier |
| `merged` | `merge` created a merge commit |

A state is never derived from a transport exit code.

## `completion_method`

| Value | Meaning |
|---|---|
| `sentinel` | The worker wrote the sentinel; withdone ended the group |
| `child-exit` | The driver returned on its own without writing |
| `none` | Nothing has been launched |

`sentinel_value` is populated only on the `sentinel` path.

## `external_programs`

| Field | Type | Constraint |
|---|---|---|
| `name` | string | As named on the command line |
| `resolved_path` | string | Where `PATH` resolution found it |
| `exit_status` | integer or null | Null if never reaped |

This array is the signal OBS-004. It is what makes the Elephant prohibition
checkable from an artefact rather than from review: an entry named `elephant`
is a violation, and there is never one.

## `timestamps`

| Field | Written by |
|---|---|
| `prepared_at` | `prepare` |
| `launched_at` | `launch`, before the pane starts |
| `completed_at` | `launch`, after the outcome is known |
| `decided_at` | `accept` |
| `merged_at` | `merge`, on success only |

## Example

```json
{
  "schema_version": 1,
  "attempt_id": "model/1",
  "repository": "circus",
  "task": "model",
  "attempt": 1,
  "state": "merged",
  "integration_ref": "circus/spec-001",
  "worktree_path": "/work/circus-model-1",
  "branch": "circus/model/1",
  "pane_name": "circus-model-1",
  "log_path": "/work/circus/.git/circus/model/1/transcript.log",
  "sentinel_path": "/work/circus/.git/circus/model/1/sentinel",
  "sentinel_value": 0,
  "transport_exit_code": 0,
  "completion_method": "sentinel",
  "process_group_residue": 0,
  "external_programs": [
    { "name": "git", "resolved_path": "/usr/bin/git", "exit_status": 0 },
    { "name": "tmux", "resolved_path": "/opt/homebrew/bin/tmux", "exit_status": 0 }
  ],
  "verifier": {
    "command": ["cargo", "test"],
    "exit_code": 0,
    "output_path": "/tmp/verifier.txt"
  },
  "evidence_refs": ["theory:spec-001/q42"],
  "decision": "accepted",
  "merge": { "result": "merged", "commit": "9f2c1ab" },
  "timestamps": {
    "prepared_at": "2026-08-08T09:00:00Z",
    "launched_at": "2026-08-08T09:00:04Z",
    "completed_at": "2026-08-08T09:31:12Z",
    "decided_at": "2026-08-08T09:34:02Z",
    "merged_at": "2026-08-08T09:34:20Z"
  }
}
```

Contract: CON-007. Signals: OBS-001, OBS-002, OBS-003, OBS-004.
