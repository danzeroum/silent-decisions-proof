// compile-fail test: Protection C
// Silently dropping an InclusionReceipt is a hard error (#[must_use]).
// Expected: unused_must_use promoted to error

use btv_transparency::LogClient;

fn main() {
    let client = LogClient::new("http://localhost:8080");
    let hash = [0u8; 32];
    // Intentionally ignoring the Result<InclusionReceipt, _>
    // This must NOT compile (or must trigger must_use error).
    client.submit_and_await(&hash);
}
