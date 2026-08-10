# cbcl-rs

The Rust implementation of [[concepts/cbcl|CBCL]]: a linear-time parser and the
R1–R5 validation pipeline, with the core algorithms machine-checked in Lean 4.
Its pure crates are `no_std + alloc` and forbid unsafe code.

It is the **one recogniser** for CBCL across the composition. Every other program
links it rather than reading S-expressions itself, which is what removes
parser-differential bugs by construction.
[[SPEC-002-leaderless-dispatch-loop#REQ-005]] forbids a dispatch loop from
adding a second reader.
