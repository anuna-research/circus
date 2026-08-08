//! Recogniser for the verifier record of
//! `SPEC-001-circus-agent-harness#CON-003`.
//!
//! ```json
//! {
//!   "type": "object",
//!   "additionalProperties": false,
//!   "required": ["command", "exit_code", "output_path"],
//!   "properties": {
//!     "command":     { "type": "array", "minItems": 1,
//!                      "items": { "type": "string" } },
//!     "exit_code":   { "type": "integer", "minimum": 0, "maximum": 255 },
//!     "output_path": { "type": "string", "pattern": "^/" }
//!   }
//! }
//! ```
//!
//! The typed struct below is that schema: `deny_unknown_fields` is
//! `additionalProperties: false`, non-`Option` fields are `required`, and the
//! field types are the declared types. The three constraints serde cannot
//! carry — `minItems`, the integer range, and the path pattern — are checked
//! in [`recognise`] before it returns, so recognition completes in one step
//! and no caller ever holds a partially-validated record.
//!
//! This is the highest-trust input Circus accepts: it is the evidence half of
//! the acceptance gate. Nothing here interprets it. Interpretation is
//! [`super::decision::acceptance_decision`], which takes the typed value.

use serde::{Deserialize, Serialize};

use super::RecognitionError;

/// A verifier run, as attested by the lead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifierRecord {
    /// The argv of the verifier the lead ran. Never executed by Circus.
    pub command: Vec<String>,
    /// Its exit code, in `0..=255`.
    pub exit_code: i32,
    /// Absolute path to its captured output.
    pub output_path: String,
}

/// Recognise a verifier record from bytes.
///
/// Returns the typed record or the production it failed. On any error nothing
/// has been read out of the document — the caller cannot act on half of it.
pub fn recognise(bytes: &[u8]) -> Result<VerifierRecord, RecognitionError> {
    const P: &str = "verifier-record";

    let record: VerifierRecord = serde_json::from_slice(bytes)
        .map_err(|e| RecognitionError::new(P, format!("not a conforming JSON object: {e}")))?;

    if record.command.is_empty() {
        return Err(RecognitionError::new(
            P,
            "`command` must have at least one element",
        ));
    }
    if !(0..=255).contains(&record.exit_code) {
        return Err(RecognitionError::new(
            P,
            format!("`exit_code` {} is outside 0..=255", record.exit_code),
        ));
    }
    if !record.output_path.starts_with('/') {
        return Err(RecognitionError::new(P, "`output_path` is not absolute"));
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"command":["cargo","test"],"exit_code":0,"output_path":"/tmp/v.txt"}"#;

    #[test]
    fn accepts_a_conforming_record() {
        let r = recognise(GOOD.as_bytes()).unwrap();
        assert_eq!(r.command, ["cargo", "test"]);
        assert_eq!(r.exit_code, 0);
        assert_eq!(r.output_path, "/tmp/v.txt");
    }

    #[test]
    fn rejects_an_unknown_property() {
        // TEST-023 — additionalProperties: false
        let bad = r#"{"command":["x"],"exit_code":0,"output_path":"/t","extra":1}"#;
        assert!(recognise(bad.as_bytes()).is_err());
    }

    #[test]
    fn rejects_a_missing_required_property() {
        // TEST-023
        for bad in [
            r#"{"exit_code":0,"output_path":"/t"}"#,
            r#"{"command":["x"],"output_path":"/t"}"#,
            r#"{"command":["x"],"exit_code":0}"#,
        ] {
            assert!(recognise(bad.as_bytes()).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn rejects_an_exit_code_outside_the_range() {
        // TEST-023
        for bad in [
            r#"{"command":["x"],"exit_code":300,"output_path":"/t"}"#,
            r#"{"command":["x"],"exit_code":-1,"output_path":"/t"}"#,
        ] {
            assert!(recognise(bad.as_bytes()).is_err(), "accepted {bad}");
        }
        assert!(
            recognise(r#"{"command":["x"],"exit_code":255,"output_path":"/t"}"#.as_bytes()).is_ok()
        );
    }

    #[test]
    fn rejects_an_empty_command() {
        let bad = r#"{"command":[],"exit_code":0,"output_path":"/t"}"#;
        assert!(recognise(bad.as_bytes()).is_err());
    }

    #[test]
    fn rejects_a_relative_output_path() {
        let bad = r#"{"command":["x"],"exit_code":0,"output_path":"v.txt"}"#;
        assert!(recognise(bad.as_bytes()).is_err());
    }

    #[test]
    fn rejects_wrong_types_and_malformed_json() {
        for bad in [
            r#"{"command":"cargo test","exit_code":0,"output_path":"/t"}"#,
            r#"{"command":["x"],"exit_code":"0","output_path":"/t"}"#,
            r#"{"command":["x"],"exit_code":0,"output_path":null}"#,
            r#"{"command":[1],"exit_code":0,"output_path":"/t"}"#,
            "not json at all",
            "",
            "[]",
        ] {
            assert!(recognise(bad.as_bytes()).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn roundtrips() {
        let r = recognise(GOOD.as_bytes()).unwrap();
        let out = serde_json::to_vec(&r).unwrap();
        assert_eq!(recognise(&out).unwrap(), r);
    }
}
