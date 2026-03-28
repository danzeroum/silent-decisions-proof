// Corollary 4.8 — Case D: External code cannot call OperatorToken::consume().
//
// consume() is pub(crate). External crates cannot invoke it.
// Expected: error[E0624]: method `consume` is private
use silent_decisions_proof::OperatorAuthority;

fn main() {
    let authority = OperatorAuthority::new([0xBB; 32]);
    let token = authority.issue_token([0x01; 32]);
    let _consumed = token.consume(); // Should fail: method is private
}
