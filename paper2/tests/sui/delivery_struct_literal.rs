// compile-fail test: Case A — Struct-Literal Forgery
// An external crate cannot construct DeliveryToken via struct literal.
// All fields of DeliveryToken are private.
// Expected error: E0451 (field is private)

use btv_transparency::DeliveryToken;

fn main() {
    // This must NOT compile.
    let _t = DeliveryToken {
        verdict_bytes: vec![],
        log_index: 0,
        signature_bytes: vec![],
    };
}
