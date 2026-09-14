//! sweep_concurrent.rs — Concurrency sweep harness for BTV verdict construction.
//!
//! Measures per-thread p50/p95/p99 latency and aggregate throughput of
//! EvidenceToken + ComplianceToken -> Verdict::new under Rayon parallelism,
//! isolating the absence of structural lock contention claimed in Section 5,
//! PLUS (Round 3, Task B1) a minimal status-quo baseline mode,
//! Mode::StatusQuoAsyncLog, demanded by the editorial letter's E2: the same
//! decision data (payload + jurisdiction + policy version + deadline +
//! explanation) serialized as a JSON record and handed to a fire-and-forget
//! async logging channel, with NO linear-type discipline — the structural
//! guarantee is precisely what the status quo lacks, and the contrast is the
//! measurement.
//!
//! Run:  cargo run --release --features sweep-bench --bin sweep_concurrent \
//!           > data/sweep_raw.csv
//!
//! Reduced-footprint collection (Round 3, Task B2): set
//! BTV_SWEEP_TARGET_WALL_SECS=<seconds> to shrink the per-configuration
//! wall-clock target below the committed 90 s default — e.g. 10 s for a
//! sandboxed or CI-hosted representative run. The value used is part of the
//! run's provenance and must be recorded alongside the CSV
//! (scripts/run_sweep.sh writes it into data/sweep_env_<timestamp>.txt).
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
//! StatusQuoAsyncLog mode (Round 3, Task B1 — the E2 baseline):
//!   Implements the minimal status-quo pattern the paper describes as the
//!   dominant industry practice ("a system may emit a denial, log it
//!   asynchronously, and silently drop the log"): serialize a structured
//!   JSON decision record and hand it to an async channel, fire-and-forget.
//!   The record carries the SAME decision data as the BTV modes — decision,
//!   jurisdiction, policy version, appeal deadline, explanation — plus the
//!   FULL decision context, hex-encoded: the status quo has no compact
//!   cryptographic binding (a 32-byte BLAKE3 digest) and must embed the
//!   whole context to preserve any evidentiary value, so its record size
//!   scales with payload size. That scaling is a property of the pattern
//!   under comparison, not a harness artifact.
//!   Channel discipline (three deliberate choices):
//!   1. THREAD-LOCAL — one channel pair per Rayon worker task, built in
//!      the setup region, NEVER shared across threads. A single global
//!      MPSC queue would inject cross-thread lock contention into the hot
//!      path and strawman the baseline: BTV's hashing path is lock-free,
//!      so an unfairly slow baseline would be (rightly) rejected by a
//!      reviewer. What is being measured is the per-decision cost of the
//!      logging pattern — serialization, allocation, hand-off — not the
//!      provisioning of a shared queue.
//!   2. RECEIVER DROPPED AT SETUP — send() therefore takes the
//!      disconnected fast path (Err, non-blocking, no queue growth): the
//!      black-box dummy sink. This keeps resident memory O(1) per thread
//!      across millions of iterations (an undrained live queue would grow
//!      linearly and OOM the sweep) while still exercising the real send
//!      path. The fail-open SEMANTICS this implies — the decision "goes
//!      out" even though the record is never persisted — are demonstrated
//!      in the dedicated integration test tests/test_status_quo_contrast.rs
//!      (fail-open) against Verdict::new's structural token-consumption
//!      requirement (fail-secure).
//!   3. SEND ERROR EXPLICITLY IGNORED — `let _ = tx.send(...)` is the
//!      correct expression of fire-and-forget: unwrapping or propagating
//!      the Err would panic the worker thread and fake a fail-secure
//!      behavior the status quo does not have.
//!
//! Calibration methodology:
//!   Iteration count is calibrated ONCE per (payload_bytes, mode) pair,
//!   BEFORE the thread-count loop, and reused across all thread counts and
//!   all trials for that pair. This is deliberate: per-operation cost is
//!   measured single-threaded, and recalibrating per thread-count would
//!   introduce sampling noise into the very axis (thread count) whose
//!   comparability the experiment exists to establish. The calibration
//!   probe measures the FULL pipeline in all modes, so a VerdictOnly or
//!   StatusQuoAsyncLog configuration runs fewer (or, for large payloads
//!   where serializing the whole context costs more than hashing it, more)
//!   wall-seconds than the target while iteration counts stay comparable
//!   across modes — which is what the cross-mode comparison requires.
//!   The committed default target is 90 s per configuration;
//!   BTV_SWEEP_TARGET_WALL_SECS overrides it for reduced-footprint
//!   collection runs (documented per-run in the sweep_env fingerprint).
//!
//! API wiring (btv-core 0.2.0, signatures inspected in src/lib.rs):
//!   - `EvidenceToken::new(&[u8]) -> EvidenceToken` — infallible (BLAKE3).
//!   - `ComplianceToken` has NO public constructor; tokens are issued by
//!     `ComplianceAuthority::issue_token(&str, &str, u32) -> Result<_, BtvError>`,
//!     HMAC-signed with the authority key. The sweep uses
//!     `ComplianceAuthority::new_from_env()` (NOT `new_for_test`, which is
//!     gated on the `test-support` feature) so that plain
//!     `cargo run --release --features sweep-bench --bin sweep_concurrent`
//!     works with no extra feature flags. The authority is built ONCE
//!     outside the timed region; only `issue_token` participates in the
//!     measured full pipeline. Signature VALUES depend on the resolved key
//!     but issuance/verification timing does not (fixed-size HMAC over
//!     fixed-size fields).
//!   - `Verdict::new(EvidenceToken, ComplianceToken, Decision, String)
//!     -> Result<Verdict, BtvError>` — verifies the compliance token's
//!     authority signature (OS-02) before constructing; the sweep's tokens
//!     come from the recognized (env-resolved) authority, so construction
//!     succeeds. The compile-time linear-type guarantee of Theorem 4.1 is
//!     unchanged; the signature check is the runtime part of L2.

