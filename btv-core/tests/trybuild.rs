// Test 1 (compile-fail): runs all 8 UI tests via trybuild.
//
// This file itself must compile and pass — trybuild verifies that each
// .rs file in tests/ui/ FAILS to compile with the expected .stderr.
//
// NOTE: Two additional runtime checks (F: forged-hash via PyO3, G: drop
// external via PyO3) live in tests/pyo3/test_binding.py because they
// require the PyO3 binding to be built. They cannot be compile-fail tests
// at the Rust level since PyO3 errors surface at Python runtime.

#[test]
fn compile_fail_suite() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/verdict_struct_literal.rs");
    t.compile_fail("tests/ui/blake3hash_public_constructor.rs");
    t.compile_fail("tests/ui/external_consume_call.rs");
    t.compile_fail("tests/ui/dropped_evidence_token.rs");
    t.compile_fail("tests/ui/escalated_struct_literal.rs");
    t.compile_fail("tests/ui/escalated_token_reuse.rs");
    t.compile_fail("tests/ui/escalated_consume_external.rs");
    t.compile_fail("tests/ui/escalated_operator_token_drop.rs");
}
