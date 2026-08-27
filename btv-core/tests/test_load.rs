//! Test 6 — Concorrência e carga: p50/p95/p99 sob contenção multi-thread.
//!
//! Usa `rayon` para disparar `issue_verdict` de múltiplas threads
//! concorrentemente contra um `InMemoryLogSink` compartilhado (via `Arc`).
//!
//! Métricas reportadas em `reports/load_stats.csv`:
//! - p50, p95, p99 (em μs)
//! - throughput agregado (ops/s)
//! - variância (stdev)
//!
//! Epistemic footer:
//!   Este teste valida a latência do BTV sob contenção multi-thread em
//!   hardware single-node. Ele NÃO substitui benchmarks em ambiente de
//!   produção com locust/wrk contra um servidor HTTP real, pois não
//!   modela custo de rede, serialização JSON, scheduling do SO sob
//!   carga mista, nem efeitos de GC do Python.

#![cfg(test)]

use btv_core::{
    issue_verdict, ComplianceAuthority, Decision, EvidenceToken, InMemoryLogSink,
};
use rayon::prelude::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn concurrent_load_in_memory_p50_p95_p99() {
    let n_threads = num_cpus();
    let ops_per_thread = 1_000;
    let total_ops = n_threads * ops_per_thread;

    let sink = Arc::new(InMemoryLogSink::new());
    let authority = Arc::new(ComplianceAuthority::new_for_test());

    let latencies: Vec<Duration> = (0..n_threads)
        .into_par_iter()
        .flat_map(|thread_id| {
            let sink = Arc::clone(&sink);
            let auth = Arc::clone(&authority);
            let mut local_latencies: Vec<Duration> = Vec::with_capacity(ops_per_thread);
            for i in 0..ops_per_thread {
                let ctx = format!("thread-{thread_id}-op-{i}");
                let token = EvidenceToken::new(ctx.as_bytes());
                let compliance = auth.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
                let t0 = Instant::now();
                let verdict = issue_verdict(
                    token, compliance, Decision::Allow,
                    "load test".to_string(), &*sink,
                ).expect("issue_verdict should succeed under load");
                let elapsed = t0.elapsed();
                assert!(verdict.verify_integrity());
                local_latencies.push(elapsed);
            }
            local_latencies
        })
        .collect();

    assert_eq!(latencies.len(), total_ops);
    assert_eq!(sink.len(), total_ops);

    // Compute stats
    let mut sorted_us: Vec<f64> = latencies
        .iter()
        .map(|d| d.as_nanos() as f64 / 1000.0)
        .collect();
    sorted_us.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p = |q: f64| -> f64 {
        let idx = ((sorted_us.len() as f64 - 1.0) * q).round() as usize;
        sorted_us[idx.min(sorted_us.len() - 1)]
    };
    let mean: f64 = sorted_us.iter().sum::<f64>() / sorted_us.len() as f64;
    let variance: f64 =
        sorted_us.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / sorted_us.len() as f64;
    let stdev = variance.sqrt();
    let total_time_s: f64 = latencies.iter().map(|d| d.as_secs_f64()).sum::<f64>();
    let throughput = total_ops as f64 / total_time_s;

    println!(
        "[load_stats] threads={n_threads} ops={total_ops} \
         p50={:.2}us p95={:.2}us p99={:.2}us mean={:.2}us stdev={:.2}us \
         throughput={:.0}ops/s",
        p(0.50), p(0.95), p(0.99), mean, stdev, throughput
    );

    // Soft assertions (with wide margins — never use tight bounds)
    // Under QEMU emulation, ARM64 throughput is ~20-30× slower than native.
    assert!(p(0.99) < 5000.0, "p99 must be under 5ms in-memory; got {:.2}us", p(0.99));
    assert!(throughput > 1_000.0, "throughput must exceed 1k ops/s; got {:.0}", throughput);

    // Detect if we're under QEMU emulation; if so, write a separate CSV.
    let is_qemu = std::env::var("BTV_UNDER_QEMU").is_ok()
        || p(0.50) > 100.0;  // heuristic: native p50 is <30us, QEMU is >200us
    let csv_name = if is_qemu { "load_stats_arm64_qemu.csv" } else { "load_stats.csv" };
    let csv_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("reports")
        .join(csv_name);
    if let Some(parent) = csv_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let csv = format!(
        "metric,value\n\
         threads,{n_threads}\n\
         ops_per_thread,{ops_per_thread}\n\
         total_ops,{total_ops}\n\
         p50_us,{:.3}\n\
         p95_us,{:.3}\n\
         p99_us,{:.3}\n\
         mean_us,{:.3}\n\
         stdev_us,{:.3}\n\
         throughput_ops_per_s,{:.0}\n\
         emulated,{is_qemu}\n",
        p(0.50), p(0.95), p(0.99), mean, stdev, throughput
    );
    std::fs::write(&csv_path, csv).expect("write load_stats csv");
}

// Bring in num_cpus if not available as a crate
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }
}
use num_cpus::get as num_cpus;
