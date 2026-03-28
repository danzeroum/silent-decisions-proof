// Corollary 4.8 — Case A: EscalatedVerdict struct literal is blocked.
//
// All fields of EscalatedVerdict are private.
// Expected: error containing "private"
use silent_decisions_proof::{Decision, EscalatedVerdict};

fn main() {
    let _v = EscalatedVerdict {
        decision: Decision::Allow,
    };
}
