//! sweep_concurrent.rs — Concurrency sweep harness for BTV verdict construction.
//!
//! Measures per-thread p50/p95/p99 latency and aggregate throughput of
//! EvidenceToken + ComplianceToken -> Verdict::new under Rayon parallelism,
//! isolating the absence of structural lock contention claimed in Section 5.
//!
//! Run:  cargo run --release --features sweep-bench --bin sweep_concurrent \
//!           > data/sweep_raw.csv
//!
//! The binary is gated behind the opt-in `sweep-bench` feature so that plain
//! `cargo build --workspace` / `cargo test --workspace` (and CI) never
//! compile it; it builds only on demand, via scripts/run_sweep.sh or the
//! command above.
//!
//! Memory profile — O(1) per thread (Round 2):
//!   Percentiles are computed by an online P² quantile estimator (Jain &
//!   Chlamtac, 1985), which keeps five markers per tracked quantile and
//!   never stores individual observations. This replaced the previous
//!   exact-storage design — one Duration per timed iteration per thread
//!   (3–58 GB aggregate at the 90 s calibration target), plus an equally
//!   unbounded prebuilt Vec<(EvidenceToken, ComplianceToken)> in
//!   VerdictOnly mode — which OOM-killed reference runs. Resident memory
//!   is now independent of iteration count and thread count.
//!   The paper's methodology (Section 5.1) MUST declare that reported
//!   percentiles are P² estimates, not exact order statistics, and cite:
//!   R. Jain & I. Chlamtac, "The P² Algorithm for Dynamic Calculation of
//!   Quantiles and Histograms Without Storing Observations", Comm. ACM
//!   28(10), 1985.
//!
//! Anti-LLVM strategy (three locks):
//!   1. Iteration index written into the first 8 bytes of the payload each
//!      iteration -> input is never bit-identical twice; no constant-folding.
//!   2. black_box(&verdict) before drop -> no dead-code elimination.
//!   3. black_box(&payload) at token construction -> no full precomputation
//!      of the buffer by the optimizer.
//!
//! Measurement methodology (Round 2 — NO synchronization in the hot path):
//!   Each Rayon worker owns thread-local P2Estimator instances. There is
//!   deliberately NO Mutex/RwLock/atomic in the measurement loop: inserting
//!   one would put harness synchronization latency into the very tail
//!   measurements this experiment exists to characterize — the p99 would
//!   measure the harness's lock, not BTV's (absent) structural contention.
//!   Cross-thread aggregation therefore happens AFTER the run, in
//!   `aggregate`: the reported p50/p95/p99 are the arithmetic means of the
//!   per-thread P² estimates, and p99_max_ns is the worst single thread
//!   (stragglers cannot hide behind the mean). Trade-off, documented for
//!   the methodology section: mean-of-per-thread-quantiles is exact for
//!   homogeneous workers and mildly conservative for heterogeneous ones —
//!   the correct posture for a no-contention claim facing a reviewer.
//!
//!   VerdictOnly mode times ONLY Verdict::new. BTV tokens are linear
//!   (single-use; not Clone by design — cloning would break the
//!   V ⊸ (E ⊗ C) discipline of Theorem 4.1), so a fresh token pair must
//!   be constructed for every verdict. That construction happens OUTSIDE
//!   the timed window and costs O(1) memory (the old prebuilt buffer is
//!   gone). Consequently the wall time of a VerdictOnly configuration
//!   includes untimed token construction and its throughput column is
//!   construction-bound by design; the latency columns are the mode's
//!   payload. Each timed window carries one Instant::now()/elapsed() pair
//!   (~20–25 ns), identical in both modes, so it cancels in cross-mode
//!   differences.
//!
//! Calibration methodology:
//!   Iteration count is calibrated ONCE per (payload_bytes, mode) pair,
//!   BEFORE the thread-count loop, and reused across all thread counts and
//!   all trials for that pair. This is deliberate: per-operation cost is
//!   measured single-threaded, and recalibrating per thread-count would
//!   introduce sampling noise into the very axis (thread count) whose
//!   comparability the experiment exists to establish. The calibration
//!   probe measures the FULL pipeline in both modes, so a VerdictOnly
//!   configuration runs fewer wall-seconds than the 90 s target (its
//!   per-op cost is lower) while iteration counts stay comparable across
//!   modes — which is what the cross-mode comparison requires.
//!
//! API wiring (btv-core 0.2.0, signatures inspected in src/lib.rs):
//!   - `EvidenceToken::new(&[u8]) -> EvidenceToken` — infallible (BLAKE3).
//!   - `ComplianceToken` has NO public constructor; tokens are issued by
//!     `ComplianceAuthority::issue_token(&str, &str, u32) -> Result<_, BtvError>`.
//!     The sweep uses the always-available public constructor
//!     `ComplianceAuthority::new(key, allowlist)` (NOT `new_for_test`, which
//!     is gated on the `test-support` feature) so that plain
//!     `cargo run --release --features sweep-bench --bin sweep_concurrent`
//!     works with no extra feature flags. The authority is built ONCE
//!     outside the timed region; only `issue_token` participates in the
//!     measured full pipeline.
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

