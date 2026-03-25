// Corollary 4.8 — Case C: OperatorToken cannot be silently dropped.
//
// #[must_use] + #![deny(unused_must_use)] promotes the warning to error.
// Expected: error: unused `OperatorToken` that must be used
#![deny(unused_must_use)]
use silent_decisions_proof::OperatorAuthority;

fn main() {
    let authority = OperatorAuthority::new([0xBB; 32]);
    authority.issue_token([0x01; 32]);
    // ↑ OperatorToken created and immediately dropped — must_use violation
}
