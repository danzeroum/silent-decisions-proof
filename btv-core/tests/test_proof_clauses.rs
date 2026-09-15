//! Canonical proof-clause suite (OS-09, closes H4/H6).
//!
//! History: the repository carried TWO divergent implementations
//! (`paper1/src/lib.rs`, 985 lines, 17 clauses; `btv-core/src/lib.rs`,
//! 1,057 lines, 5 clauses) and the manuscript said "seven clauses" while
//! the submitted PDF said "fifteen". OS-09 elects `btv-core` as the single
//! canonical implementation; `paper1/src/lib.rs` is a re-export of it.
//! The clause count is now ONE number, fixed by this suite: the grep count
//! of clause-function definitions across `btv-core` equals SEVENTEEN, and
//! Sections 1 and 7 of the manuscript cite exactly that number (gate).
//!
//! Clauses 1, 2, 5, 16, 17 live in `src/lib.rs` (`#[cfg(test)]`); this
//! file carries the remaining twelve, migrated from `paper1` verbatim in
//! substance (adapted to the signed-token API of OS-02).

use btv_core::{
    AccountableDecision, ComplianceAuthority, ContextRef, Decision, EscalatedVerdict,
    EvidenceToken, OperatorAuthority, Verdict,
};

// ── trybuild-backed clauses (compile-fail proofs; files in tests/ui/) ────

/// Clause 3: a `Verdict` struct literal is blocked outside the crate (E0451).
#[test]
fn clause_3_verdict_struct_literal_is_blocked() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/verdict_struct_literal.rs");
}

/// Clause 4: `Blake3Hash` has no public arbitrary constructor (E0423).
#[test]
fn clause_4_blake3hash_has_no_public_constructor() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/blake3hash_public_constructor.rs");
}

/// Clause 6: dropping an `EvidenceToken` without consuming it is a hard
/// error (`#[must_use]` + `#![deny(unused_must_use)]`).
#[test]
fn clause_6_dropped_token_produces_compiler_warning() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/dropped_evidence_token.rs");
}

/// Clause 7: external `consume()` is blocked (`pub(crate)` visibility, E0624).
#[test]
fn clause_7_external_consume_is_blocked() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/external_consume_call.rs");
}

/// Clause 10: an `EscalatedVerdict` struct literal is blocked (E0451).
#[test]
fn clause_10_escalated_struct_literal_is_blocked() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/escalated_struct_literal.rs");
}

/// Clause 11: `OperatorToken` reuse is blocked (E0382).
#[test]
fn clause_11_operator_token_reuse_is_blocked() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/escalated_token_reuse.rs");
}

/// Clause 12: dropping an `OperatorToken` without consuming it is blocked.
#[test]
fn clause_12_dropped_operator_token_is_blocked() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/escalated_operator_token_drop.rs");
}

/// Clause 13: external `OperatorToken::consume()` is blocked (E0624).
#[test]
fn clause_13_external_consume_is_blocked() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/escalated_consume_external.rs");
}

// ── runtime clauses ──────────────────────────────────────────────────────

/// Clause 8: an `EscalatedVerdict` can be constructed through the sole
/// linear path (`OperatorToken` consumed by value) and verifies.
#[test]
fn clause_8_escalated_verdict_can_be_constructed() {
    let authority = OperatorAuthority::new_for_test();
    let token = authority.issue_token([0x42; 32]);
    let ctx = ContextRef::from_context(b"medical-triage-context-timeout");
    let verdict = EscalatedVerdict::new(
        token,
        Decision::Allow,
        ctx,
        "System timeout during triage — nurse approved immediate treatment".to_string(),
    );
    assert!(
        verdict.verify_integrity(),
        "Freshly constructed EscalatedVerdict must pass integrity check"
    );
    assert_eq!(verdict.operator_id(), &[0x42; 32]);
    assert_eq!(
        verdict.reason(),
        "System timeout during triage — nurse approved immediate treatment"
    );
}

// Clause 9 lives in `src/lib.rs`'s `#[cfg(test)]` module: it exercises
// `OperatorToken::consume()`, which is `pub(crate)` BY DESIGN (an external
// call is itself a compile error — Clause 13).

/// Clause 14: a tampered `EscalatedVerdict` fails integrity.
#[test]
fn clause_14_tampered_escalated_verdict_fails_integrity() {
    let authority = OperatorAuthority::new_for_test();
    let token = authority.issue_token([0x42; 32]);
    let ctx = ContextRef::from_context(b"triage-context");
    let mut verdict = EscalatedVerdict::new(
        token,
        Decision::Allow,
        ctx,
        "Legitimate escalation reason".to_string(),
    );
    assert!(verdict.verify_integrity(), "Pre-tamper: must pass");
    verdict.tamper_reason_for_test("Maliciously altered reason");
    assert!(!verdict.verify_integrity(), "Post-tamper: must FAIL");
}

/// Clause 15: `AccountableDecision` is polymorphic over both verdict types.
#[test]
fn clause_15_accountable_decision_trait_is_polymorphic() {
    let token = EvidenceToken::new(b"auto-context");
    let authority = ComplianceAuthority::new_for_test();
    let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
    let auto_verdict = Verdict::new(token, compliance, Decision::Deny, "Auto denial".to_string())
        .expect("new_for_test authority holds the recognized key");

    let op_authority = OperatorAuthority::new_for_test();
    let op_token = op_authority.issue_token([0x42; 32]);
    let ctx = ContextRef::from_context(b"failed-context");
    let esc_verdict =
        EscalatedVerdict::new(op_token, Decision::Allow, ctx, "Human override".to_string());

    assert!(check_integrity(&auto_verdict));
    assert!(check_integrity(&esc_verdict));
    assert!(auto_verdict.is_automated());
    assert!(!esc_verdict.is_automated());
}

/// Clause 15 helper: dispatch through the trait object.
fn check_integrity(d: &dyn AccountableDecision) -> bool {
    d.verify_integrity()
}
