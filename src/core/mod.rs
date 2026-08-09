//! The pure core: deterministic, side-effect free, no I/O.
//!
//! Every recogniser here is total. It returns a typed value or a
//! [`RecognitionError`], and it never partially consumes an input before
//! rejecting it. Downstream code consumes the typed value, never the raw
//! bytes — LangSec principle 7, "separate recognition from interpretation".

pub mod ansi;
pub mod decision;
pub mod grammar;
pub mod instruction;
pub mod marker;
pub mod paths;
pub mod prompt;
pub mod record;
pub mod shquote;
pub mod time;
pub mod verifier;

/// Why an input was refused. Carries the production it failed so an error
/// message can name the grammar rather than guess at intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecognitionError {
    /// The grammar production the input failed, e.g. `"task"`.
    pub production: &'static str,
    /// What was wrong, in one clause.
    pub detail: String,
}

impl RecognitionError {
    pub fn new(production: &'static str, detail: impl Into<String>) -> Self {
        Self {
            production,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for RecognitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "input failed the `{}` production: {}",
            self.production, self.detail
        )
    }
}

impl std::error::Error for RecognitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_names_the_production_it_failed() {
        // The message is what an operator reads at 64. Naming the grammar is
        // the difference between "bad input" and a pointer at the rule.
        let e = RecognitionError::new("task", "first character must be lowercase");
        let rendered = e.to_string();
        assert!(rendered.contains("task"), "{rendered}");
        assert!(
            rendered.contains("first character must be lowercase"),
            "{rendered}"
        );
    }
}
