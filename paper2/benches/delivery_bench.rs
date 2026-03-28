//! Criterion benchmarks for BTV-Transparency.
//!
//! Bench 1: DeliveryToken::seal overhead (in-process, no I/O)
//! Bench 2: Full pipeline latency (requires btv-log on localhost:8080)
//!
//! Run standalone benches (no server needed):
//!   cargo bench --manifest-path paper2/Cargo.toml -- seal
//!
//! Run integration bench (start btv-log first):
//!   cargo run --bin btv-log &
//!   cargo bench --manifest-path paper2/Cargo.toml -- submission

use btv_transparency::{
    log_server::{LogState, SharedLog},
    DeliveryToken, LogClient,
};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::Serialize;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Minimal test verdict (mirrors Paper 1 structure)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct MockVerdict {
    decision: &'static str,
    evidence_id: [u8; 32],
}

fn mock_verdict() -> MockVerdict {
    MockVerdict {
        decision: "Deny",
        evidence_id: [0xABu8; 32],
    }
}

// ---------------------------------------------------------------------------
// Bench 1: In-process seal (no network)
// ---------------------------------------------------------------------------

fn bench_seal_in_process(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let log: SharedLog = Arc::new(Mutex::new(LogState::new(signing_key)));

    c.bench_function("DeliveryToken::seal (in-process)", |b| {
        b.iter_batched(
            || {
                // Setup: submit a hash directly to the in-process log
                let hash = [0x42u8; 32];
                let (index, _root, sig_bytes) = log.lock().unwrap().submit(hash);
                use ed25519_dalek::Signature;
                let sig = Signature::from_bytes(&sig_bytes);
                btv_transparency::InclusionReceipt::new_for_bench(index, sig, verifying_key)
            },
            |receipt| {
                let verdict = mock_verdict();
                std::hint::black_box(
                    DeliveryToken::seal(&verdict, receipt)
                        .expect("seal must succeed with valid receipt")
                        .deliver()
                )
            },
            BatchSize::SmallInput,
        )
    });
}

// ---------------------------------------------------------------------------
// Bench 2: Full pipeline (requires btv-log on localhost:8080)
// ---------------------------------------------------------------------------

fn bench_full_pipeline(c: &mut Criterion) {
    // Skip if btv-log is not running
    if ureq::get("http://localhost:8080/health").call().is_err() {
        eprintln!("[bench_full_pipeline] btv-log not running on :8080 — skipping");
        return;
    }

    let vk_hex = std::env::var("BTV_LOG_VERIFYING_KEY")
        .expect("Set BTV_LOG_VERIFYING_KEY to the hex verifying key printed by btv-log");
    let vk_bytes: Vec<u8> = (0..vk_hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&vk_hex[i..i + 2], 16).unwrap())
        .collect();
    let vk_arr: [u8; 32] = vk_bytes.try_into().unwrap();
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&vk_arr).unwrap();

    let client = LogClient::new("http://localhost:8080", verifying_key);

    c.bench_function("Full pipeline: Verdict → Log → DeliveryToken", |b| {
        b.iter_batched(
            || mock_verdict(),
            |verdict| {
                let hash = [0x42u8; 32]; // simplified: use verdict hash in production
                let receipt = client
                    .submit_and_await(&hash)
                    .expect("btv-log must be running");
                let token = DeliveryToken::seal(&verdict, receipt)
                    .expect("seal must succeed");
                std::hint::black_box(token.deliver())
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_seal_in_process, bench_full_pipeline);
criterion_main!(benches);
