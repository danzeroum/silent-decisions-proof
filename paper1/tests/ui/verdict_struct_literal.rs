// Clause 3: Verdict struct literal syntax is blocked outside the defining module.
//
// All fields of Verdict are private. This file must fail to compile with E0451.
use silent_decisions_proof::{Decision, Verdict};

fn main() {
    // Attempting to construct a Verdict via struct literal syntax.
    // Expected: error[E0451] field `evidence_id` of struct `Verdict` is private
    let _verdict = Verdict {
        decision: Decision::Allow,
    };
}
