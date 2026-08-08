//! POSIX shell quoting.
//!
//! Circus generates two tiny shell scripts per attempt (see
//! [`crate::shell::pane`]). Constitutional Principle 14 forbids
//! string-concatenation serialisation of a structured format at a trust
//! boundary, and shell source is a structured format. Driver argv never
//! reaches these scripts as text — it arrives as `"$@"` — but the attempt's
//! own paths do, and a repository checked out under a directory with a space
//! or an apostrophe in its name would otherwise produce a broken or, worse, a
//! differently-behaving script.
//!
//! The single-quote form is used because it is total: inside `'…'` every byte
//! except `'` is literal, and `'` itself is expressed by closing the quote,
//! emitting an escaped quote, and reopening. There is no character this
//! cannot express and no shell metacharacter that survives it.

/// Quote a string so a POSIX shell reads it as one literal word.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            // Close, emit a literal quote, reopen.
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Quote a path for the same purpose.
pub fn quote_path(p: &std::path::Path) -> String {
    quote(&p.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Ask a real shell what it made of the quoting. A quoter tested only
    /// against its own expectations proves nothing.
    fn shell_roundtrip(s: &str) -> String {
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("printf %s {}", quote(s)))
            .output()
            .expect("sh runs");
        assert!(out.status.success(), "shell rejected the quoting of {s:?}");
        String::from_utf8(out.stdout).expect("printf echoes the input bytes")
    }

    #[test]
    fn quotes_ordinary_words() {
        assert_eq!(quote("abc"), "'abc'");
        assert_eq!(shell_roundtrip("abc"), "abc");
    }

    #[test]
    fn survives_every_metacharacter_a_shell_knows() {
        let hostile = [
            "a b",
            "a\tb",
            "a\nb",
            "$HOME",
            "`id`",
            "$(id)",
            "a;id",
            "a|id",
            "a&id",
            "a>b",
            "a<b",
            "*",
            "?",
            "[a-z]",
            "~root",
            "a\\b",
            "\"quoted\"",
            "it's",
            "''",
            "'; id; '",
            "#comment",
            "!history",
            "a{b,c}",
            "\u{1f409}",
        ];
        for s in hostile {
            assert_eq!(shell_roundtrip(s), s, "quoting failed for {s:?}");
        }
    }

    #[test]
    fn an_apostrophe_is_expressed_rather_than_escaped_inline() {
        assert_eq!(quote("it's"), r"'it'\''s'");
        assert_eq!(shell_roundtrip("it's"), "it's");
    }

    #[test]
    fn the_empty_string_stays_one_word() {
        assert_eq!(quote(""), "''");
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("set -- {}; echo $#", quote("")))
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "1");
    }

    #[test]
    fn a_quoted_path_with_spaces_remains_one_argument_with_its_content() {
        let p = std::path::Path::new("/a dir/with 'quotes'/file");
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!(
                "set -- {}; echo $#; printf %s \"$1\"",
                quote_path(p)
            ))
            .output()
            .unwrap();
        let got = String::from_utf8_lossy(&out.stdout).into_owned();
        let (count, arg) = got.split_once('\n').expect("two lines");
        assert_eq!(count, "1", "path did not stay one word");
        assert_eq!(arg, p.to_string_lossy(), "path content changed");
    }
}