#![allow(clippy::pedantic)] // matches the existing bench convention (benches/verdict_construction.rs)

use rayon::prelude::*;
use std::hint::black_box;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PAYLOAD_SIZES: &[usize] = &[64, 512, 4096];
const DEFAULT_TARGET_WALL_PER_CONFIG_SECS: u64 = 90;
const CALIBRATION_SAMPLES: usize = 2_000;
const MIN_ITERS: usize = 1_000_000;
const TRIALS: usize = 5;

/// Wall-clock target per (payload_bytes, mode) configuration. The committed
/// default (90 s) is the value the paper's headline data collection must
/// use. BTV_SWEEP_TARGET_WALL_SECS overrides it for reduced-footprint runs
/// — e.g. 10 s in the Round 3 sandbox/CI collection — and the effective
/// value is part of each run's provenance (stderr banner + sweep_env
/// fingerprint written by scripts/run_sweep.sh).
fn target_wall_per_config() -> Duration {
    match std::env::var("BTV_SWEEP_TARGET_WALL_SECS") {
        Ok(raw) => raw
            .trim()
            .parse::<u64>()
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(DEFAULT_TARGET_WALL_PER_CONFIG_SECS)),
        Err(_) => Duration::from_secs(DEFAULT_TARGET_WALL_PER_CONFIG_SECS),
    }
}

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
    /// Minimal status-quo baseline (Round 3, Task B1): JSON serialization
    /// of the same decision data + fire-and-forget channel hand-off, with
    /// NO linear-type discipline. See the file header for the channel
    /// discipline (thread-local pair, dropped receiver, ignored send
    /// error) and tests/test_status_quo_contrast.rs for the fail-open vs.
    /// fail-secure semantics this mode stands for.
    StatusQuoAsyncLog,
}

impl Mode {
    fn as_str(&self) -> &'static str {
        match self {
            Mode::FullPipeline => "full_pipeline",
            Mode::VerdictOnly => "verdict_only",
            Mode::StatusQuoAsyncLog => "status_quo_async_log",
        }
    }
}

// ============================================================================
// Status-quo decision record (Task B1 — the E2 baseline)
// ============================================================================
//
/// The structured JSON record the status quo serializes per decision — a
/// SIEM-style decision event. The field set mirrors the public data of a
/// BTV `Verdict` (decision, jurisdiction, policy_version, deadline,
/// explanation) plus the FULL decision context, hex-encoded: without a
/// compact cryptographic binding (BTV's 32-byte BLAKE3 digest) the status
/// quo must embed the whole context to preserve any evidentiary value, so
/// the record scales with payload size. Same decision inputs as the other
/// modes: `Decision::Deny`, "EU-GDPR", "sweep-policy-1", 720 h deadline,
/// "sweep-bench" explanation.
#[derive(serde::Serialize)]
struct StatusQuoRecord<'a> {
    ts_unix_ms: u64,
    decision: &'a str,
    jurisdiction: &'a str,
    policy_version: &'a str,
    context_hex: String,
    appeal_deadline_hours: u32,
    explanation: &'a str,
}

