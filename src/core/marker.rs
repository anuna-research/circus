//! The child-exit marker — how Circus tells a signalled completion from a
//! natural one.
//!
//! `SPEC-001-circus-agent-harness#OBS-003` reports `completion_method`, and
//! getting it right is harder than it looks. withdone unlinks the sentinel on
//! both exit paths and its exit code is the sentinel value on one and the
//! child's own code on the other, so after the fact the two are
//! indistinguishable from withdone alone.
//!
//! The first design inferred the answer from the *absence* of a marker written
//! after the child returned. A spike against `codex exec` showed why that is
//! wrong: a non-interactive agent writes the sentinel and then immediately
//! ends its turn, so the child exits before withdone notices the write. The
//! marker gets written, and a properly signalled attempt was reported as
//! `child-exit` with no sentinel value.
//!
//! So the wrapper reads the sentinel itself, at the one moment nothing else
//! can have removed it yet: immediately after the child returns. The marker
//! carries what it saw.
//!
//! ```text
//! code=0
//! wrote=yes
//! value=0
//! ```
//!
//! The format is three `key=value` lines, recognised in full before any field
//! is used, like every other input Circus accepts.

use super::RecognitionError;

/// What the wrapper observed when the child returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    /// The child's own exit code.
    pub code: i32,
    /// Whether the sentinel existed at that instant.
    pub wrote: bool,
    /// What the sentinel contained, when it existed and held an integer.
    pub value: Option<i32>,
}

/// Recognise a marker.
pub fn recognise(text: &str) -> Result<Marker, RecognitionError> {
    const P: &str = "child-exit-marker";
    let mut code = None;
    let mut wrote = None;
    let mut value = None;

    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let Some((key, raw)) = line.split_once('=') else {
            return Err(RecognitionError::new(
                P,
                format!("line without `=`: {line:?}"),
            ));
        };
        match key.trim() {
            "code" => {
                code = Some(raw.trim().parse().map_err(|_| {
                    RecognitionError::new(P, format!("`code` is not an integer: {raw:?}"))
                })?);
            }
            "wrote" => {
                wrote = Some(match raw.trim() {
                    "yes" => true,
                    "no" => false,
                    other => {
                        return Err(RecognitionError::new(
                            P,
                            format!("`wrote` is not yes or no: {other:?}"),
                        ));
                    }
                });
            }
            // The sentinel holds whatever the agent echoed. A non-integer
            // there is the agent's mistake, not a malformed marker, so it is
            // recorded as absent rather than refused.
            "value" => value = raw.trim().parse().ok(),
            other => {
                return Err(RecognitionError::new(P, format!("unknown key {other:?}")));
            }
        }
    }

    Ok(Marker {
        code: code.ok_or_else(|| RecognitionError::new(P, "no `code`"))?,
        wrote: wrote.ok_or_else(|| RecognitionError::new(P, "no `wrote`"))?,
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_a_signalled_completion() {
        // The codex case the spike found: the agent wrote the sentinel and
        // then ended its turn.
        let m = recognise("code=0\nwrote=yes\nvalue=0\n").unwrap();
        assert_eq!(
            m,
            Marker {
                code: 0,
                wrote: true,
                value: Some(0)
            }
        );
    }

    #[test]
    fn recognises_a_natural_exit() {
        let m = recognise("code=3\nwrote=no\nvalue=\n").unwrap();
        assert_eq!(
            m,
            Marker {
                code: 3,
                wrote: false,
                value: None
            }
        );
    }

    #[test]
    fn recognises_a_non_zero_signal() {
        let m = recognise("code=0\nwrote=yes\nvalue=2\n").unwrap();
        assert_eq!(m.value, Some(2));
        assert!(m.wrote);
    }

    #[test]
    fn tolerates_a_sentinel_the_agent_filled_with_nonsense() {
        // `wrote` is still true: the agent did signal. Only the value is lost.
        let m = recognise("code=0\nwrote=yes\nvalue=done!\n").unwrap();
        assert!(m.wrote);
        assert_eq!(m.value, None);
    }

    #[test]
    fn ignores_blank_lines_and_surrounding_space() {
        let m = recognise("\n code = 7 \n\nwrote = no \n").unwrap();
        assert_eq!(m.code, 7);
        assert!(!m.wrote);
    }

    #[test]
    fn refuses_a_marker_missing_a_required_field() {
        assert!(recognise("wrote=yes\n").is_err(), "no code");
        assert!(recognise("code=0\n").is_err(), "no wrote");
        assert!(recognise("").is_err(), "empty");
    }

    #[test]
    fn refuses_a_malformed_marker() {
        for bad in [
            "code=nope\nwrote=yes\n",
            "code=0\nwrote=maybe\n",
            "code=0\nwrote=yes\nsurprise=1\n",
            "no equals sign here",
        ] {
            assert!(recognise(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn a_negative_child_code_is_recognised() {
        // A child killed by a signal has no exit code; the shell reports
        // 128+signal, but a wrapper on some platforms can report -1.
        assert_eq!(recognise("code=-1\nwrote=no\n").unwrap().code, -1);
    }
}
