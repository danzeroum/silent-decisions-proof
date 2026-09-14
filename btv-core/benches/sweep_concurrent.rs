//! sweep_concurrent.rs — Concurrency sweep harness for BTV verdict construction.
//!
//! Measures per-thread p50/p95/p99 latency and aggregate throughput of
//! EvidenceToken + ComplianceToken -> Verdict::new under Rayon parallelism,
//! isolating the absence of structural lock contention claimed in Section 5.
//!
//! Run:  cargo run --release --bin sweep_concurrent > data/sweep_raw.csv
//!
//! Anti-LLVM strategy (three locks):
//!   1. Iteration index written into the first 8 bytes of the payload each
//!      iteration -> input is never bit-identical twice; no constant-folding.
//!   2. black_box(&verdict) before drop -> no dead-code elimination.
//!   3. black_box(&payload) at token construction -> no full precomputation
//!      of the buffer by the optimizer.
//!
//! Calibration methodology:
//!   Iteration count is calibrated ONCE per (payload_bytes, mode) pair,
//!   BEFORE the thread-count loop, and reused across all thread counts and
//!   all trials for that pair. This is deliberate: per-operation cost is
//!   measured single-threaded, and recalibrating per thread-count would
//!   introduce sampling noise into the very axis (thread count) whose
//!   comparability the experiment exists to establish.
//!
//! API wiring (btv-core 0.2.0, signatures inspected in src/lib.rs):
//!   - `EvidenceToken::new(&[u8]) -> EvidenceToken` — infallible (BLAKE3).
//!   - `ComplianceToken` has NO public constructor; tokens are issued by
//!     `ComplianceAuthority::issue_token(&str, &str, u32) -> Result<_, BtvError>`.
//!     The sweep uses the always-available public constructor
//!     `ComplianceAuthority::new(key, allowlist)` (NOT `new_for_test`, which
//!     is gated on the `test-support` feature) so that plain
//!     `cargo run --release --bin sweep_concurrent` works with no feature
//!     flags. The authority is built ONCE outside the timed region; only
//!     `issue_token` participates in the measured full pipeline.
//!   - `Verdict::new(EvidenceToken, ComplianceToken, Decision, String)`
//!     is infallible — Theorem 4.1's guarantee is compile-time.

#![allow(clippy::pedantic)] // matches the existing bench convention (benches/verdict_construction.rs)

use rayon::prelude::*;
use std::hint::black_box;
use std::time::{Duration, Instant};

const PAYLOAD_SIZES: &[usize] = &[64, 512, 4096];
const TARGET_WALL_PER_CONFIG: Duration = Duration::from_secs(90);
const CALIBRATION_SAMPLES: usize = 2_000;
const MIN_ITERS: usize = 1_000_000;
const TRIALS: usize = 5;

#[derive(Clone, Copy)]
enum Mode {
    FullPipeline,
    VerdictOnly,
}

impl Mode {
    fn as_str(&self) -> &'static str {
        match self {
            Mode::FullPipeline => "full_pipeline",
            Mode::VerdictOnly => "verdict_only",
        }
    }
}

struct ThreadResult {
    durations: Vec<Duration>,
}

fn calibrate_iters(
    target_wall: Duration,
    payload: &mut [u8],
    mut op: impl FnMut(&mut [u8]) -> Duration,
) -> usize {
    let mut total = Duration::ZERO;
    for _ in 0..CALIBRATION_SAMPLES {
        total += op(payload);
    }
    let per_op = total.as_secs_f64() / CALIBRATION_SAMPLES as f64;
    ((target_wall.as_secs_f64() / per_op) as usize).max(MIN_ITERS)
}

fn run_thread<F, G, E, C, V>(
    payload_len: usize,
    mode: Mode,
    iters: usize,
    evidence_new: &F,
    compliance_new: &G,
    verdict_new: fn(E, C) -> V,
) -> ThreadResult
where
    F: Fn(&[u8]) -> E,
    G: Fn() -> C,
{
    let mut payload = vec![0u8; payload_len];
    for (i, b) in payload.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }

    let warmup = (iters / 20).max(100);

    let mut prebuilt: Option<Vec<(E, C)>> = if matches!(mode, Mode::VerdictOnly) {
        let mut v = Vec::with_capacity(iters + warmup);
        for iter in 0..(iters + warmup) {
            payload[0..8].copy_from_slice(&(iter as u64).to_le_bytes());
            v.push((evidence_new(black_box(&payload)), compliance_new()));
        }
        Some(v)
    } else {
        None
    };

    for iter in 0..warmup {
        let (e, c) = match &mut prebuilt {
            Some(v) => v.pop().expect("warmup budget exhausted"),
            None => {
                payload[0..8].copy_from_slice(&(iter as u64).to_le_bytes());
                (evidence_new(black_box(&payload)), compliance_new())
            }
        };
        black_box(&verdict_new(e, c));
    }

    let mut durations = Vec::with_capacity(iters);
    for iter in 0..iters {
        let start = Instant::now();
        let (e, c) = match &mut prebuilt {
            Some(v) => v.pop().expect("iteration budget exhausted"),
            None => {
                payload[0..8].copy_from_slice(&(iter as u64).to_le_bytes());
                (evidence_new(black_box(&payload)), compliance_new())
            }
        };
        black_box(&verdict_new(e, c));
        durations.push(start.elapsed());
    }

    ThreadResult { durations }
}

