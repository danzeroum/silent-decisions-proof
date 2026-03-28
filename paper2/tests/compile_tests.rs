// Compile-fail test driver.
// Invokes trybuild to verify that all attack cases in the Persistence
// Enclosure Theorem (Section 4) are rejected by the Rust compiler.
#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    // Protections on InclusionReceipt (mirrors Cases A-C from Paper 1):
    t.compile_fail("tests/ui/receipt_struct_literal.rs");
    t.compile_fail("tests/ui/receipt_consume_external.rs");
    t.compile_fail("tests/ui/receipt_drop_silent.rs");
    t.compile_fail("tests/ui/receipt_token_reuse.rs");
    // Protections on DeliveryToken (Case A) and Persistence Enclosure (Case B):
    t.compile_fail("tests/ui/delivery_struct_literal.rs");
    t.compile_fail("tests/ui/delivery_without_receipt.rs");
    t.compile_fail("tests/ui/delivery_token_reuse.rs");
}
