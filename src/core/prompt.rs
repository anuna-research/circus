//! Prompt recognition — the whole of Circus's involvement with prompt content.
//!
//! `SPEC-001-circus-agent-harness#CON-002` declares:
//!
//! ```abnf
//! PROMPT   = *OCTET SENTINEL *OCTET
//! SENTINEL = <the attempt's sentinel path, as a literal byte substring>
//! ```
//!
//! That is a regular language, and a literal substring search recognises it
//! exactly. Nothing else about the file is parsed, which is what lets
//! `SPEC-001-circus-agent-harness#REQ-003.c` coexist with
//! `#REQ-003.d`: Circus checks one substring and forwards every byte
//! unchanged. The instruction telling the worker *when* to write the sentinel
//! is the lead's to author, and Circus never inspects it.

use std::path::Path;

use super::RecognitionError;

/// Whether a prompt admits the attempt's sentinel path.
///
/// The comparison is over bytes, not characters, and the needle is the path as
/// Circus will pass it to withdone. A prompt naming a *different* path — an
/// older attempt's, or one the caller invented — fails, because the literal
/// bytes differ.
pub fn recognise_prompt(prompt: &[u8], sentinel: &Path) -> Result<(), RecognitionError> {
    const P: &str = "PROMPT";
    let needle = sentinel.as_os_str().as_encoded_bytes();
    if needle.is_empty() {
        return Err(RecognitionError::new(P, "the attempt has no sentinel path"));
    }
    if contains(prompt, needle) {
        Ok(())
    } else {
        Err(RecognitionError::new(
            P,
            format!(
                "the prompt does not contain the literal sentinel path {}",
                sentinel.display()
            ),
        ))
    }
}

/// Literal byte-substring search. Written out rather than reached for, because
/// the standard library's `contains` is defined on `str` and a prompt is not
/// required to be UTF-8.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s() -> &'static Path {
        Path::new("/repo/.git/circus/model/1/sentinel")
    }

    #[test]
    fn accepts_a_prompt_carrying_the_literal_path() {
        let p = format!("Do the work. When done: echo 0 > {}\n", s().display());
        assert!(recognise_prompt(p.as_bytes(), s()).is_ok());
    }

    #[test]
    fn accepts_the_path_at_either_boundary() {
        assert!(recognise_prompt(s().as_os_str().as_encoded_bytes(), s()).is_ok());
        let lead = format!("x{}", s().display());
        let trail = format!("{}x", s().display());
        assert!(recognise_prompt(lead.as_bytes(), s()).is_ok());
        assert!(recognise_prompt(trail.as_bytes(), s()).is_ok());
    }

    #[test]
    fn rejects_a_prompt_without_the_path() {
        // TEST-004
        assert!(recognise_prompt(b"Do the work and stop.", s()).is_err());
        assert!(recognise_prompt(b"", s()).is_err());
    }

    #[test]
    fn rejects_a_near_miss_path() {
        // A different attempt's sentinel is not this attempt's sentinel.
        let other = "/repo/.git/circus/model/2/sentinel";
        assert!(recognise_prompt(other.as_bytes(), s()).is_err());
        let truncated = "/repo/.git/circus/model/1/sentine";
        assert!(recognise_prompt(truncated.as_bytes(), s()).is_err());
    }

    #[test]
    fn accepts_a_prompt_that_is_not_utf8() {
        let mut p = vec![0xff, 0xfe, 0x00];
        p.extend_from_slice(s().as_os_str().as_encoded_bytes());
        p.push(0x80);
        assert!(recognise_prompt(&p, s()).is_ok());
    }

    #[test]
    fn recognition_does_not_depend_on_the_rest_of_the_prompt() {
        // An invite-shaped token in the prompt changes nothing: the recogniser
        // sees one needle and ignores every other byte. TEST-012 leans on this.
        let p = format!(
            "invite: ELEPHANT-INVITE-abcdef0123456789\nsentinel: {}\n",
            s().display()
        );
        assert!(recognise_prompt(p.as_bytes(), s()).is_ok());
    }
}
