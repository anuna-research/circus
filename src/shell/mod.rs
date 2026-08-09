//! The effectful shell: Git, tmux, withdone, the filesystem, and the advisory
//! lock.
//!
//! Everything here performs effects and calls inward to [`crate::core`]. The
//! reverse import is forbidden by the Purity Boundary Map of
//! `SPEC-001-circus-agent-harness`, and `tests/purity.rs` enforces it.

pub mod git;
pub mod pane;
pub mod proc;
pub mod state;
pub mod status;
pub mod ui;

use std::fmt;

use crate::core::RecognitionError;
use crate::exit;

/// Anything that can go wrong once effects are involved.
///
/// Each variant carries the exit code the specification assigns it, so the
/// error model of a contract is decided in one place rather than at each
/// return site.
#[derive(Debug)]
pub enum Error {
    /// Input failed a declared grammar, or a pre-condition did not hold.
    Usage(String),
    /// A recognition failure, with the production named.
    Recognition(RecognitionError),
    /// A run record on disk failed schema recognition.
    DataErr(String),
    /// An external program failed.
    Software(String),
    /// A required external program is not on `PATH`.
    NotFound(String),
    /// A recorded negative outcome. The command worked; the answer was "no".
    Rejected(String),
    /// An unexpected I/O failure.
    Io(std::io::Error),
}

impl Error {
    pub fn usage(m: impl Into<String>) -> Self {
        Self::Usage(m.into())
    }
    pub fn software(m: impl Into<String>) -> Self {
        Self::Software(m.into())
    }
    pub fn rejected(m: impl Into<String>) -> Self {
        Self::Rejected(m.into())
    }

    /// What to run next, where a generic answer exists.
    ///
    /// Most messages already carry their own guidance inline, because advice
    /// specific to one failure belongs next to it. This covers the two cases
    /// where the remedy is the same every time.
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::NotFound(p) => Some(format!("install `{p}`, then run the same command again")),
            Self::DataErr(_) => {
                Some("inspect the record, or prepare a new attempt with `circus prepare`".into())
            }
            _ => None,
        }
    }

    /// The process exit code for this error.
    pub fn code(&self) -> i32 {
        match self {
            Self::Usage(_) | Self::Recognition(_) => exit::USAGE,
            Self::DataErr(_) => exit::DATAERR,
            Self::Software(_) | Self::Io(_) => exit::SOFTWARE,
            Self::NotFound(_) => exit::NOTFOUND,
            Self::Rejected(_) => exit::REJECTED,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(m) | Self::DataErr(m) | Self::Software(m) | Self::Rejected(m) => {
                f.write_str(m)
            }
            Self::NotFound(p) => write!(
                f,
                "required external program `{p}` was not found on PATH; \
                 Circus composes it rather than reimplementing it"
            ),
            Self::Recognition(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<RecognitionError> for Error {
    fn from(e: RecognitionError) -> Self {
        Self::Recognition(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
