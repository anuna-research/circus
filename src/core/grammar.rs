//! Recognisers for the shared productions of `SPEC-001-circus-agent-harness`.
//!
//! ```abnf
//! task         = LOWER *62( LOWER / DIGIT / "-" )
//! attempt-n    = NZDIGIT *3DIGIT                 ; 1..9999
//! attempt-id   = task "/" attempt-n
//! evidence-ref = 1*128( ALPHA / DIGIT / "-" / "_" / "." / ":" / "/" )
//! abs-path     = "/" *VCHAR
//! ```
//!
//! Each production is a regular language, so each recogniser is a byte-class
//! scan with a length bound. That is the minimal recogniser for the declared
//! grammar, not an ad-hoc parser: there is no state beyond the position, and
//! no input is acted on before the scan completes.
//!
//! `ref` is deliberately absent from this module. Git owns that language, and
//! re-implementing its rules here would create the parser differential
//! Constitutional Principle 14 prohibits. What lives here is
//! [`ref_prefilter`], which only *rejects*; `git check-ref-format` remains the
//! authority on what is accepted. See its documentation.

use super::RecognitionError;

/// A recognised task identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Task(String);

/// A recognised attempt number, in `1..=9999`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptN(u16);

/// A recognised `task/attempt` pair — the handle every command after
/// `prepare` takes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptId {
    pub task: Task,
    pub attempt: AttemptN,
}

/// A recognised evidence reference. Opaque to Circus by
/// `SPEC-001-circus-agent-harness#REQ-004.e`; the grammar exists only to bound
/// its length and character set, so it cannot carry a shell metacharacter or a
/// Git option into a downstream command.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceRef(String);