// ============================================================================
// Online P² quantile estimator (Jain & Chlamtac, 1985)
// ============================================================================
//
/// O(1) memory per tracked quantile — no per-sample storage required.
/// Reference: "The P² Algorithm for Dynamic Calculation of Quantiles and
/// Histograms Without Storing Observations", Comm. ACM 28(10), 1985.
/// Cited here for the Section 5.1 methodology declaration: reported
/// percentiles are P² estimates, not exact order statistics.
struct P2Estimator {
    q: f64,
    n: [f64; 5],
    np: [f64; 5],
    dn: [f64; 5],
    heights: [f64; 5],
    count: usize,
}

impl P2Estimator {
    fn new(q: f64) -> Self {
        Self {
            q,
            n: [1.0, 2.0, 3.0, 4.0, 5.0],
            np: [1.0, 1.0 + 2.0 * q, 1.0 + 4.0 * q, 3.0 + 2.0 * q, 5.0],
            dn: [0.0, q / 2.0, q, (1.0 + q) / 2.0, 1.0],
            heights: [0.0; 5],
            count: 0,
        }
    }

    fn observe(&mut self, x: f64) {
        self.count += 1;
        if self.count <= 5 {
            self.heights[self.count - 1] = x;
            if self.count == 5 {
                self.heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
            }
            return;
        }

        let k = if x < self.heights[0] {
            self.heights[0] = x;
            0
        } else if x >= self.heights[4] {
            self.heights[4] = x;
            3
        } else {
            (0..4).find(|&i| x < self.heights[i + 1]).unwrap_or(3)
        };

        for i in (k + 1)..5 {
            self.n[i] += 1.0;
        }
        for i in 0..5 {
            self.np[i] += self.dn[i];
        }

        for i in 1..4 {
            let d = self.np[i] - self.n[i];
            if (d >= 1.0 && self.n[i + 1] - self.n[i] > 1.0)
                || (d <= -1.0 && self.n[i - 1] - self.n[i] < -1.0)
            {
                let sign = d.signum();
                let qi = self.parabolic(i, sign);
                self.heights[i] = if self.heights[i - 1] < qi && qi < self.heights[i + 1] {
                    qi
                } else {
                    self.linear(i, sign)
                };
                self.n[i] += sign;
            }
        }
    }

    fn parabolic(&self, i: usize, d: f64) -> f64 {
        let (n, h) = (&self.n, &self.heights);
        h[i] + d / (n[i + 1] - n[i - 1])
            * ((n[i] - n[i - 1] + d) * (h[i + 1] - h[i]) / (n[i + 1] - n[i])
                + (n[i + 1] - n[i] - d) * (h[i] - h[i - 1]) / (n[i] - n[i - 1]))
    }

    fn linear(&self, i: usize, d: f64) -> f64 {
        let j = (i as f64 + d) as usize;
        self.heights[i] + d * (self.heights[j] - self.heights[i]) / (self.n[j] - self.n[i])
    }

    fn estimate(&self) -> f64 {
        if self.count < 5 {
            let mut sorted = self.heights[..self.count].to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let idx = ((self.q * (self.count.max(1) - 1) as f64).round() as usize)
                .min(self.count.saturating_sub(1));
            sorted.get(idx).copied().unwrap_or(0.0)
        } else {
            self.heights[2]
        }
    }
}

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

/// Per-thread measurement state — O(1) memory: three P² estimators (five
/// f64 markers each) plus two scalars. Replaces the previous exact-storage
/// `Vec<Duration>` (16 bytes per timed iteration per thread) that produced
/// the 3–58 GB aggregate footprint documented in Round 1.
struct ThreadResult {
    p50: P2Estimator,
    p95: P2Estimator,
    p99: P2Estimator,
    sample_count: u64,
    sum_ns: f64,
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

