//! Test 6 — Concorrência e carga: p50/p95/p99 sob contenção multi-thread.
//!
//! Usa `rayon` para disparar `issue_verdict` de múltiplas threads
//! concorrentemente contra um `InMemoryLogSink` compartilhado (via `Arc`).
//!
//! OS-08 (COMSI-2026-04-0112, closes F9): este teste NÃO escreve mais em
//! `reports/` — essa era exatamente a falha F9 (o teste sobrescrevia a
//! evidência commitada com os números da máquina local a cada `cargo test`,
//! e a heurística `p50 > 100us` rotulava incorretamente uma máquina nativa
//! carregada como "ARM64 emulado"). Este teste agora só mede e afirma limites
//! amplos; a evidência commitada é gerada deliberadamente por
//! `examples/load_report.rs` (mesmo padrão de `examples/rss_probe_fail_secure.rs`).
//!
//! Epistemic footer:
//!   Este teste valida a latência do BTV sob contenção multi-thread em
//!   hardware single-node. Ele NÃO substitui benchmarks em ambiente de
//!   produção com locust/wrk contra um servidor HTTP real, pois não
//!   modela custo de rede, serialização JSON, scheduling do SO sob
//!   carga mista, nem efeitos de GC do Python.

#![cfg(test)]

use btv_core::{issue_verdict, ComplianceAuthority, Decision, EvidenceToken, InMemoryLogSink};
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;

/// Thread count for the load test: explicit default, overridable via
/// `BTV_LOAD_THREADS` (OS-08 — no more silent `available_parallelism()`,
/// which produced a different, non-reproducible number on every machine).
fn load_threads() -> usize {
    std::env::var("BTV_LOAD_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4)
}

#[test]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::redundant_closure_for_method_calls,
    clippy::uninlined_format_args
)]
fn concurrent_load_in_memory_p50_p95_p99() {
    let n_threads = load_threads();
    let ops_per_thread = 1_000;
    let total_ops = n_threads * ops_per_thread;

    let sink = Arc::new(InMemoryLogSink::new());
    let authority = Arc::new(ComplianceAuthority::new_for_test());

    // Wall-clock throughput (OS-08, closes F8's throughput half): total_ops /
    // wall_clock across the whole concurrent batch. Summing per-op latencies
    // and dividing into total_ops (the old approach) is not throughput under
    // concurrency — it is closer to the reciprocal of mean single-op latency,
    // which undercounts real throughput by roughly a factor of n_threads.
    let wall_start = Instant::now();
    let latencies_ns: Vec<u64> = (0..n_threads)
        .into_par_iter()
        .flat_map(|thread_id| {
            let sink = Arc::clone(&sink);
            let auth = Arc::clone(&authority);
            let mut local_latencies: Vec<u64> = Vec::with_capacity(ops_per_thread);
            for i in 0..ops_per_thread {
                let ctx = format!("thread-{thread_id}-op-{i}");
                let token = EvidenceToken::new(ctx.as_bytes());
                let compliance = auth.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
                let t0 = Instant::now();
                let verdict = issue_verdict(
                    token,
                    compliance,
                    Decision::Allow,
                    "load test".to_string(),
                    &*sink,
                )
                .expect("issue_verdict should succeed under load");
                let elapsed = t0.elapsed();
                assert!(verdict.verify_integrity());
                local_latencies.push(elapsed.as_nanos() as u64);
            }
            local_latencies
        })
        .collect();
    let wall_elapsed = wall_start.elapsed();

    assert_eq!(latencies_ns.len(), total_ops);
    assert_eq!(sink.len(), total_ops);

    let mut sorted_us: Vec<f64> = latencies_ns.iter().map(|&ns| ns as f64 / 1000.0).collect();
    sorted_us.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p = |q: f64| -> f64 {
        let idx = ((sorted_us.len() as f64 - 1.0) * q).round() as usize;
        sorted_us[idx.min(sorted_us.len() - 1)]
    };
    let mean: f64 = sorted_us.iter().sum::<f64>() / sorted_us.len() as f64;
    let throughput = total_ops as f64 / wall_elapsed.as_secs_f64();

    println!(
        "[load_test] threads={n_threads} ops={total_ops} wall={:.3}ms \
         p50={:.2}us p95={:.2}us p99={:.2}us mean={:.2}us throughput={:.0}ops/s",
        wall_elapsed.as_secs_f64() * 1000.0,
        p(0.50),
        p(0.95),
        p(0.99),
        mean,
        throughput
    );

    // Soft assertions (wide margins — never tight bounds; CI runs on shared,
    // unpredictable hardware and under QEMU emulation on the ARM64 job).
    assert!(
        p(0.99) < 5000.0,
        "p99 must be under 5ms in-memory; got {:.2}us",
        p(0.99)
    );
    assert!(
        throughput > 100.0,
        "throughput must exceed 100 ops/s even under heavy contention or emulation; got {throughput:.0}"
    );
}
