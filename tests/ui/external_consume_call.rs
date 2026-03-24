// Case B: External invocation of EvidenceToken::consume() is blocked.
//
// consume() is pub(crate). This file must fail to compile because external
// code cannot call a pub(crate) method.
use silent_decisions_proof::EvidenceToken;

fn main() {
    let token = EvidenceToken::new(b"test-context");
    let _hash = token.consume(); // expected compile error: method is private
}
