//! Human-facing output. Everything here goes to stderr.
//!
//! `SPEC-001-circus-agent-harness#REQ-008` splits the two streams: stdout
//! carries [`crate::core::record`] and nothing else, so a caller parsing it
//! never has to filter progress out. Every message a person reads goes here
//! instead.
//!
//! Styling is governed by [`ADR-008`](SPEC-001-circus-agent-harness#ADR-008):
//! stderr is styled only when it is a terminal, `--no-color` was not given,
//! `NO_COLOR` is unset or empty, and `TERM` is not `dumb`.

use std::io::{IsTerminal, Write};
use std::time::Duration;

/// How much to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Errors only — `--quiet`.
    Quiet,
    /// Progress, state changes, and hints.
    Normal,
    /// Everything, plus each external program invoked — `--verbose`.
    Verbose,
}

/// The stderr channel.
#[derive(Debug, Clone, Copy)]
pub struct Ui {
    level: Level,
    style: bool,
    /// Whether stderr is a terminal, which decides in-place progress.
    terminal: bool,
}

const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";
/// Return to column zero and clear the line.
const CLEAR: &str = "\r\x1b[2K";

impl Ui {
    pub fn new(level: Level, no_color: bool) -> Self {
        let terminal = std::io::stderr().is_terminal();
        Self {
            level,
            style: terminal && !no_color && !color_suppressed_by_environment(),
            terminal,
        }
    }

    /// A `Ui` that says nothing, for tests and for the error path before flags
    /// have been parsed.
    pub fn silent() -> Self {
        Self {
            level: Level::Quiet,
            style: false,
            terminal: false,
        }
    }

    pub fn is_verbose(self) -> bool {
        self.level == Level::Verbose
    }

    /// Whether stderr is a terminal. Decides in-place progress versus a new
    /// line per report — `ADR-008`.
    pub fn is_terminal(self) -> bool {
        self.terminal
    }

    fn speaks(self) -> bool {
        self.level != Level::Quiet
    }

    /// A state change worth confirming: "prepared model/1".
    pub fn state(self, msg: &str) {
        if self.speaks() {
            self.line(&format!("circus: {msg}"));
        }
    }

    /// A suggestion for what to run next. Dimmed, because it is optional
    /// reading once the operator knows the loop.
    pub fn hint(self, msg: &str) {
        if self.speaks() {
            if self.style {
                self.line(&format!("{DIM}  {msg}{RESET}"));
            } else {
                self.line(&format!("  {msg}"));
            }
        }
    }

    /// Something worth emphasising, such as the attach command.
    pub fn emphasis(self, msg: &str) {
        if self.speaks() {
            if self.style {
                self.line(&format!("  {BOLD}{msg}{RESET}"));
            } else {
                self.line(&format!("  {msg}"));
            }
        }
    }

    /// One external program invocation — `--verbose` only.
    pub fn invocation(self, program: &str, args: &[String]) {
        if !self.is_verbose() {
            return;
        }
        let rendered = args.join(" ");
        if self.style {
            self.line(&format!("{DIM}circus: + {program} {rendered}{RESET}"));
        } else {
            self.line(&format!("circus: + {program} {rendered}"));
        }
    }

    /// An error. Never suppressed, because `--quiet` silences messages and not
    /// failures — `#REQ-008.e`.
    pub fn error(self, msg: &str) {
        let text = if self.style {
            format!("{RED}circus:{RESET} {msg}")
        } else {
            format!("circus: {msg}")
        };
        self.line(&text);
    }

    /// Overwrite the progress line on a terminal, or emit a new line
    /// elsewhere. `#REQ-008.d`, and the reason `ADR-008` mentions the smear a
    /// carriage return leaves in a log file.
    pub fn progress(self, msg: &str) {
        if !self.speaks() {
            return;
        }
        let mut err = std::io::stderr().lock();
        if self.terminal {
            let _ = write!(err, "{CLEAR}circus: {msg}");
        } else {
            let _ = writeln!(err, "circus: {msg}");
        }
        let _ = err.flush();
    }

    /// Clear a terminal progress line before printing something durable.
    pub fn end_progress(self) {
        if self.speaks() && self.terminal {
            let mut err = std::io::stderr().lock();
            let _ = write!(err, "{CLEAR}");
            let _ = err.flush();
        }
    }

    fn line(self, text: &str) {
        let mut err = std::io::stderr().lock();
        let _ = writeln!(err, "{text}");
    }
}

fn color_suppressed_by_environment() -> bool {
    // https://no-color.org — any non-empty value disables colour.
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return true;
    }
    std::env::var_os("TERM").is_some_and(|t| t == "dumb")
}

/// Render a duration the way a person reads it: `4s`, `2m 05s`, `1h 03m`.
pub fn human_duration(d: Duration) -> String {
    let s = d.as_secs();
    match s {
        0..=59 => format!("{s}s"),
        60..=3599 => format!("{}m {:02}s", s / 60, s % 60),
        _ => format!("{}h {:02}m", s / 3600, (s % 3600) / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_silent_ui_says_nothing_and_is_not_verbose() {
        let ui = Ui::silent();
        assert!(!ui.speaks());
        assert!(!ui.is_verbose());
        assert!(!ui.style);
    }

    #[test]
    fn quiet_suppresses_messages_but_not_errors() {
        // The behavioural claim of REQ-008.e, at the level the type can carry.
        // TEST-037 checks the observable half.
        let quiet = Ui {
            level: Level::Quiet,
            style: false,
            terminal: false,
        };
        assert!(
            !quiet.speaks(),
            "quiet must suppress state, hints, and progress"
        );
        // `error` has no `speaks()` guard, which is the point.
    }

    #[test]
    fn verbose_implies_speaking() {
        let v = Ui {
            level: Level::Verbose,
            style: false,
            terminal: false,
        };
        assert!(v.speaks());
        assert!(v.is_verbose());
        let n = Ui {
            level: Level::Normal,
            style: false,
            terminal: false,
        };
        assert!(n.speaks());
        assert!(!n.is_verbose(), "normal must not print invocations");
    }

    #[test]
    fn styling_is_off_when_stderr_is_not_a_terminal() {
        // ADR-008. Under `cargo test` stderr is captured, so this is the
        // real code path rather than a simulated one.
        let ui = Ui::new(Level::Normal, false);
        assert!(!ui.style, "captured stderr must never be styled");
    }

    #[test]
    fn no_color_flag_wins_over_everything() {
        assert!(!Ui::new(Level::Normal, true).style);
    }

    #[test]
    fn durations_read_the_way_a_person_says_them() {
        let cases = [
            (0u64, "0s"),
            (1, "1s"),
            (59, "59s"),
            (60, "1m 00s"),
            (65, "1m 05s"),
            (3599, "59m 59s"),
            (3600, "1h 00m"),
            (7_380, "2h 03m"),
        ];
        for (secs, want) in cases {
            assert_eq!(
                human_duration(Duration::from_secs(secs)),
                want,
                "for {secs}"
            );
        }
    }
}
