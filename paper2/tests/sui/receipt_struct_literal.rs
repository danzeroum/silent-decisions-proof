// compile-fail test: Protection A
// An external crate cannot construct InclusionReceipt via struct literal.
// Expected error: E0451 (field is private)

use btv_transparency::InclusionReceipt;

fn main() {
    // This must NOT compile.
    let _r = InclusionReceipt {
        log_index: 0,
        signed_tree_head: todo!(),
        verifying_key: todo!(),
    };
}
