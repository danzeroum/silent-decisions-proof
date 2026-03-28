// compile-fail test: Protection C
// Silently dropping the Result<InclusionReceipt, _> from submit_and_await
// is a hard error because Result is #[must_use]. The deny attribute below
// promotes the warning to an error (mirroring btv_transparency's crate-level
// #![deny(unused_must_use)]).
// Expected: error: unused `Result` which must be used

#![deny(unused_must_use)]

use btv_transparency::LogClient;
use ed25519_dalek::VerifyingKey;

fn main() {
    // Use a fixed 32-byte key (not valid cryptographically, but the test
    // never runs — it must fail to compile before execution).
    let key_bytes = [2u8; 32];
    let vk = VerifyingKey::from_bytes(&key_bytes).unwrap();
    let client = LogClient::new("http://localhost:8080", vk);
    let hash = [0u8; 32];
    // Intentionally ignoring the Result<InclusionReceipt, _>.
    // This must NOT compile (unused_must_use promoted to error).
    client.submit_and_await(&hash);
}
