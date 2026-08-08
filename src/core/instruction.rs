//! The completion instruction — `SPEC-001-circus-agent-harness#REQ-010`.
//!
//! This is the one piece of prompt text Circus knows, and it is deliberately
//! the only one. It is not provider-specific: any agent that can run a shell
//! command can follow it, whichever CLI hosts the agent. What makes it worth
//! centralising is that the clause which matters most — write the sentinel
//! when the work is *done*, not when you plan to finish — is the clause every
//! hand-written version leaves out.
//!
//! Circus prints it. Circus never inserts it into a prompt; that would be the
//! prompt injection `#REQ-003.d` and `#REQ-007.b` forbid. Composition does the
//! joining:
//!
//! ```sh
//! { cat task.md; circus instruction --attempt model/1; } > prompt.md
//! ```
//!
//! Source: adapted from the withdone recipes, where the mechanism was first
//! written down.

use std::path::Path;

/// Build the completion instruction for one attempt.
///
/// Pure: the sentinel path is the only input, and the output is a function of
/// it alone.
pub fn instruction_text(sentinel: &Path) -> String {
    // A leading blank line, so `{ cat task.md; circus instruction; }` reads as
    // two paragraphs rather than one run-on.
    format!(
        "
When you have completed the task, signal completion by running exactly this
shell command — no other output:

  echo <CODE> > {path}

<CODE> is a single non-negative integer:

  0   the task is complete
  1   you could not complete it
  2+  any other failure you want to distinguish

Do not run this command until the task is genuinely done. Writing it ends the
session immediately, and anything you had not finished is lost. Treat the path
as opaque; it is your runtime's completion-signal target.
",
        path = sentinel.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::prompt::recognise_prompt;

    fn sentinel() -> &'static Path {
        Path::new("/repo/.git/circus/model/1/sentinel")
    }

    #[test]
    fn the_instruction_carries_the_sentinel_path_verbatim() {
        // REQ-010.b
        let text = instruction_text(sentinel());
        assert!(
            text.contains("/repo/.git/circus/model/1/sentinel"),
            "path missing: {text}"
        );
    }

    #[test]
    fn a_prompt_made_of_the_instruction_is_recognised() {
        // The property that makes REQ-010 useful rather than decorative: what
        // Circus prints is what its own recogniser accepts.
        let text = instruction_text(sentinel());
        assert!(recognise_prompt(text.as_bytes(), sentinel()).is_ok());
    }

    #[test]
    fn a_task_prepended_to_the_instruction_is_still_recognised() {
        // The documented composition: `{ cat task.md; circus instruction; }`.
        let joined = format!("Fix the failing test.\n\n{}", instruction_text(sentinel()));
        assert!(recognise_prompt(joined.as_bytes(), sentinel()).is_ok());
    }

    #[test]
    fn the_instruction_for_one_attempt_does_not_satisfy_another() {
        let other = Path::new("/repo/.git/circus/model/2/sentinel");
        let text = instruction_text(sentinel());
        assert!(
            recognise_prompt(text.as_bytes(), other).is_err(),
            "an instruction must not launch the wrong attempt"
        );
    }

    #[test]
    fn the_instruction_states_when_not_to_write() {
        // The clause every hand-written version omits, and the reason this
        // function exists at all. Losing it silently would leave agents
        // signalling completion the moment they form a plan.
        let text = instruction_text(sentinel());
        assert!(
            text.contains("Do not run this command until the task is genuinely done"),
            "the timing clause is missing: {text}"
        );
    }

    #[test]
    fn the_instruction_explains_the_codes() {
        let text = instruction_text(sentinel());
        for expected in ["<CODE>", "0 ", "1 ", "2+"] {
            assert!(text.contains(expected), "missing {expected:?}: {text}");
        }
    }

    #[test]
    fn it_separates_itself_from_whatever_precedes_it() {
        // The documented composition concatenates two files; without this the
        // instruction runs on from the last line of the task.
        assert!(instruction_text(sentinel()).starts_with('\n'));
    }

    #[test]
    fn it_is_deterministic_and_ends_with_a_newline() {
        assert_eq!(instruction_text(sentinel()), instruction_text(sentinel()));
        assert!(instruction_text(sentinel()).ends_with('\n'));
    }

    #[test]
    fn a_path_with_spaces_survives_intact() {
        let odd = Path::new("/a dir/with spaces/sentinel");
        let text = instruction_text(odd);
        assert!(text.contains("/a dir/with spaces/sentinel"));
        assert!(recognise_prompt(text.as_bytes(), odd).is_ok());
    }
}
