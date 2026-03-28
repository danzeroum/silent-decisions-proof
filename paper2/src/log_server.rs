//! btv-log — lightweight in-memory Transparency Log server.
//!
//! Implements a minimal CT-style interface:
//!   POST /submit   → { index, root_hash, signature }
//!   GET  /proof/:i → Merkle inclusion proof
//!
//! This is a REFERENCE IMPLEMENTATION for benchmarking and testing.
//! Production deployments should substitute Trillian + a durable backend.

use ed25519_dalek::{SigningKey, Signature, Signer};
use sha2::{Sha256, Digest};

/// Single node of the append-only Merkle tree.
#[derive(Clone)]
pub struct LogEntry {
    pub hash: [u8; 32],
}

/// In-memory Transparency Log.
pub struct TransparencyLog {
    entries: Vec<LogEntry>,
    signing_key: SigningKey,
}

impl TransparencyLog {
    /// Create a new log with a freshly generated Ed25519 signing key.
    pub fn new(signing_key: SigningKey) -> Self {
        Self {
            entries: Vec::new(),
            signing_key,
        }
    }

    /// Append a verdict hash and return (index, root_hash, signature).
    ///
    /// The signature covers `index || root_hash` using the log's private key.
    pub fn submit(&mut self, verdict_hash: [u8; 32]) -> (u64, [u8; 32], Signature) {
        self.entries.push(LogEntry { hash: verdict_hash });
        let index = (self.entries.len() - 1) as u64;
        let root = self.compute_root();
        let mut msg = Vec::with_capacity(8 + 32);
        msg.extend_from_slice(&index.to_le_bytes());
        msg.extend_from_slice(&root);
        let signature = self.signing_key.sign(&msg);
        (index, root, signature)
    }

    /// Compute the Merkle root over all entries (sequential SHA-256 hash chain).
    ///
    /// A production implementation would use a balanced binary Merkle tree;
    /// this sequential hash suffices for the reference implementation.
    fn compute_root(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for entry in &self.entries {
            hasher.update(entry.hash);
        }
        hasher.finalize().into()
    }

    /// Return the number of entries (for inclusion proof verification).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
