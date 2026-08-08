//! Circus — a small local harness that runs coding-agent CLIs in isolated Git
//! worktrees.
//!
//! Specified by `SPEC-001-circus-agent-harness`. The module split is the
//! Purity Boundary Map of that specification:
//!
//! - [`core`] is deterministic and side-effect free. It recognises input,
//!   derives paths, codes the run record, and decides acceptance.
//! - [`shell`] performs every effect: Git, tmux, withdone, the filesystem, and
//!   the advisory lock.
//!
//! Dependencies point inward. `core` MUST NOT import from `shell`; the
//! `dependency_rule` test in `tests/purity.rs` enforces it.

pub mod core;
pub mod shell;

/// Process exit codes. The values follow `sysexits.h` where one applies, so a
/// caller can distinguish a usage error from a data error from a tool failure.
pub mod exit {
    /// Success.
    pub const OK: i32 = 0;
    /// A recorded negative outcome: verifier rejected, or merge conflicted.
    /// Not a Circus fault — the command did its job and the answer was "no".
    pub const REJECTED: i32 = 1;
    /// Input failed recognition against a declared grammar, or a
    /// pre-condition did not hold.
    pub const USAGE: i32 = 64;
    /// A run record on disk failed schema recognition.
    pub const DATAERR: i32 = 65;
    /// An external program Circus depends on failed.
    pub const SOFTWARE: i32 = 70;
    /// A required external program is absent from `PATH`.
    pub const NOTFOUND: i32 = 127;
}
