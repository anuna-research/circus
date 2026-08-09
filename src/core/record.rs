//! The run record codec — `SPEC-001-circus-agent-harness#CON-007`.
//!
//! This is the only durable state Circus owns. Every command's pre-condition
//! on attempt state is a predicate over it, and Circus both writes and reads
//! it, which is why the format is a contract with a schema rather than an
//! implementation detail.
//!
//! The types below *are* the declared JSON Schema. `deny_unknown_fields` is
//! `additionalProperties: false`; a non-`Option` field without a default is
//! `required`; `Option` is a nullable or absent property. Two constraints
//! serde cannot express — the `schema_version` const and the `attempt` range —
//! are checked in [`recognise`], so a caller never holds a record that passed
//! some of the schema.

use serde::{Deserialize, Serialize};

use super::RecognitionError;
use super::grammar::AttemptId;
use super::paths::AttemptPaths;
use super::verifier::VerifierRecord;

/// The schema version this build reads and writes.
pub const SCHEMA_VERSION: u32 = 1;

/// Where an attempt sits in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// A worktree and branch exist; nothing has been launched.
    Prepared,
    /// The driver ran to a terminal outcome. Not a judgement about the work.
    Completed,
    /// A verifier passed and evidence was attested.
    Accepted,
    /// Acceptance was requested and refused.
    Rejected,
    /// Merged into the recorded integration ref.
    Merged,
    /// Cleanup did not complete, per REQ NFR-002.c.
    Failed,
}

/// How the launched command ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionMethod {
    /// The worker wrote the sentinel. The explicit channel.
    Sentinel,
    /// The child exited without writing. A transport outcome, nothing more.
    ChildExit,
    /// Nothing has been launched yet.
    #[default]
    None,
}

/// The acceptance decision. Never derived from a transport outcome —
/// see `SPEC-001-circus-agent-harness#REQ-004.b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Accepted,
    Rejected,
}

/// How a merge ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeOutcome {
    Merged,
    Conflict,
}

/// The result of the one merge an attempt may have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Merge {
    pub result: MergeOutcome,
    #[serde(default)]
    pub commit: Option<String>,
}

/// One external program Circus executed.
///
/// `SPEC-001-circus-agent-harness#REQ-007.c` requires this list, and
/// `#OBS-004` is it. Recording every invocation is what turns the Elephant
/// prohibition (`#REQ-006.c`) and the composition boundary (`#REQ-007.a`) from
/// review-only claims into something `TEST-011` can check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalProgram {
    /// The program as named on the command line.
    pub name: String,
    /// Where `PATH` resolution found it.
    pub resolved_path: String,
    /// Its exit status, or null if it was never reaped.
    pub exit_status: Option<i32>,
}

/// Lifecycle instants, RFC 3339 with a `Z` offset.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timestamps {
    #[serde(default)]
    pub prepared_at: Option<String>,
    #[serde(default)]
    pub launched_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub decided_at: Option<String>,
    #[serde(default)]
    pub merged_at: Option<String>,
}

/// One attempt's complete record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    pub schema_version: u32,
    pub attempt_id: String,
    pub repository: String,
    pub task: String,
    pub attempt: u16,
    pub state: State,
    pub integration_ref: String,
    pub worktree_path: String,
    pub branch: String,

    #[serde(default)]
    pub pane_name: Option<String>,
    #[serde(default)]
    pub log_path: Option<String>,
    #[serde(default)]
    pub sentinel_path: Option<String>,
    #[serde(default)]
    pub sentinel_value: Option<i32>,
    #[serde(default)]
    pub transport_exit_code: Option<i32>,
    #[serde(default)]
    pub completion_method: CompletionMethod,
    #[serde(default)]
    pub process_group_residue: u32,
    #[serde(default)]
    pub external_programs: Vec<ExternalProgram>,
    #[serde(default)]
    pub verifier: Option<VerifierRecord>,
    /// Circus's own copy of the verifier output, distinct from
    /// `verifier.output_path`, which names where the caller left the original.
    /// This is the one that survives — `#REQ-014`.
    #[serde(default)]
    pub verifier_log: Option<String>,
    #[serde(default)]
    pub verifier_log_truncated: bool,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub decision: Option<Decision>,
    #[serde(default)]
    pub merge: Option<Merge>,

    pub timestamps: Timestamps,
}

