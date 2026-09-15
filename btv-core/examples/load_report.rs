#![allow(clippy::pedantic)]
//! Deliberate evidence generator for `reports/load_stats*.csv` — OS-08 gate
//! (COMSI-2026-04-0112, closes F9).
//!
//! F9: `tests/test_load.rs` used to write this report as a side effect of
//! `cargo test`, with `n_threads = available_parallelism()` (whatever the
//! CI runner or local machine happened to have) and an "under QEMU" label
//! decided by the heuristic `p50 > 100us`. That meant: (a) the number cited
//! in README/RELEASE_NOTES did not reproduce from the documented command on
//! a different machine, (b) trying to reproduce it silently overwrote the
//! committed evidence, and (c) a loaded native machine could mislabel its
//! own numbers as "ARM64 emulado".
//!
//! This example is the single, deliberate producer of that evidence — the
//! same pattern as `examples/rss_probe_fail_secure.rs`. It is never run by
//! `cargo test`/CI as a side effect; it is run by hand (or by a documented
//! CI *artifact* step, never a commit-back step) when the evidence needs
//! refreshing, and it always prints the exact command + toolchain + arch
//! fingerprint alongside the numbers, per OS-08's "every published number
//! carries its command and fingerprint" gate.
//!
//! Run:
//!   cargo run --release --features test-support --example load_report
//!
//! Threads: explicit default (4), override via `BTV_LOAD_THREADS=<n>`.
//! Emulation label: ONLY from `std::env::consts::ARCH` (compile-time target,
//! not a latency guess) plus the explicit `BTV_UNDER_QEMU=1` flag CI already
//! sets on the aarch64-via-QEMU job — never inferred from measured latency.

use btv_core::{issue_verdict, ComplianceAuthority, Decision, EvidenceToken, InMemoryLogSink};
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;

fn load_threads() -> usize {
    std::env::var("BTV_LOAD_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4)
}

fn main() {
    let n_threads = load_threads();
    let ops_per_thread = 1_000;
    let total_ops = n_threads * ops_per_thread;

    let sink = Arc::new(InMemoryLogSink::new());
    let authority = Arc::new(ComplianceAuthority::new_from_env());

    let wall_start = Instant::now();
    let latencies_ns: Vec<u64> = (0..n_threads)
        .into_par_iter()
        .flat_map(|thread_id| {
            let sink = Arc::clone(&sink);
            let auth = Arc::clone(&authority);
            let mut local: Vec<u64> = Vec::with_capacity(ops_per_thread);
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
                local.push(elapsed.as_nanos() as u64);
            }
            local
        })
        .collect();
    let wall_elapsed = wall_start.elapsed();

    let mut sorted_us: Vec<f64> = latencies_ns.iter().map(|&ns| ns as f64 / 1000.0).collect();
    sorted_us.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p = |q: f64| -> f64 {
        let idx = ((sorted_us.len() as f64 - 1.0) * q).round() as usize;
        sorted_us[idx.min(sorted_us.len() - 1)]
    };
    let mean: f64 = sorted_us.iter().sum::<f64>() / sorted_us.len() as f64;
    let variance: f64 =
        sorted_us.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / sorted_us.len() as f64;
    let stdev = variance.sqrt();
    // Wall-clock throughput (OS-08 / F8): total_ops / wall_clock, not
    // total_ops / sum-of-latencies (which is not throughput under
    // concurrency — it undercounts by roughly a factor of n_threads).
    let throughput = total_ops as f64 / wall_elapsed.as_secs_f64();

    // Architecture + emulation: from the compile-time target and an
    // explicit env flag only — never from measured latency (F9). Filenames
    // match the pre-existing README/RELEASE_NOTES/EVIDENCE-MANIFEST
    // references (`load_stats.csv`, `load_stats_arm64_qemu.csv`); only the
    // DECISION mechanism changes, not the naming scheme.
    let arch = std::env::consts::ARCH;
    let under_qemu = std::env::var("BTV_UNDER_QEMU").is_ok();
    let csv_name = if under_qemu {
        "load_stats_arm64_qemu.csv".to_string()
    } else {
        "load_stats.csv".to_string()
    };

    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root resolves");
    let reports_dir = repo_root.join("reports");
    std::fs::create_dir_all(&reports_dir).expect("reports/ exists");
    let csv_path = reports_dir.join(&csv_name);

    let rustc_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());
    let uname = std::process::Command::new("uname")
        .arg("-a")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());

    let csv = format!(
        "# load_report — OS-08 gate (closes F9)\n\
         # Command: cargo run --release --features test-support --example load_report\n\
         # BTV_LOAD_THREADS={n_threads} (explicit; default 4) BTV_UNDER_QEMU={under_qemu}\n\
         # arch={arch} rustc={} uname={}\n\
         metric,value\n\
         threads,{n_threads}\n\
         ops_per_thread,{ops_per_thread}\n\
         total_ops,{total_ops}\n\
         wall_ms,{:.3}\n\
         p50_us,{:.3}\n\
         p95_us,{:.3}\n\
         p99_us,{:.3}\n\
         mean_us,{:.3}\n\
         stdev_us,{:.3}\n\
         throughput_ops_per_s,{:.0}\n\
         emulated,{under_qemu}\n\
         arch,{arch}\n",
        rustc_version.trim(),
        uname.trim(),
        wall_elapsed.as_secs_f64() * 1000.0,
        p(0.50),
        p(0.95),
        p(0.99),
        mean,
        stdev,
        throughput
    );
    std::fs::write(&csv_path, &csv).expect("write load_stats csv");

    println!("{csv}");
    println!("Wrote {}", csv_path.display());
}