/// Serialize one status-quo decision record. Timestamps each record, as a
/// real async logger does (SystemTime::now is a vDSO call, ~20–25 ns —
/// the same order as the Instant::now() the timing window itself carries).
fn build_status_quo_record(payload: &[u8]) -> String {
    let ts_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let record = StatusQuoRecord {
        ts_unix_ms,
        decision: "deny",
        jurisdiction: "EU-GDPR",
        policy_version: "sweep-policy-1",
        context_hex: hex::encode(payload),
        appeal_deadline_hours: 720,
        explanation: "sweep-bench",
    };
    serde_json::to_string(&record).expect("serializing StatusQuoRecord cannot fail")
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

    // StatusQuoAsyncLog — thread-local fire-and-forget sink (Round 3, B1).
    // One channel pair PER run_thread invocation (= per Rayon worker task),
    // built here in the setup region, OUTSIDE every timed window, and never
    // shared across threads (a single global MPSC queue would put
    // cross-thread lock contention on the measured path — a strawman
    // baseline; see the file header). The Receiver is dropped immediately
    // so send() takes the disconnected fast path: the measured cost is the
    // status quo's per-decision overhead (JSON serialization + String
    // allocation + hand-off call), not queue provisioning, and resident
    // memory stays O(1) across millions of iterations.
    let status_quo_tx = match mode {
        Mode::StatusQuoAsyncLog => {
            let (tx, rx) = std::sync::mpsc::channel::<String>();
            drop(rx);
            Some(tx)
        }
        _ => None,
    };
    let sq_tx = status_quo_tx.as_ref();

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
            Mode::StatusQuoAsyncLog => {
                // Same decision inputs as the other modes (payload bytes +
                // EU-GDPR + deny + sweep-policy-1 + 720 h), serialized as a
                // JSON record and fired at the async channel. NO linear
                // types participate — that absence IS the baseline.
                payload[0..8].copy_from_slice(&(iter as u64).to_le_bytes());
                // Resolve the thread-local sender OUTSIDE the timed window
                // (one predictable branch, untimed — same posture as the
                // untimed token construction in VerdictOnly).
                let tx = sq_tx.expect("thread-local channel built in setup");
                let start = Instant::now();
                let record = build_status_quo_record(black_box(&payload));
                // Fire-and-forget: the send result is EXPLICITLY ignored.
                // Unwrapping here would panic the worker on the very
                // disconnected-channel condition this mode deliberately
                // arranges — faking a fail-secure behavior the status quo
                // does not have. The decision is already "out" the moment
                // this arm returns; whether the record is ever persisted is
                // not this code's concern (see
                // tests/test_status_quo_contrast.rs).
                let _ = tx.send(black_box(record));
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
    //   mode — full_pipeline | verdict_only | status_quo_async_log
    //       (status_quo_async_log = the E2 baseline: same decision data,
    //       JSON serialization + fire-and-forget channel hand-off, no
    //       linear-type discipline)
    //   p50_ns / p95_ns / p99_ns — arithmetic MEAN across threads of the
    //       thread-local P² quantile ESTIMATES (online, O(1) memory — not
    //       exact order statistics; Jain & Chlamtac 1985).
    //   p99_max_ns — worst single thread's p99 estimate (straggler tail).
    //   mean_ns    — arithmetic mean latency over all timed operations.
    println!("trial,mode,threads,payload_bytes,iters_per_thread,wall_ms,throughput_ops_s,p50_ns,p95_ns,p99_ns,p99_max_ns,mean_ns");

    let modes = [
        Mode::FullPipeline,
        Mode::VerdictOnly,
        Mode::StatusQuoAsyncLog,
    ];
    let target_wall = target_wall_per_config();

    for &payload_bytes in PAYLOAD_SIZES {
        for &mode in &modes {
            let mut cal_payload = vec![0u8; payload_bytes];
            for (i, b) in cal_payload.iter_mut().enumerate() {
                *b = (i % 251) as u8;
            }
            let mut cal_iter: u64 = 0;
            let iters = calibrate_iters(target_wall, &mut cal_payload, |buf| {
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

    // Provenance banner — STDERR ONLY (stdout is the CSV data stream and
    // must stay machine-parseable). scripts/run_sweep.sh captures the
    // environment fingerprint separately; this banner ties the mode list
    // and the effective wall target to the run itself.
    eprintln!(
        "sweep_concurrent provenance: modes=[full_pipeline, verdict_only, \
         status_quo_async_log] payloads={:?} thread_counts={:?} trials={} \
         target_wall_per_config={}s (default {}s; override: \
         BTV_SWEEP_TARGET_WALL_SECS)",
        PAYLOAD_SIZES,
        thread_counts,
        TRIALS,
        target_wall_per_config().as_secs(),
        DEFAULT_TARGET_WALL_PER_CONFIG_SECS
    );

    use btv_core::{ComplianceAuthority, Decision, EvidenceToken, Verdict};

    // Recognized authority via the env-resolved key (BTV_AUTHORITY_KEY or
    // the deterministic PoC fallback). `new_for_test` is feature-gated
    // (`test-support`) and the sweep must run without extra feature flags;
    // `Verdict::new` verifies token signatures against the SAME env-resolved
    // key (OS-02), so issuer and verifier agree by construction here. A
    // custom-key authority (the old `new(key, allowlist)` call) would issue
    // tokens the verifier rejects.
    let authority = ComplianceAuthority::new_from_env();

    sweep(
        &thread_counts,
        |ctx: &[u8]| EvidenceToken::new(ctx),
        || {
            authority
                .issue_token("EU-GDPR", "sweep-policy-1", 720)
                .expect("EU-GDPR is allowlisted above")
        },
        |e, c| {
            Verdict::new(e, c, Decision::Deny, "sweep-bench".to_string())
                .expect("recognized-authority tokens verify by construction")
        },
    );
}
