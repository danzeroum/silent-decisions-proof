// compile-fail test: InclusionReceipt does not implement Clone or Copy.
// Attempting to clone it is a hard error.
// Expected: E0599 — no method named `clone` found for struct `InclusionReceipt`

use btv_transparency::InclusionReceipt;

fn attempt_clone(r: InclusionReceipt) {
    // InclusionReceipt is not Clone — this must NOT compile.
    let _r2 = r.clone();
}

fn main() {}
