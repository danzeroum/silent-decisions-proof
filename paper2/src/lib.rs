//! btv-transparency — Paper 2 core crate.
//!
//! Extends BTV (Paper 1) to the persistence layer.
//! Type law: DeliveryToken ⊸ (V ⊗ R)

#![deny(unused_must_use)]

pub mod log_server;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// InclusionReceipt — linear resource, mirrors EvidenceToken from Paper 1
// ---------------------------------------------------------------------------

/// Ed25519-signed proof that a Verdict hash was accepted by the Transparency Log.
///
/// Linear by construction:
/// - No `Clone` / `Copy` (contraction prohibited)
/// - `#[must_use]` prevents silent drops (weakening prohibited)
/// - All fields private (Protection A: E0451)
/// - Constructor `pub(crate)` (Protection B: E0624)
/// - `consume()` is `pub(crate)` (closes forged-hash vector)
#[must_use = "InclusionReceipt must be consumed by DeliveryToken::seal"]
pub struct InclusionReceipt {
    log_index: u64,
    signed_tree_head: Signature,
    verifying_key: VerifyingKey,
}

impl InclusionReceipt {
    /// Internal constructor — called only by `LogClient` inside this crate.
    pub(crate) fn new(index: u64, sth: Signature, key: VerifyingKey) -> Self {
        Self {
            log_index: index,
            signed_tree_head: sth,
            verifying_key: key,
        }
    }

    /// Bench-only constructor exposed for in-process benchmarks.
    /// NOT part of the public API used in proofs — benches are `#[cfg(bench)]`.
    #[cfg(any(test, feature = "bench_helpers"))]
    pub fn new_for_bench(index: u64, sth: Signature, key: VerifyingKey) -> Self {
        Self::new(index, sth, key)
    }

    /// Verify the Ed25519 signature before consuming.
    pub fn verify(&self, message: &[u8]) -> Result<(), ed25519_dalek::SignatureError> {
        self.verifying_key.verify(message, &self.signed_tree_head)
    }

    /// Linear destructor — `pub(crate)` closes the E0624 attack vector.
    pub(crate) fn consume(self) -> (u64, Signature, VerifyingKey) {
        (self.log_index, self.signed_tree_head, self.verifying_key)
    }

    /// Read-only accessor for auditing without consuming.
    pub fn log_index(&self) -> u64 {
        self.log_index
    }
}

// ---------------------------------------------------------------------------
// DeliveryToken — the sole type that authorises crossing the API boundary
// ---------------------------------------------------------------------------

#[must_use = "DeliveryToken must be consumed by deliver()"]
pub struct DeliveryToken {
    verdict_bytes: Vec<u8>,
    log_index: u64,
    signature_bytes: Vec<u8>,
}

impl DeliveryToken {
    /// Atomically seals a Verdict and its InclusionReceipt.
    /// Verifies the Ed25519 signature before consuming the receipt.
    pub fn seal(
        verdict: &impl Serialize,
        receipt: InclusionReceipt,
    ) -> Result<Self, SealError> {
        let verdict_bytes =
            serde_json::to_vec(verdict).map_err(SealError::Serialization)?;
        let msg = index_message(receipt.log_index);
        receipt.verify(&msg).map_err(|_| SealError::InvalidSignature)?;
        let (log_index, sig, _key) = receipt.consume();
        Ok(Self {
            verdict_bytes,
            log_index,
            signature_bytes: sig.to_bytes().to_vec(),
        })
    }

