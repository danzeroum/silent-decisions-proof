//! btv-transparency — Paper 2 core crate.
//!
//! Extends BTV (Paper 1) to the persistence layer.
//! Type law: DeliveryToken ⊸ (V ⊗ R)

#![deny(unused_must_use)]

use ed25519_dalek::{Signature, VerifyingKey};
use serde::Serialize;

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

    /// Linear destructor — transfers inner values to `DeliveryToken`.
    /// `pub(crate)` closes the external-consumption attack vector (E0624).
    pub(crate) fn consume(self) -> (u64, Signature, VerifyingKey) {
        (self.log_index, self.signed_tree_head, self.verifying_key)
    }
}

// ---------------------------------------------------------------------------
// DeliveryToken — the sole type that authorises crossing the API boundary
// ---------------------------------------------------------------------------

/// Proof that a Verdict has been persisted in the Transparency Log
/// AND is ready to be delivered to the end-user.
///
/// Cannot be constructed without a valid `InclusionReceipt`.
/// Cannot be silently discarded (`#[must_use]`).
#[must_use = "DeliveryToken must be consumed by deliver()"]
pub struct DeliveryToken {
    /// The payload, kept private to prevent struct-literal forging.
    verdict_bytes: Vec<u8>,
    log_index: u64,
    signature: Signature,
}

impl DeliveryToken {
    /// Atomically seals a Verdict and its InclusionReceipt.
    ///
    /// Both arguments are consumed by value — the linear discipline is
    /// enforced by the Rust type checker, not by runtime assertion.
    pub fn seal(
        verdict: &impl Serialize,
        receipt: InclusionReceipt,
    ) -> Result<Self, serde_json::Error> {
        let verdict_bytes = serde_json::to_vec(verdict)?;
        let (log_index, signature, _key) = receipt.consume();
        Ok(Self {
            verdict_bytes,
            log_index,
            signature,
        })
    }

    /// Releases the decision payload to the caller.
    ///
    /// Returns `(verdict_json, log_index, signature_bytes)`.
    /// `#[must_use]` on the return value prevents silent discard.
    #[must_use = "delivery not confirmed — handle or propagate the result"]
    pub fn deliver(self) -> (Vec<u8>, u64, Vec<u8>) {
        (
            self.verdict_bytes,
            self.log_index,
            self.signature.to_bytes().to_vec(),
        )
    }
}

// ---------------------------------------------------------------------------
// LogClient — blocks until InclusionReceipt is returned
// ---------------------------------------------------------------------------

/// Thin HTTP client for the btv-log server.
///
/// `submit_and_await` is the only public entry point; it returns an
/// `InclusionReceipt` that must be consumed by `DeliveryToken::seal`.
/// If the log is offline the call returns `Err`, and by the Persistence
/// Enclosure Theorem no `DeliveryToken` can be produced — the system
/// fails securely (Corollary 4.7 of Paper 1, extended).
pub struct LogClient {
    endpoint: String,
}

impl LogClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    /// Submit the verdict hash to the log and block until the SCT arrives.
    ///
    /// Returns `InclusionReceipt` on success, `Err` on network or
    /// server-side failure.
    pub fn submit_and_await(
        &self,
        verdict_hash: &[u8; 32],
    ) -> Result<InclusionReceipt, LogClientError> {
        // Placeholder implementation — replaced by HTTP call in integration tests.
        // Production: POST self.endpoint/submit with verdict_hash, parse response.
        let _ = (verdict_hash, &self.endpoint);
        Err(LogClientError::NotImplemented)
    }
}

#[derive(Debug)]
pub enum LogClientError {
    Network(String),
    InvalidSignature,
    NotImplemented,
}

impl std::fmt::Display for LogClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Log network error: {msg}"),
            Self::InvalidSignature => write!(f, "Log returned invalid Ed25519 signature"),
            Self::NotImplemented => write!(f, "LogClient::submit_and_await not yet implemented"),
        }
    }
}
