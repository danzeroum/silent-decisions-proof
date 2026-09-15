// Test 1 (compile-fail): the 8 UI fixtures that correspond to the paper's
// numbered proof clauses moved into individually-named `clause_N_*` tests
// (clause_3, clause_4, clause_6, clause_7, clause_10, clause_11, clause_12,
// clause_13) split across `src/lib.rs`'s `mod tests` and
// `tests/test_proof_clauses.rs` (OS-09, COMSI-2026-04-0112, closes H4) —
// `grep -c "fn clause_"` across `btv-core` now gives a single number
// (17) matching the manuscript's clause count, instead of three divergent
// counts (paper text vs. paper1 vs. btv-core). This file keeps only the
// fixture that is NOT one of the 17 paper clauses: an OS-06 regression
// check added by this audit round.
//
// NOTE: Two additional runtime checks (F: forged-hash via PyO3, G: drop
// external via PyO3) live in tests/pyo3/test_binding.py because they
// require the PyO3 binding to be built. They cannot be compile-fail tests
// at the Rust level since PyO3 errors surface at Python runtime.

#[test]
fn compile_fail_suite() {
    let t = trybuild::TestCases::new();
    // OS-06 gate: tokens moved into a failed `issue_verdict` cannot be
    // reused by the caller — retry is a compile error (E0382), which is
    // the linearity guarantee `mem::forget` never provided.
    t.compile_fail("tests/ui/token_reuse_after_fail_secure.rs");
}
