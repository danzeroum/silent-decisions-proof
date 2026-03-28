// Corollary 4.8 — Case B: OperatorToken is linear (cannot be reused).
//
// After being consumed by EscalatedVerdict::new(), the token is moved.
// Expected: error[E0382]: use of moved value: `token`
use silent_decisions_proof::{
    ContextRef, Decision, EscalatedVerdict, OperatorAuthority,
};

fn main() {
    let authority = OperatorAuthority::new([0xBB; 32]);
    let token = authority.issue_token([0x01; 32]);
    let ctx = ContextRef::from_context(b"failed-context");

    // First use — valid
    let _v1 = EscalatedVerdict::new(
        token,
        Decision::Allow,
        ctx,
        "System timeout — operator approved".to_string(),
    );

    // Second use — MUST fail with E0382
    let _v2 = EscalatedVerdict::new(
        token,
        Decision::Deny,
        ctx,
        "Attempted reuse".to_string(),
    );
}
