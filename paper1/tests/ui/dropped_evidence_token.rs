// Clause 6: Dropping an EvidenceToken without consuming it is a compile error.
//
// EvidenceToken is #[must_use]. With #[deny(unused_must_use)], failing to use
// the return value of EvidenceToken::new() is an error, not merely a warning.
#![deny(unused_must_use)]

use silent_decisions_proof::EvidenceToken;

fn main() {
    // Dropped without calling .consume() — violates #[must_use].
    // Expected: error: unused return value of `EvidenceToken` that must be used
    EvidenceToken::new(b"decision-context-that-is-silently-discarded");
}
