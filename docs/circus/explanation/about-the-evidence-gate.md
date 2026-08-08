---
title: About the evidence gate
mode: explanation
spec: SPEC-001
---

# About the evidence gate

Why `circus accept` needs two things from you, and why it deliberately checks
one of them less than a reader expects.

## The problem with an exit code

An agent CLI that exits 0 has told you one thing: its process ended without an
error it recognised. That is a fact about transport. It is not a claim that the
tests pass, that the change is correct, or that the work was even attempted.

Every harness that treats exit 0 as "done" inherits the agent's own judgement
about its own work. Circus records the exit code as `transport_exit_code` and
refuses to let it reach `state` or `decision`. The acceptance function does not
take the transport code as an argument at all, so no future edit can quietly
start using it.

## What the gate actually checks

Two conditions, together:

- The verifier record reports exit code 0.
- At least one evidence reference is present.

Neither alone is enough. A passing verifier with no evidence exits 64. A
failing verifier with abundant evidence records `rejected`.

## The part Circus does not check

An evidence reference is an opaque string. Circus records it verbatim and never
resolves it. `circus accept --evidence hunter2` passes the presence check.

This is a deliberate limit, and stating it plainly matters more than hiding it.
Circus is forbidden from touching Elephant: it creates no identity, parses no
invite, and never invokes the binary. A harness that resolves references
needs an identity of its own, which turns a deliberately out-of-band trust
decision into an implicit local privilege.

So the guarantee Circus offers is narrower than "the evidence is real". It is:

> A verifier ran, it passed, someone attested at least one reference, and all
> of it is recorded durably before anything merges.

Establishing that the reference means what it says is the lead's work, done
with `elephant explain` and `elephant describe --json` before acceptance. The
gate makes that work *recorded*; it does not do that work.

## Why a narrow guarantee beats a fake one

The alternative designs are worse in specific ways.

One design has Circus resolve references itself. That re-creates the supervisor the
Elephant design removed, and it needs the identity ADR-003 refuses.

Another skips the presence check and trusts the verifier alone. That drops
the link between a change and the reasoning that justified it, which is the
only durable trace of *why* a merge happened.

A third accepts on the exit code and offers an override. Every harness that has
tried this discovers the override is the default within a week.

A gate that admits its limits can be reasoned about. One that overstates them
gets trusted for something it never did.

## Where this leaves the operator

Read the transcript. Run your own verifier. Query the theory. Then hand Circus
the two artefacts, and it will make the decision permanent and inspectable.

Decisions: ADR-002. Requirements: REQ-004. Signals: OBS-002.