impl RunRecord {
    /// A freshly prepared attempt.
    ///
    /// Takes the identity and the derived paths rather than nine strings, so
    /// no caller can transpose the task for the branch or the worktree for the
    /// log.
    pub fn prepared(
        id: &AttemptId,
        paths: &AttemptPaths,
        repository: String,
        integration_ref: String,
        prepared_at: String,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            attempt_id: id.to_string(),
            repository,
            task: id.task.to_string(),
            attempt: id.attempt.get(),
            state: State::Prepared,
            integration_ref,
            worktree_path: paths.worktree.to_string_lossy().into_owned(),
            branch: paths.branch.clone(),
            pane_name: None,
            log_path: None,
            sentinel_path: None,
            sentinel_value: None,
            transport_exit_code: None,
            completion_method: CompletionMethod::None,
            process_group_residue: 0,
            external_programs: Vec::new(),
            evidence_refs: Vec::new(),
            verifier: None,
            verifier_log: None,
            verifier_log_truncated: false,
            decision: None,
            merge: None,
            timestamps: Timestamps {
                prepared_at: Some(prepared_at),
                ..Timestamps::default()
            },
        }
    }

    /// Whether this attempt may be merged — `#REQ-005.a`.
    pub fn is_accepted(&self) -> bool {
        self.state == State::Accepted && self.decision == Some(Decision::Accepted)
    }
}

/// Recognise a run record from bytes.
pub fn recognise(bytes: &[u8]) -> Result<RunRecord, RecognitionError> {
    const P: &str = "run-record";

    let record: RunRecord = serde_json::from_slice(bytes)
        .map_err(|e| RecognitionError::new(P, format!("not a conforming JSON object: {e}")))?;

    if record.schema_version != SCHEMA_VERSION {
        return Err(RecognitionError::new(
            P,
            format!(
                "schema_version {} is not {SCHEMA_VERSION}",
                record.schema_version
            ),
        ));
    }
    if !(1..=9999).contains(&record.attempt) {
        return Err(RecognitionError::new(
            P,
            format!("attempt {} is outside 1..=9999", record.attempt),
        ));
    }
    Ok(record)
}

