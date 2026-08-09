//! Stripping terminal control sequences from a transcript —
//! `SPEC-001-circus-agent-harness#REQ-012.c`.
//!
//! The transcript is a capture of a terminal, so it carries the colour, cursor
//! movement, and redraws the agent emitted. That is faithful, and for a
//! full-screen agent it is unreadable. `--plain` removes the control bytes and
//! keeps the text.
//!
//! This recognises the escape forms a terminal capture actually contains — CSI,
//! OSC, and the two-byte escapes — rather than attempting a general parser for
//! everything ECMA-48 permits. An unrecognised escape loses its introducer and
//! nothing else, which degrades to noise rather than to silence.

/// Remove ANSI control sequences and carriage returns.
///
/// Operates on bytes: a transcript is whatever the agent wrote, and is not
/// required to be UTF-8.
pub fn strip(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        match input[i] {
            // A carriage return without a newline is a redraw in place. Left
            // in, it makes a log file overwrite itself when displayed.
            b'\r' => {
                i += 1;
                if input.get(i) == Some(&b'\n') {
                    out.push(b'\n');
                    i += 1;
                }
            }
            0x1b => i += escape_len(&input[i..]),
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    out
}

/// How many bytes the escape sequence at the start of `s` occupies.
///
/// Always at least 1, so a caller cannot fail to make progress.
fn escape_len(s: &[u8]) -> usize {
    match s.get(1) {
        // CSI: ESC [ params intermediates final. The final byte is @ to ~.
        Some(b'[') => {
            let mut i = 2;
            while i < s.len() && !(0x40..=0x7e).contains(&s[i]) {
                i += 1;
            }
            (i + 1).min(s.len().max(1))
        }
        // OSC: ESC ] ... terminated by BEL or ST (ESC \).
        Some(b']') => {
            let mut i = 2;
            while i < s.len() {
                if s[i] == 0x07 {
                    return i + 1;
                }
                if s[i] == 0x1b && s.get(i + 1) == Some(&b'\\') {
                    return i + 2;
                }
                i += 1;
            }
            s.len()
        }
        // ESC intermediate… final, per ECMA-48: intermediates are 0x20..=0x2F
        // and the final byte is 0x30..=0x7E. `ESC ( B` is the common one, and
        // treating it as two bytes leaves a stray `B` in the text.
        Some(b) if (0x20..=0x2f).contains(b) => {
            let mut i = 1;
            while i < s.len() && (0x20..=0x2f).contains(&s[i]) {
                i += 1;
            }
            (i + 1).min(s.len())
        }
        // Two-byte escapes: ESC =, ESC 7, ESC M, and so on.
        Some(_) => 2,
        // A trailing ESC with nothing after it.
        None => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(b: &[u8]) -> String {
        String::from_utf8_lossy(b).into_owned()
    }

    #[test]
    fn plain_text_is_untouched() {
        assert_eq!(s(&strip(b"hello world\n")), "hello world\n");
        assert_eq!(s(&strip(b"")), "");
    }

    #[test]
    fn removes_colour() {
        assert_eq!(s(&strip(b"\x1b[31mred\x1b[0m\n")), "red\n");
        assert_eq!(s(&strip(b"\x1b[1;32;40mbold\x1b[m")), "bold");
    }

    #[test]
    fn removes_cursor_movement_and_clears() {
        assert_eq!(s(&strip(b"\x1b[2K\x1b[1;1Hhere")), "here");
        assert_eq!(s(&strip(b"a\x1b[3Db")), "ab");
    }

    #[test]
    fn removes_an_osc_terminated_either_way() {
        assert_eq!(s(&strip(b"\x1b]0;a title\x07text")), "text");
        assert_eq!(s(&strip(b"\x1b]8;;https://x\x1b\\link")), "link");
    }

    #[test]
    fn removes_two_byte_escapes() {
        assert_eq!(s(&strip(b"\x1b(B\x1b=body\x1b>")), "body");
    }

    #[test]
    fn collapses_a_bare_carriage_return_and_keeps_crlf() {
        assert_eq!(s(&strip(b"progress\rdone\n")), "progressdone\n");
        assert_eq!(s(&strip(b"line\r\n")), "line\n");
    }

    #[test]
    fn a_truncated_escape_does_not_run_off_the_end() {
        // A capture cut mid-sequence must not panic or loop.
        for bad in [
            &b"\x1b"[..],
            &b"\x1b["[..],
            &b"\x1b[31"[..],
            &b"\x1b]0;unterminated"[..],
            &b"\x1b]8;;x\x1b"[..],
        ] {
            let _ = strip(bad);
        }
    }

    #[test]
    fn every_input_terminates_and_never_grows() {
        // strip removes bytes and adds none, and escape_len always advances.
        let cases: Vec<Vec<u8>> = vec![
            b"\x1b\x1b\x1b".to_vec(),
            b"\x1b[".repeat(50),
            (0u8..=255).collect(),
            b"\r\r\r\n".to_vec(),
        ];
        for c in cases {
            let out = strip(&c);
            assert!(out.len() <= c.len(), "output grew for {c:?}");
        }
    }

    #[test]
    fn non_utf8_bytes_survive() {
        let input = b"\xff\xfe ok \x1b[0m\x80";
        let out = strip(input);
        assert!(out.contains(&0xff) && out.contains(&0x80), "{out:?}");
        assert!(!out.contains(&0x1b));
    }

    #[test]
    fn a_real_capture_reduces_to_its_text() {
        let capture = b"\x1b[?1049h\x1b[2J\x1b[H\x1b[32magent\x1b[0m working\r\n\x1b[?1049l";
        assert_eq!(s(&strip(capture)), "agent working\n");
    }
}