    #[must_use = "delivery not confirmed — handle the returned payload"]
    pub fn deliver(self) -> DeliveryPayload {
        DeliveryPayload {
            verdict_json: self.verdict_bytes,
            log_index: self.log_index,
            signature_hex: encode_hex(&self.signature_bytes),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DeliveryPayload {
    pub verdict_json: Vec<u8>,
    pub log_index: u64,
    pub signature_hex: String,
}

#[derive(Debug)]
pub enum SealError {
    Serialization(serde_json::Error),
    InvalidSignature,
}

impl std::fmt::Display for SealError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialization(e) => write!(f, "Verdict serialisation error: {e}"),
            Self::InvalidSignature => write!(f, "InclusionReceipt signature verification failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// LogClient — synchronous blocking HTTP client
// ---------------------------------------------------------------------------

pub struct LogClient {
    endpoint: String,
    verifying_key: VerifyingKey,
}

#[derive(Deserialize)]
pub struct SubmitResponse {
    pub index: u64,
    pub signature_hex: String,
}

impl LogClient {
    /// Create a client pinned to a specific log public key.
    /// The verifying key must be obtained out-of-band, not from the server.
    pub fn new(endpoint: impl Into<String>, verifying_key: VerifyingKey) -> Self {
        Self {
            endpoint: endpoint.into(),
            verifying_key,
        }
    }

    /// Submit the 32-byte verdict hash and block until the SCT arrives.
    pub fn submit_and_await(
        &self,
        verdict_hash: &[u8; 32],
    ) -> Result<InclusionReceipt, LogClientError> {
        let url = format!("{}/submit", self.endpoint);
        let hash_hex = encode_hex(verdict_hash);
        let body = ureq::json!({ "verdict_hash_hex": hash_hex });

        let resp: SubmitResponse = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| LogClientError::Network(e.to_string()))?
            .into_json()
            .map_err(|e| LogClientError::Network(format!("invalid JSON: {e}")))?
        ;

        let sig_bytes = decode_hex(&resp.signature_hex)
            .map_err(|_| LogClientError::InvalidSignature)?;
        let sig_arr: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| LogClientError::InvalidSignature)?;
        let signature = Signature::from_bytes(&sig_arr);

        // Verify before constructing the receipt
        let msg = index_message(resp.index);
        self.verifying_key
            .verify(&msg, &signature)
            .map_err(|_| LogClientError::InvalidSignature)?;

        Ok(InclusionReceipt::new(resp.index, signature, self.verifying_key))
    }
}

/// Error type returned by [`LogClient::submit_and_await`].
#[derive(Debug)]
pub enum LogClientError {
    Network(String),
    InvalidSignature,
}

impl std::fmt::Display for LogClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Log network error: {msg}"),
            Self::InvalidSignature => write!(f, "Log returned invalid Ed25519 signature"),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn index_message(index: u64) -> Vec<u8> {
    index.to_le_bytes().to_vec()
}

pub fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestVerdict {
        id: u32,
        outcome: &'static str,
    }

    fn make_receipt(index: u64, signing_key: &SigningKey) -> InclusionReceipt {
        use ed25519_dalek::Signer;
        let msg = index_message(index);
        let sig = signing_key.sign(&msg);
        InclusionReceipt::new(index, sig, signing_key.verifying_key())
    }

    #[test]
    fn seal_and_deliver_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let receipt = make_receipt(0, &sk);
        let verdict = TestVerdict { id: 1, outcome: "Deny" };
        let token = DeliveryToken::seal(&verdict, receipt).unwrap();
        let payload = token.deliver();
        assert_eq!(payload.log_index, 0);
        assert!(!payload.verdict_json.is_empty());
        assert_eq!(payload.signature_hex.len(), 128); // 64 bytes × 2 hex chars
    }

    #[test]
    fn invalid_signature_rejected() {
        let sk = SigningKey::generate(&mut OsRng);
        let other_sk = SigningKey::generate(&mut OsRng);
        // Receipt signed by other_sk, but verifying_key is sk.verifying_key()
        use ed25519_dalek::Signer;
        let msg = index_message(0);
        let sig = other_sk.sign(&msg);
        let receipt = InclusionReceipt::new(0, sig, sk.verifying_key());
        let verdict = TestVerdict { id: 2, outcome: "Allow" };
        assert!(matches!(
            DeliveryToken::seal(&verdict, receipt),
            Err(SealError::InvalidSignature)
        ));
    }
}
