// compile-fail test: Protection B
// An external crate cannot call InclusionReceipt::consume.
// Expected error: E0624 (method is private)

use btv_transparency::InclusionReceipt;

fn steal(r: InclusionReceipt) {
    // This must NOT compile.
    let _ = r.consume();
}

fn main() {}