    // Thread-local P² estimators — see the file header for why nothing
    // shared (Mutex/RwLock/atomic) may appear in this loop.
    let mut p50 = P2Estimator::new(0.50);
    let mut p95 = P2Estimator::new(0.95);
    let mut p99 = P2Estimator::new(0.99);
    let mut sample_count: u64 = 0;
    let mut sum_ns: f64 = 0.0;

    for iter in 0..(warmup + iters) {
        let observing = iter >= warmup;
        // FullPipeline: token construction is INSIDE the timed window (that
        // is the mode's definition). VerdictOnly: construction is untimed
        // and O(1)-memory — tokens are linear (not Clone by design), so a
        // fresh pair is built per verdict outside the window; only
        // Verdict::new is measured.
        let elapsed_ns = match mode {
            Mode::FullPipeline => {
                payload[0..8].copy_from_slice(&(iter as u64).to_le_bytes());
                let start = Instant::now();
                let e = evidence_new(black_box(&payload));
                let c = compliance_new();
                black_box(&verdict_new(e, c));
                start.elapsed().as_nanos() as f64
            }
            Mode::VerdictOnly => {
                payload[0..8].copy_from_slice(&(iter as u64).to_le_bytes());
                let e = evidence_new(black_box(&payload));
                let c = compliance_new();
                let start = Instant::now();
                black_box(&verdict_new(e, c));
                start.elapsed().as_nanos() as f64
            }
        };
        if observing {
            p50.observe(elapsed_ns);
            p95.observe(elapsed_ns);
            p99.observe(elapsed_ns);
            sample_count += 1;
            sum_ns += elapsed_ns;
        }
    }

    ThreadResult {
        p50,
        p95,
        p99,
        sample_count,
        sum_ns,
    }
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
    p99_max_ns: f64,
    mean_ns: f64,
}

fn aggregate(
    mode: Mode,
    threads: usize,
    payload_bytes: usize,
    results: Vec<ThreadResult>,
    wall: Duration,
) -> SweepRow {
    // Cross-thread aggregation happens AFTER the run (see file header):
    // reported percentiles are means of per-thread P² estimates; p99_max_ns
    // is the worst single thread so stragglers cannot hide behind the mean.
    let total_ops: u64 = results.iter().map(|r| r.sample_count).sum();
    let total_sum_ns: f64 = results.iter().map(|r| r.sum_ns).sum();
    let p50s: Vec<f64> = results.iter().map(|r| r.p50.estimate()).collect();
    let p95s: Vec<f64> = results.iter().map(|r| r.p95.estimate()).collect();
    let p99s: Vec<f64> = results.iter().map(|r| r.p99.estimate()).collect();
    let mean_of = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    SweepRow {
        mode,
        threads,
        payload_bytes,
        wall_ms: wall.as_secs_f64() * 1e3,
        throughput_ops_s: total_ops as f64 / wall.as_secs_f64(),
        p50_ns: mean_of(&p50s),
        p95_ns: mean_of(&p95s),
        p99_ns: mean_of(&p99s),
        p99_max_ns: p99s.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        mean_ns: if total_ops > 0 {
            total_sum_ns / total_ops as f64
        } else {
            0.0
        },
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
    // CSV column semantics (documented for the Section 5.1 methodology):
    //   p50_ns / p95_ns / p99_ns — arithmetic MEAN across threads of the
    //       thread-local P² quantile ESTIMATES (online, O(1) memory — not
    //       exact order statistics; Jain & Chlamtac 1985).
    //   p99_max_ns — worst single thread's p99 estimate (straggler tail).
    //   mean_ns    — arithmetic mean latency over all timed operations.
    println!("trial,mode,threads,payload_bytes,iters_per_thread,wall_ms,throughput_ops_s,p50_ns,p95_ns,p99_ns,p99_max_ns,mean_ns");

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
                        "{},{},{},{},{},{:.2},{:.0},{:.0},{:.0},{:.0},{:.0},{:.2}",
                        trial,
                        row.mode.as_str(),
                        row.threads,
                        row.payload_bytes,
                        iters,
                        row.wall_ms,
                        row.throughput_ops_s,
                        row.p50_ns,
                        row.p95_ns,
                        row.p99_ns,
                        row.p99_max_ns,
                        row.mean_ns
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
    // extra feature flags; `new_from_env` reads BTV_AUTHORITY_KEY
    // (nondeterministic across machines). Fixed key + single-jurisdiction
    // allowlist keeps the measured `issue_token` path deterministic and
    // identical across runs.
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
