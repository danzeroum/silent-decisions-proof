// Clause 4: Blake3Hash has no public arbitrary constructor.
//
// The inner field of Blake3Hash is private. This file must fail to compile with E0603.
use silent_decisions_proof::Blake3Hash;

fn main() {
    // Attempting to construct a Blake3Hash by directly accessing the inner field.
    // Expected: error[E0603] tuple struct field `0` of `Blake3Hash` is private
    let _hash = Blake3Hash([0u8; 32]);
}