/// Serialise a run record. Pretty-printed with a trailing newline, because a
/// human reads this file at least as often as Circus does.
pub fn serialise(record: &RunRecord) -> String {
    let mut s = serde_json::to_string_pretty(record).expect("RunRecord always serialises");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RunRecord {
        let id = AttemptId::recognise("model/1").unwrap();
        let roots = crate::core::paths::Roots {
            state: "/repo/.git/circus".into(),
            worktrees: "/work".into(),
            repository: "circus".into(),
        };
        let paths = crate::core::paths::attempt_paths(&roots, &id);
        RunRecord::prepared(
            &id,
            &paths,
            "circus".into(),
            "circus/spec-001".into(),
            "2026-08-08T00:00:00Z".into(),
        )
    }

    fn full() -> RunRecord {
        let mut r = sample();
        r.state = State::Merged;
        r.pane_name = Some("circus-model-1".into());
        r.log_path = Some("/repo/.git/circus/model/1/transcript.log".into());
        r.sentinel_path = Some("/repo/.git/circus/model/1/sentinel".into());
        r.sentinel_value = Some(0);
        r.transport_exit_code = Some(0);
        r.completion_method = CompletionMethod::Sentinel;
        r.process_group_residue = 0;
        r.external_programs = vec![ExternalProgram {
            name: "git".into(),
            resolved_path: "/usr/bin/git".into(),
            exit_status: Some(0),
        }];
        r.verifier = Some(VerifierRecord {
            command: vec!["cargo".into(), "test".into()],
            exit_code: 0,
            output_path: "/tmp/v.txt".into(),
        });
        r.verifier_log = Some("/repo/.git/circus/model/1/verifier.log".into());
        r.verifier_log_truncated = true;
        r.evidence_refs = vec!["theory:spec-001/q1".into()];
        r.decision = Some(Decision::Accepted);
        r.merge = Some(Merge {
            result: MergeOutcome::Merged,
            commit: Some("abc123".into()),
        });
        r.timestamps.merged_at = Some("2026-08-08T00:05:00Z".into());
        r
    }

    #[test]
    fn roundtrips_a_minimal_record() {
        // TEST-028
        let r = sample();
        assert_eq!(recognise(serialise(&r).as_bytes()).unwrap(), r);
    }

    #[test]
    fn roundtrips_a_fully_populated_record() {
        // TEST-028
        let r = full();
        assert_eq!(recognise(serialise(&r).as_bytes()).unwrap(), r);
    }

    #[test]
    fn roundtrips_every_state_and_completion_method() {
        // TEST-028 — the generator covers every enum value.
        for state in [
            State::Prepared,
            State::Completed,
            State::Accepted,
            State::Rejected,
            State::Merged,
            State::Failed,
        ] {
            for method in [
                CompletionMethod::Sentinel,
                CompletionMethod::ChildExit,
                CompletionMethod::None,
            ] {
                for attempt in [1u16, 4321, 9999] {
                    let mut r = full();
                    r.state = state;
                    r.completion_method = method;
                    r.attempt = attempt;
                    assert_eq!(
                        recognise(serialise(&r).as_bytes()).unwrap(),
                        r,
                        "{state:?}/{method:?}/{attempt}"
                    );
                }
            }
        }
    }

    #[test]
    fn roundtrips_with_optional_objects_null() {
        // TEST-028 — both the populated and the absent shape of every nullable.
        let mut r = full();
        r.verifier = None;
        r.verifier_log = None;
        r.verifier_log_truncated = false;
        r.merge = None;
        r.decision = None;
        r.sentinel_value = None;
        r.transport_exit_code = None;
        r.pane_name = None;
        r.log_path = None;
        r.sentinel_path = None;
        r.evidence_refs.clear();
        r.external_programs.clear();
        assert_eq!(recognise(serialise(&r).as_bytes()).unwrap(), r);
    }

    #[test]
    fn rejects_an_unknown_property() {
        let mut v: serde_json::Value = serde_json::from_str(&serialise(&sample())).unwrap();
        v["surprise"] = serde_json::json!(1);
        assert!(recognise(v.to_string().as_bytes()).is_err());
    }

    #[test]
    fn rejects_a_missing_required_property() {
        for key in [
            "schema_version",
            "attempt_id",
            "repository",
            "task",
            "attempt",
            "state",
            "integration_ref",
            "worktree_path",
            "branch",
            "timestamps",
        ] {
            let mut v: serde_json::Value = serde_json::from_str(&serialise(&sample())).unwrap();
            v.as_object_mut().unwrap().remove(key);
            assert!(
                recognise(v.to_string().as_bytes()).is_err(),
                "accepted a record with no `{key}`"
            );
        }
    }

    #[test]
    fn rejects_a_foreign_schema_version() {
        let mut v: serde_json::Value = serde_json::from_str(&serialise(&sample())).unwrap();
        v["schema_version"] = serde_json::json!(2);
        assert!(recognise(v.to_string().as_bytes()).is_err());
    }

    #[test]
    fn rejects_an_attempt_outside_the_range() {
        for n in [0i64, 10_000] {
            let mut v: serde_json::Value = serde_json::from_str(&serialise(&sample())).unwrap();
            v["attempt"] = serde_json::json!(n);
            assert!(recognise(v.to_string().as_bytes()).is_err(), "accepted {n}");
        }
    }

    #[test]
    fn rejects_an_unknown_enum_value() {
        for (key, bad) in [("state", "half-done"), ("completion_method", "vibes")] {
            let mut v: serde_json::Value = serde_json::from_str(&serialise(&full())).unwrap();
            v[key] = serde_json::json!(bad);
            assert!(
                recognise(v.to_string().as_bytes()).is_err(),
                "accepted {key}={bad}"
            );
        }
    }

    #[test]
    fn rejects_malformed_json() {
        for bad in ["", "{", "[]", "null", "not json"] {
            assert!(recognise(bad.as_bytes()).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn is_accepted_requires_both_the_state_and_the_decision() {
        let mut r = sample();
        assert!(!r.is_accepted());
        r.state = State::Accepted;
        assert!(!r.is_accepted(), "state alone must not admit a merge");
        r.decision = Some(Decision::Accepted);
        assert!(r.is_accepted());
        r.state = State::Rejected;
        assert!(!r.is_accepted(), "decision alone must not admit a merge");
    }

    #[test]
    fn a_prepared_record_carries_its_identity_and_the_moment_it_was_made() {
        let r = sample();
        assert_eq!(r.schema_version, SCHEMA_VERSION);
        assert_eq!(r.attempt_id, "model/1");
        assert_eq!(r.task, "model");
        assert_eq!(r.attempt, 1);
        assert_eq!(r.repository, "circus");
        assert_eq!(r.integration_ref, "circus/spec-001");
        assert_eq!(r.branch, "circus/model/1");
        assert!(r.worktree_path.contains("circus-model-1"));
        assert_eq!(
            r.timestamps.prepared_at.as_deref(),
            Some("2026-08-08T00:00:00Z"),
            "NFR-003: the record must carry the instant it was prepared"
        );
    }

    #[test]
    fn a_prepared_record_names_no_outcome() {
        // REQ-004.b at the type level: nothing about a fresh attempt implies
        // acceptance.
        let r = sample();
        assert_eq!(r.decision, None);
        assert_eq!(r.verifier, None);
        assert_eq!(r.merge, None);
        assert_eq!(r.completion_method, CompletionMethod::None);
        assert!(!r.is_accepted());
    }
}
