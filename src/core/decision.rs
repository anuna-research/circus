//! The acceptance predicate of `SPEC-001-circus-agent-harness#REQ-004.c`.
//!
//! One function, total, with no access to anything but its arguments. That is
//! deliberate: the gate is the control the whole specification turns on, and a
//! control that cannot be evaluated in isolation cannot be tested in
//! isolation.

use super::grammar::EvidenceRef;
use super::record::Decision;

/// Decide acceptance from a verifier exit code and the evidence references the
/// caller supplied.
///
/// Accepted only when **both** conditions hold: the verifier reported 0, and
/// at least one evidence reference is present. Note what is absent — no
/// transport exit code is an input here, which is
/// `SPEC-001-circus-agent-harness#REQ-004.b` enforced by the function's
/// signature rather than by its body.
///
/// The references are not inspected. `SPEC-001-circus-agent-harness#REQ-004.e`
/// forbids resolving them, so only their count can matter.
pub fn acceptance_decision(verifier_exit_code: i32, evidence: &[EvidenceRef]) -> Decision {
    if verifier_exit_code == 0 && !evidence.is_empty() {
        Decision::Accepted
    } else {
        Decision::Rejected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(n: usize) -> Vec<EvidenceRef> {
        (0..n)
            .map(|i| EvidenceRef::recognise(&format!("theory:q{i}")).unwrap())
            .collect()
    }

    #[test]
    fn accepts_only_on_zero_exit_and_at_least_one_reference() {
        assert_eq!(acceptance_decision(0, &refs(1)), Decision::Accepted);
        assert_eq!(acceptance_decision(0, &refs(5)), Decision::Accepted);
    }

    #[test]
    fn rejects_a_failed_verifier_however_much_evidence_is_supplied() {
        // TEST-007
        for code in [1, 2, 127, 255] {
            assert_eq!(acceptance_decision(code, &refs(3)), Decision::Rejected);
        }
    }

    #[test]
    fn rejects_a_passing_verifier_with_no_evidence() {
        // TEST-021 — the branch a suite of refusals alone would miss.
        assert_eq!(acceptance_decision(0, &refs(0)), Decision::Rejected);
    }

    #[test]
    fn the_two_conditions_are_conjunctive() {
        // Exhaustive over the 2x2 truth table, so no mutation of `&&` to `||`
        // survives.
        assert_eq!(acceptance_decision(0, &refs(1)), Decision::Accepted);
        assert_eq!(acceptance_decision(0, &refs(0)), Decision::Rejected);
        assert_eq!(acceptance_decision(1, &refs(1)), Decision::Rejected);
        assert_eq!(acceptance_decision(1, &refs(0)), Decision::Rejected);
    }
}
