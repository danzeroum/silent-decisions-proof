//! btv-log — lightweight in-memory Transparency Log server (Axum HTTP).
//!
//! Endpoints:
//!   POST /submit   { verdict_hash_hex: ".." } → { index, root_hash_hex, signature_hex }
//!   GET  /proof/:i → { index, hash_hex }       (stub — full Merkle path in future work)
//!   GET  /health   → 200 OK
//!
//! This is a REFERENCE IMPLEMENTATION for benchmarking.
//! Production deployments should substitute Trillian + a durable backend.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use ed25519_dalek::{SigningKey, Signer, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Log state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct LogEntry {
    pub hash: [u8; 32],
}

pub struct LogState {
    pub entries: Vec<LogEntry>,
    pub signing_key: SigningKey,
}

impl LogState {
    pub fn new(signing_key: SigningKey) -> Self {
        Self {
            entries: Vec::new(),
            signing_key,
        }
    }

    /// Append a verdict hash; return (index, root_hash, Ed25519 signature).
    pub fn submit(&mut self, verdict_hash: [u8; 32]) -> (u64, [u8; 32], [u8; 64]) {
        self.entries.push(LogEntry { hash: verdict_hash });
        let index = (self.entries.len() - 1) as u64;
        let root = self.compute_root();
        // Sign: index (8 bytes LE) || root_hash (32 bytes)
        let mut msg = Vec::with_capacity(40);
        msg.extend_from_slice(&index.to_le_bytes());
        msg.extend_from_slice(&root);
        let signature = self.signing_key.sign(&msg);
        (index, root, signature.to_bytes())
    }

    fn compute_root(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for entry in &self.entries {
            hasher.update(entry.hash);
        }
        hasher.finalize().into()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
}

pub type SharedLog = Arc<Mutex<LogState>>;

// ---------------------------------------------------------------------------
// Axum handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SubmitRequest {
    pub verdict_hash_hex: String,
}

#[derive(Serialize)]
pub struct SubmitResponse {
    pub index: u64,
    pub root_hash_hex: String,
    pub signature_hex: String,
}

async fn handle_submit(
    State(log): State<SharedLog>,
    Json(req): Json<SubmitRequest>,
) -> impl IntoResponse {
    let hash_bytes = match decode_hex_32(&req.verdict_hash_hex) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid verdict_hash_hex (expected 64 hex chars)"}))
            ).into_response();
        }
    };

    let (index, root, sig) = log.lock().unwrap().submit(hash_bytes);

    let resp = SubmitResponse {
        index,
        root_hash_hex: encode_hex(&root),
        signature_hex: encode_hex(&sig),
    };
    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Serialize)]
struct ProofResponse {
    index: u64,
    hash_hex: String,
}

async fn handle_proof(
    State(log): State<SharedLog>,
    Path(index): Path<u64>,
) -> impl IntoResponse {
    let guard = log.lock().unwrap();
    match guard.entries.get(index as usize) {
        Some(entry) => {
            let resp = ProofResponse {
                index,
                hash_hex: encode_hex(&entry.hash),
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "index not found"}))
        ).into_response(),
    }
}

async fn handle_health() -> impl IntoResponse {
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// Router factory (used in main and in integration tests)
// ---------------------------------------------------------------------------

pub fn build_router(log: SharedLog) -> Router {
    Router::new()
        .route("/submit", post(handle_submit))
        .route("/proof/:index", get(handle_proof))
        .route("/health", get(handle_health))
        .with_state(log)
}

// ---------------------------------------------------------------------------
// Hex helpers
// ---------------------------------------------------------------------------

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex_32(s: &str) -> Result<[u8; 32], ()> {
    if s.len() != 64 {
        return Err(());
    }
    let bytes: Result<Vec<u8>, _> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect();
    let bytes = bytes.map_err(|_| ())?;
    bytes.try_into().map_err(|_| ())
}
