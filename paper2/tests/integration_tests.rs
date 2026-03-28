// Integration tests for btv-transparency.
//
// end_to_end_pipeline: verifies the full HTTP path
//   axum btv-log server → LogClient::submit_and_await → DeliveryToken::seal → deliver
//
// log_unavailable_fails_secure: verifies Corollary IV-B (Fail-Secure Persistence)
//   LogClient pointing to a dead port → submit_and_await returns Err
//   → no DeliveryToken can be produced

use btv_transparency::{DeliveryToken, LogClient, LogClientError};
use btv_transparency::log_server::{build_router, LogState};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Serialize)]
struct TestVerdict {
    id: u32,
    outcome: &'static str,
}

/// Empirical proof of the Persistence Enclosure Theorem (Theorem III):
/// a DeliveryToken can only be obtained after a genuine log submission.
/// Spins up a real btv-log HTTP server in a background tokio thread.
/// Uses spawn_blocking to avoid deadlocking the tokio runtime with ureq's
/// synchronous HTTP client.
#[test]
fn end_to_end_pipeline() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();

    let log = Arc::new(Mutex::new(LogState::new(sk)));
    let app = build_router(log);

    // Create the runtime and bind the listener inside it
    let rt = tokio::runtime::Runtime::new().unwrap();
    let listener = rt
        .block_on(async { tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap() });
    let addr = listener.local_addr().unwrap();

    // Serve in the background within this runtime
    rt.spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // ureq is synchronous/blocking — run it in a separate OS thread so it
    // doesn't block the tokio executor threads (which are needed by the server).
    let verdict = TestVerdict { id: 42, outcome: "Allow" };
    let verdict_bytes = serde_json::to_vec(&verdict).unwrap();
    let verdict_hash: [u8; 32] = {
        use sha2::{Digest, Sha256};
        Sha256::digest(&verdict_bytes).into()
    };

    let receipt = std::thread::spawn(move || {
        let client = LogClient::new(format!("http://{addr}"), vk);
        client.submit_and_await(&verdict_hash)
    })
    .join()
    .unwrap()
    .expect("submit to local btv-log must succeed");

    let verdict = TestVerdict { id: 42, outcome: "Allow" }; // re-create (moved)
    let token = DeliveryToken::seal(&verdict, receipt)
        .expect("seal must succeed with a valid receipt");
    let payload = token.deliver();

    assert_eq!(payload.log_index, 0);
    assert!(!payload.verdict_json.is_empty());
    assert_eq!(payload.signature_hex.len(), 128); // 64 bytes × 2 hex chars
}

/// Empirical proof of Corollary IV-B (Fail-Secure Persistence):
/// if the log is unreachable, submit_and_await returns Err,
/// and the type system prevents constructing a DeliveryToken from that Err.
#[test]
fn log_unavailable_fails_secure() {

    // Bind to port 0 to get a free OS port, then drop the listener immediately.
    // The port will be unreachable (connection refused) since no server binds to it.
    let free_addr = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap()
    }; // listener dropped here — port is now closed

    let vk = SigningKey::generate(&mut OsRng).verifying_key();
    let client = LogClient::new(format!("http://{free_addr}"), vk);

    let hash = [0u8; 32];
    let result = client.submit_and_await(&hash);

    // Must return an error — no InclusionReceipt, therefore no DeliveryToken.
    assert!(
        matches!(result, Err(LogClientError::Network(_))),
        "expected Network error when log is unavailable",
    );

    // The type system statically ensures: because result is Err,
    // there is no InclusionReceipt in scope, so DeliveryToken::seal
    // cannot be called. This is enforced at compile time (not just here).
}