struct SweepRow {
    mode: Mode,
    threads: usize,
    payload_bytes: usize,
    wall_ms: f64,
    throughput_ops_s: f64,
    p50_ns: f64,
    p95_ns: f64,
    p99_ns: f64,
}

fn percentile(sorted: &[Duration], q: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    let rank = (q / 100.0) * (n as f64 - 1.0);
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    let a = sorted[lo].as_nanos() as f64;
    let b = sorted[hi].as_nanos() as f64;
    a + (b - a) * frac
}

fn aggregate(
    mode: Mode,
    threads: usize,
    payload_bytes: usize,
    results: Vec<ThreadResult>,
    wall: Duration,
) -> SweepRow {
    let mut all: Vec<Duration> =
        Vec::with_capacity(results.iter().map(|r| r.durations.len()).sum());
    for r in results {
        all.extend(r.durations);
    }
    all.sort_unstable();
    let total_ops = all.len() as f64;
    SweepRow {
        mode,
        threads,
        payload_bytes,
        wall_ms: wall.as_secs_f64() * 1e3,
        throughput_ops_s: total_ops / wall.as_secs_f64(),
        p50_ns: percentile(&all, 50.0),
        p95_ns: percentile(&all, 95.0),
        p99_ns: percentile(&all, 99.0),
    }
}

fn sweep<F, G, E, C, V>(
    thread_counts: &[usize],
    evidence_new: F,
    compliance_new: G,
    verdict_new: fn(E, C) -> V,
) where
    F: Fn(&[u8]) -> E + Sync,
    G: Fn() -> C + Sync,
    V: Send,
{
    println!("trial,mode,threads,payload_bytes,iters_per_thread,wall_ms,throughput_ops_s,p50_ns,p95_ns,p99_ns");

    let modes = [Mode::FullPipeline, Mode::VerdictOnly];

    for &payload_bytes in PAYLOAD_SIZES {
        for &mode in &modes {
            let mut cal_payload = vec![0u8; payload_bytes];
            for (i, b) in cal_payload.iter_mut().enumerate() {
                *b = (i % 251) as u8;
            }
            let mut cal_iter: u64 = 0;
            let iters = calibrate_iters(TARGET_WALL_PER_CONFIG, &mut cal_payload, |buf| {
                cal_iter += 1;
                buf[0..8].copy_from_slice(&cal_iter.to_le_bytes());
                let start = Instant::now();
                let e = evidence_new(black_box(&*buf));
                let c = compliance_new();
                black_box(&verdict_new(e, c));
                start.elapsed()
            });

            for trial in 0..TRIALS {
                for &threads in thread_counts {
                    let pool = rayon::ThreadPoolBuilder::new()
                        .num_threads(threads)
                        .build()
                        .expect("thread pool");

                    let wall_start = Instant::now();
                    let results: Vec<ThreadResult> = pool.install(|| {
                        (0..threads)
                            .into_par_iter()
                            .map(|_| {
                                run_thread(
                                    payload_bytes,
                                    mode,
                                    iters,
                                    &evidence_new,
                                    &compliance_new,
                                    verdict_new,
                                )
                            })
                            .collect()
                    });
                    let wall = wall_start.elapsed();

                    let row = aggregate(mode, threads, payload_bytes, results, wall);
                    println!(
                        "{},{},{},{},{},{:.2},{:.0},{:.0},{:.0},{:.0}",
                        trial,
                        row.mode.as_str(),
                        row.threads,
                        row.payload_bytes,
                        iters,
                        row.wall_ms,
                        row.throughput_ops_s,
                        row.p50_ns,
                        row.p95_ns,
                        row.p99_ns
                    );
                }
            }
        }
    }
}

fn main() {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);

    let mut thread_counts: Vec<usize> = std::iter::successors(Some(1usize), |t| Some(t * 2))
        .take_while(|&t| t <= cores)
        .collect();
    if !thread_counts.contains(&cores) {
        thread_counts.push(cores);
    }

    use btv_core::{ComplianceAuthority, Decision, EvidenceToken, Verdict};

    // Bench-local authority via the always-public constructor. `new_for_test`
    // is feature-gated (`test-support`) and the sweep must run without
    // feature flags; `new_from_env` reads BTV_AUTHORITY_KEY (nondeterministic
    // across machines). Fixed key + single-jurisdiction allowlist keeps the
    // measured `issue_token` path deterministic and identical across runs.
    let authority = ComplianceAuthority::new(
        b"btv-sweep-authority-key".to_vec(),
        vec!["EU-GDPR".to_string()],
    );

    sweep(
        &thread_counts,
        |ctx: &[u8]| EvidenceToken::new(ctx),
        || {
            authority
                .issue_token("EU-GDPR", "sweep-policy-1", 720)
                .expect("EU-GDPR is allowlisted above")
        },
        |e, c| Verdict::new(e, c, Decision::Deny, "sweep-bench".to_string()),
    );
}