impl Task {
    /// `task = LOWER *62( LOWER / DIGIT / "-" )`
    pub fn recognise(input: &str) -> Result<Self, RecognitionError> {
        const P: &str = "task";
        let bytes = input.as_bytes();
        match bytes.first() {
            None => return Err(RecognitionError::new(P, "empty")),
            Some(b) if !b.is_ascii_lowercase() => {
                return Err(RecognitionError::new(
                    P,
                    "first character must be an ASCII lowercase letter",
                ));
            }
            Some(_) => {}
        }
        if bytes.len() > 63 {
            return Err(RecognitionError::new(
                P,
                format!("{} characters exceeds the 63 character bound", bytes.len()),
            ));
        }
        for (i, b) in bytes.iter().enumerate().skip(1) {
            if !(b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-') {
                return Err(RecognitionError::new(
                    P,
                    format!("character {i} is not a lowercase letter, digit, or hyphen"),
                ));
            }
        }
        Ok(Self(input.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AttemptN {
    /// `attempt-n = NZDIGIT *3DIGIT` — the decimal spelling matters, so `01`
    /// is refused even though it denotes a number in range.
    pub fn recognise(input: &str) -> Result<Self, RecognitionError> {
        const P: &str = "attempt-n";
        let bytes = input.as_bytes();
        if bytes.is_empty() {
            return Err(RecognitionError::new(P, "empty"));
        }
        if bytes.len() > 4 {
            return Err(RecognitionError::new(P, "more than 4 digits"));
        }
        if bytes[0] == b'0' {
            return Err(RecognitionError::new(P, "leading zero"));
        }
        if !bytes.iter().all(u8::is_ascii_digit) {
            return Err(RecognitionError::new(P, "not all characters are digits"));
        }
        // In range by construction: 1..=4 digits with no leading zero is 1..=9999.
        Ok(Self(input.parse().expect("1..=4 digits parses as u16")))
    }

    pub fn get(self) -> u16 {
        self.0
    }

    /// Construct from a number. Returns `None` outside `1..=9999`, so the
    /// invariant holds however the value was produced.
    pub fn from_number(n: u16) -> Option<Self> {
        (1..=9999).contains(&n).then_some(Self(n))
    }

    /// The next attempt number, or `None` at the ceiling.
    pub fn next(self) -> Option<Self> {
        Self::from_number(self.0 + 1)
    }
}

impl std::fmt::Display for AttemptN {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AttemptId {
    /// `attempt-id = task "/" attempt-n`
    pub fn recognise(input: &str) -> Result<Self, RecognitionError> {
        const P: &str = "attempt-id";
        let Some((task, attempt)) = input.split_once('/') else {
            return Err(RecognitionError::new(P, "expected the form `task/attempt`"));
        };
        if attempt.contains('/') {
            return Err(RecognitionError::new(P, "more than one `/`"));
        }
        Ok(Self {
            task: Task::recognise(task)?,
            attempt: AttemptN::recognise(attempt)?,
        })
    }

    pub fn new(task: Task, attempt: AttemptN) -> Self {
        Self { task, attempt }
    }
}

impl std::fmt::Display for AttemptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.task, self.attempt)
    }
}

impl EvidenceRef {
    /// `evidence-ref = 1*128( ALPHA / DIGIT / "-" / "_" / "." / ":" / "/" )`
    pub fn recognise(input: &str) -> Result<Self, RecognitionError> {
        const P: &str = "evidence-ref";
        let bytes = input.as_bytes();
        if bytes.is_empty() {
            return Err(RecognitionError::new(P, "empty"));
        }
        if bytes.len() > 128 {
            return Err(RecognitionError::new(
                P,
                format!("{} characters exceeds the 128 character bound", bytes.len()),
            ));
        }
        for (i, b) in bytes.iter().enumerate() {
            let ok = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/');
            if !ok {
                return Err(RecognitionError::new(
                    P,
                    format!("character {i} is outside the permitted set"),
                ));
            }
        }
        Ok(Self(input.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EvidenceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A fail-closed prefilter for a Git ref name.
///
/// This is **not** a recogniser for the `ref` production. `git
/// check-ref-format --branch` is, and the shell calls it before any ref is
/// used. Two rules justify the split. Constitutional Principle 14 forbids a
/// second implementation of a language another component already parses, so
/// Git stays the authority. Principle 14 also demands the input be safe to
/// hand to that authority in the first place, and a value beginning `-` is
/// read by every Git subcommand as an option rather than a ref.
///
/// The filter therefore only ever *rejects*. Anything it passes is still
/// decided by Git, so it cannot widen what Circus accepts.
pub fn ref_prefilter(input: &str) -> Result<&str, RecognitionError> {
    const P: &str = "ref";
    if input.is_empty() {
        return Err(RecognitionError::new(P, "empty"));
    }
    if input.starts_with('-') {
        return Err(RecognitionError::new(
            P,
            "begins with `-`, which Git reads as an option",
        ));
    }
    if input.len() > 255 {
        return Err(RecognitionError::new(P, "longer than 255 characters"));
    }
    if let Some(i) = input
        .bytes()
        .position(|b| b.is_ascii_control() || b == 0x7f)
    {
        return Err(RecognitionError::new(
            P,
            format!("control character at byte {i}"),
        ));
    }
    Ok(input)
}

/// An absolute path, as the `abs-path` production requires.
pub fn abs_path(input: &str) -> Result<&std::path::Path, RecognitionError> {
    const P: &str = "abs-path";
    if !input.starts_with('/') {
        return Err(RecognitionError::new(P, "path is not absolute"));
    }
    Ok(std::path::Path::new(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_accepts_the_production() {
        for good in ["a", "model", "a-b-c", "x9", "a".repeat(63).as_str()] {
            assert!(Task::recognise(good).is_ok(), "rejected {good:?}");
        }
    }

    #[test]
    fn task_rejects_outside_the_production() {
        for bad in [
            "",
            "A",
            "9x",
            "-x",
            "a_b",
            "a b",
            "a/b",
            "café",
            &"a".repeat(64),
        ] {
            assert!(Task::recognise(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn attempt_n_accepts_the_production() {
        for (input, want) in [("1", 1u16), ("9", 9), ("10", 10), ("9999", 9999)] {
            assert_eq!(AttemptN::recognise(input).unwrap().get(), want);
        }
    }

    #[test]
    fn attempt_n_rejects_outside_the_production() {
        for bad in ["", "0", "01", "10000", "-1", "1a", " 1"] {
            assert!(AttemptN::recognise(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn attempt_id_roundtrips_through_its_display() {
        let id = AttemptId::recognise("model/12").unwrap();
        assert_eq!(id.to_string(), "model/12");
        assert_eq!(AttemptId::recognise(&id.to_string()).unwrap(), id);
    }

    #[test]
    fn attempt_id_rejects_malformed_pairs() {
        for bad in ["model", "model/", "/1", "model/1/2", "Model/1", "model/0"] {
            assert!(AttemptId::recognise(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn evidence_ref_bounds_length_and_charset() {
        assert!(EvidenceRef::recognise("theory:spec-001/q42").is_ok());
        assert!(EvidenceRef::recognise(&"a".repeat(128)).is_ok());
        for bad in ["", &"a".repeat(129), "a b", "a;rm -rf /", "a$(x)", "a\nb"] {
            assert!(EvidenceRef::recognise(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn ref_prefilter_rejects_option_shaped_and_control_input() {
        assert_eq!(ref_prefilter("main").unwrap(), "main");
        assert_eq!(ref_prefilter("circus/spec-001").unwrap(), "circus/spec-001");
        for bad in ["", "-f", "--upload-pack=x", "a\nb", "a\0b", "a\x7fb"] {
            assert!(ref_prefilter(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn ref_prefilter_length_bound_is_inclusive_at_255() {
        assert!(
            ref_prefilter(&"a".repeat(255)).is_ok(),
            "255 is within the bound"
        );
        assert!(ref_prefilter(&"a".repeat(256)).is_err(), "256 is past it");
    }

    #[test]
    fn abs_path_accepts_only_absolute_paths() {
        assert_eq!(abs_path("/tmp/x").unwrap(), std::path::Path::new("/tmp/x"));
        assert_eq!(abs_path("/").unwrap(), std::path::Path::new("/"));
        for bad in ["", "tmp/x", "./x", "../x", "~/x"] {
            assert!(abs_path(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn evidence_ref_is_carried_verbatim() {
        // REQ-004.d — the reference Circus records must be the one supplied,
        // byte for byte, since nothing downstream can check it.
        for s in [
            "theory:spec-001/q42",
            "sha256:deadbeef",
            "a",
            &"z".repeat(128),
        ] {
            let r = EvidenceRef::recognise(s).unwrap();
            assert_eq!(r.as_str(), s);
            assert_eq!(r.to_string(), s);
        }
    }

    #[test]
    fn task_and_attempt_are_carried_verbatim() {
        let t = Task::recognise("model-9").unwrap();
        assert_eq!(t.as_str(), "model-9");
        assert_eq!(t.to_string(), "model-9");
        assert_eq!(AttemptN::recognise("42").unwrap().to_string(), "42");
    }

    #[test]
    fn attempt_number_ceiling_is_closed() {
        assert_eq!(AttemptN::from_number(0), None);
        assert_eq!(AttemptN::from_number(10_000), None);
        assert_eq!(AttemptN::from_number(9999).unwrap().next(), None);
        assert_eq!(AttemptN::from_number(1).unwrap().next().unwrap().get(), 2);
    }
}
