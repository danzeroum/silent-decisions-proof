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
//! OS-07 (COMSI-2026-04-0112, closes F7 and F10) adds the two honest arms:
//!
//!   - Mode::FullPipelineDurable — the BTV pipeline WITH REAL DURABLE
//!     persistence: `issue_verdict` against a `SqliteLogSink::open(path)` on
//!     an actual file, where WAL and `synchronous=FULL` are effective. F7:
//!     the pre-audit durable claim measured `open_in_memory()`, where SQLite
//!     silently ignores WAL and FULL — the reported "ACID durable" number was
//!     a RAM insert. This mode reports the honest number, whatever it is
//!     (fsync costs 0.5–10 ms on real storage; the contrast with the RAM
//!     arms is the answer to "governance is practically free?").
//!
//!   - Mode::StatusQuoDigestLog — the SYMMETRIC baseline arm (F10): hash the
//!     context (BLAKE3) and log the digest + the same metadata the other
//!     modes carry. F10: the pre-audit contrast serialized the FULL context
//!     hex-encoded (2x expansion) for the status quo while BTV stored a
//!     32-byte digest — a premise chosen by the benchmark, not a property of
//!     the standard, and one the artifact itself contradicted elsewhere.
//!     Both status-quo arms are now reported as a SENSITIVITY RANGE around
//!     what "the status quo" means, with one sentence per assumption.
//!
//! Calibriation is PER (payload, mode): each mode runs enough iterations of
//! ITS OWN operation to meet the wall target, so iteration counts are NOT
//! comparable across modes — cross-mode comparison uses throughput and
//! within-mode latency percentiles, which is the comparison the paper makes.
//! The iteration FLOOR is mode-aware: 1,000,000 for microsecond-scale arms,
//! 100 for the durable arm (a real fsync is 3 orders of magnitude slower;
//! a 1M-iteration floor would turn a 90 s target into a 24 h run).
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
//! Measurement methodology (Round 2 — NO HARNESS synchronization in the hot
//! path): each Rayon worker owns thread-local P2Estimator instances and the
//! measurement loop carries no Mutex/RwLock/atomic of the harness's own.
//! NOTE the deliberate exception for FullPipelineDurable: a SINGLE shared
//! `SqliteLogSink` is used, matching the realistic single-log deployment;
//! SQLite's internal locking IS the mode under measurement there, not a
//! harness artifact (documented per OS-07).
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

use btv_core::{
    issue_verdict, ComplianceAuthority, ComplianceToken, Decision, EvidenceToken, LogSink,
    SqliteLogSink, Verdict,
};
use rayon::prelude::*;
use std::hint::black_box;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PAYLOAD_SIZES: &[usize] = &[64, 512, 4096];
const DEFAULT_TARGET_WALL_PER_CONFIG_SECS: u64 = 90;
const CALIBRATION_SAMPLES: usize = 2_000;
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
    /// Real durable persistence (OS-07, closes F7): `issue_verdict` against
    /// an on-disk `SqliteLogSink` (WAL + synchronous=FULL effective).
    FullPipelineDurable,
    /// Minimal status-quo baseline (Round 3, Task B1): JSON serialization
    /// of the same decision data + fire-and-forget channel hand-off, with
    /// NO linear-type discipline. See the file header for the channel
    /// discipline (thread-local pair, dropped receiver, ignored send
    /// error) and tests/test_status_quo_contrast.rs for the fail-open vs.
    /// fail-secure semantics this mode stands for.
    StatusQuoAsyncLog,
    /// Symmetric baseline arm (OS-07, closes F10): hash the context to a
    /// 32-byte BLAKE3 digest and log digest + metadata, WITHOUT linear-type
    /// discipline. This is the "status quo done sensibly" arm; together
    /// with StatusQuoAsyncLog (full-context embed) it forms the
    /// sensitivity range over what "the status quo" means.
    StatusQuoDigestLog,
}

impl Mode {
    fn as_str(&self) -> &'static str {
        match self {
            Mode::FullPipeline => "full_pipeline",
            Mode::VerdictOnly => "verdict_only",
            Mode::FullPipelineDurable => "full_pipeline_durable",
            Mode::StatusQuoAsyncLog => "status_quo_async_log",
            Mode::StatusQuoDigestLog => "status_quo_digest_log",
        }
    }

    /// Iteration floor per mode (OS-07): microsecond-scale arms keep the
    /// 1M floor for statistical stability; the durable arm's per-op cost is
    /// dominated by real fsync (0.5-10 ms), where a 1M floor would turn a
    /// 90 s target into a ~day-long run. P2 still tracks the tail.
    fn min_iters(&self) -> usize {
        match self {
            Mode::FullPipelineDurable => 100,
            _ => 1_000_000,
        }
    }

    /// Whether the mode needs a thread-local fire-and-forget channel.
    fn uses_channel(&self) -> bool {
        matches!(self, Mode::StatusQuoAsyncLog | Mode::StatusQuoDigestLog)
    }
}

// ============================================================================
// Status-quo decision records (Task B1 + OS-07)
// ============================================================================

/// The structured JSON record the status quo serializes per decision — a
/// SIEM-style decision event. The field set mirrors the public data of a
/// BTV `Verdict` (decision, jurisdiction, policy_version, deadline,
/// explanation) plus the FULL decision context, hex-encoded: without a
/// compact cryptographic binding (BTV's 32-byte BLAKE3 digest) this flavor
/// of status quo embeds the whole context to preserve any evidentiary
/// value, so the record scales with payload size. Same decision inputs as
/// the other modes: `Decision::Deny`, "EU-GDPR", "sweep-policy-1", 720 h
/// deadline, "sweep-bench" explanation.
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

/// Serialize one full-context status-quo decision record. Timestamps each
/// record, as a real async logger does (SystemTime::now is a vDSO call,
/// ~20-25 ns — the same order as the Instant::now() the timing window
/// itself carries).
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

/// The SYMMETRIC arm's record (OS-07): digest of the context (BLAKE3, the
/// same primitive BTV uses) plus the same metadata — record size is
/// CONSTANT regardless of payload. This is what a status quo with a
/// compact cryptographic binding would log; the pre-audit contrast used
/// ONLY the full-context arm, which measured hex+serde_json throughput
/// rather than accountability overhead (audit F10).
#[derive(serde::Serialize)]
struct StatusQuoDigestRecord {
    ts_unix_ms: u64,
    decision: &'static str,
    jurisdiction: &'static str,
    policy_version: &'static str,
    evidence_id_hex: String,
    appeal_deadline_hours: u32,
    explanation: &'static str,
}

fn build_status_quo_digest_record(payload: &[u8]) -> String {
    let ts_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let record = StatusQuoDigestRecord {
        ts_unix_ms,
        decision: "deny",
        jurisdiction: "EU-GDPR",
        policy_version: "sweep-policy-1",
        evidence_id_hex: blake3::hash(payload).to_hex().to_string(),
        appeal_deadline_hours: 720,
        explanation: "sweep-bench",
    };
    serde_json::to_string(&record).expect("serializing StatusQuoDigestRecord cannot fail")
}

/// Per-thread measurement state — O(1) memory: three P2 estimators (five
/// f64 markers each) plus two scalars. Replaces the previous exact-storage
/// `Vec<Duration>` (16 bytes per timed iteration per thread) that produced
/// the 3-58 GB aggregate footprint documented in Round 1.
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
    min_iters: usize,
) -> usize {
    let mut total = Duration::ZERO;
    for _ in 0..CALIBRATION_SAMPLES {
        total += op(payload);
    }
    let per_op = total.as_secs_f64() / CALIBRATION_SAMPLES as f64;
    eprintln!(
        "calibration: per_op={:.0}ns -> iters={}",
        per_op * 1e9,
        ((target_wall.as_secs_f64() / per_op) as usize).max(min_iters)
    );
    ((target_wall.as_secs_f64() / per_op) as usize).max(min_iters)
}

/// The concrete per-mode timed operation. Concrete types replace the
/// previous generics (the harness ever only instantiated
/// EvidenceToken/ComplianceToken/Verdict); `durable` carries the shared
/// on-disk sink for `FullPipelineDurable` and `channels` the per-thread
/// fire-and-forget senders for the two status-quo arms.
struct ModeOps<'a> {
    evidence_new: &'a (dyn Fn(&[u8]) -> EvidenceToken + Sync),
    compliance_new: &'a (dyn Fn() -> ComplianceToken + Sync),
    verdict_new: &'a (dyn Fn(EvidenceToken, ComplianceToken) -> Verdict + Sync),
    durable_sink: Option<&'a dyn LogSink>,
}

fn run_thread(
    payload_len: usize,
    mode: Mode,
    iters: usize,
    ops: &ModeOps<'_>,
    sq_tx: Option<&std::sync::mpsc::Sender<String>>,
) -> ThreadResult {
    let mut payload = vec![0u8; payload_len];
    for (i, b) in payload.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }

    let warmup = (iters / 20).max(100);

    // Thread-local P2 estimators — nothing of the harness's own is shared
    // inside this loop (for the durable mode, SQLite's internal locking is
    // the subject of the measurement, not harness overhead).
    let mut p50 = P2Estimator::new(0.50);
    let mut p95 = P2Estimator::new(0.95);
    let mut p99 = P2Estimator::new(0.99);
    let mut sample_count: u64 = 0;
    let mut sum_ns: f64 = 0.0;

    for iter in 0..(warmup + iters) {
        let observing = iter >= warmup;
        payload[0..8].copy_from_slice(&(iter as u64).to_le_bytes());
        let elapsed_ns = match mode {
            Mode::FullPipeline => {
                let start = Instant::now();
                let e = (ops.evidence_new)(black_box(&payload));
                let c = (ops.compliance_new)();
                black_box(&(ops.verdict_new)(e, c));
                start.elapsed().as_nanos() as f64
            }
            Mode::VerdictOnly => {
                let e = (ops.evidence_new)(black_box(&payload));
                let c = (ops.compliance_new)();
                let start = Instant::now();
                black_box(&(ops.verdict_new)(e, c));
                start.elapsed().as_nanos() as f64
            }
            Mode::FullPipelineDurable => {
                let sink = ops
                    .durable_sink
                    .expect("durable sink present for FullPipelineDurable");
                let start = Instant::now();
                let e = (ops.evidence_new)(black_box(&payload));
                let c = (ops.compliance_new)();
                let v = issue_verdict(e, c, Decision::Deny, "sweep-bench".to_string(), sink)
                    .expect("durable append must succeed (unique evidence per iter)");
                black_box(&v);
                start.elapsed().as_nanos() as f64
            }
            Mode::StatusQuoAsyncLog | Mode::StatusQuoDigestLog => {
                // Fire-and-forget: the send result is EXPLICITLY ignored.
                // Unwrapping here would panic the worker on the very
                // disconnected-channel condition this mode deliberately
                // arranges — faking a fail-secure behavior the status quo
                // does not have (see tests/test_status_quo_contrast.rs).
                let tx = sq_tx.expect("thread-local channel built in setup");
                let start = Instant::now();
                let record = match mode {
                    Mode::StatusQuoAsyncLog => build_status_quo_record(black_box(&payload)),
                    _ => build_status_quo_digest_record(black_box(&payload)),
                };
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
    // Cross-thread aggregation happens AFTER the run: reported percentiles
    // are means of per-thread P2 estimates; p99_max_ns is the worst single
    // thread so stragglers cannot hide behind the mean.
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

fn sweep(thread_counts: &[usize], ops: &ModeOps<'_>) {
    // CSV column semantics (documented for the Section 5.1 methodology):
    //   mode — full_pipeline | full_pipeline_durable | verdict_only |
    //          status_quo_async_log | status_quo_digest_log
    //   p50_ns / p95_ns / p99_ns — arithmetic MEAN across threads of the
    //       thread-local P2 quantile ESTIMATES (online, O(1) memory — not
    //       exact order statistics; Jain & Chlamtac 1985). Rotule como
    //       "estimativas P2" em qualquer tabela derivada.
    //   p99_max_ns — worst single thread's p99 estimate (straggler tail).
    //   mean_ns    — arithmetic mean latency over all timed operations.
    println!("trial,mode,threads,payload_bytes,iters_per_thread,wall_ms,throughput_ops_s,p50_ns,p95_ns,p99_ns,p99_max_ns,mean_ns");

    let all_modes = [
        Mode::FullPipeline,
        Mode::FullPipelineDurable,
        Mode::VerdictOnly,
        Mode::StatusQuoAsyncLog,
        Mode::StatusQuoDigestLog,
    ];
    // BTV_SWEEP_MODES="a,b,..." filters the mode list (OS-07): allows
    // chunked collection on hosts that cannot hold a 15-config run in one
    // process; concatenate the chunk CSVs (documented in the fingerprint).
    let modes: Vec<Mode> = match std::env::var("BTV_SWEEP_MODES") {
        Ok(spec) => {
            let wanted: Vec<&str> = spec.split(',').map(str::trim).collect();
            all_modes
                .iter()
                .copied()
                .filter(|m| wanted.contains(&m.as_str()))
                .collect()
        }
        Err(_) => all_modes.to_vec(),
    };
    if modes.is_empty() {
        eprintln!("BTV_SWEEP_MODES matched no modes; running all");
        panic!("empty mode selection");
    }
    let target_wall = target_wall_per_config();

    for &payload_bytes in PAYLOAD_SIZES {
        for &mode in &modes {
            // Per-(payload, mode) calibration with the mode's OWN operation
            // (OS-07): each mode meets the wall target with its own op, so
            // iteration counts are not comparable across modes — the
            // cross-mode comparison uses throughput + within-mode latency.
            // The closure MUTATES the first 8 bytes of the buffer per sample
            // (anti-LLVM lock 1, same as the measured loop): a constant
            // input lets LLVM hoist the entire pipeline out of the
            // calibration loop and "measure" ~74ns of Instant::now() noise.
            let mut cal_iter: u64 = 0;
            let mut cal_payload = vec![0u8; payload_bytes];
            for (i, b) in cal_payload.iter_mut().enumerate() {
                *b = (i % 251) as u8;
            }
            let mut calibrate_op = |buf: &mut [u8]| -> Duration {
                cal_iter += 1;
                buf[0..8].copy_from_slice(&cal_iter.to_le_bytes());
                match mode {
                    Mode::FullPipeline => {
                        let start = Instant::now();
                        let e = (ops.evidence_new)(black_box(&*buf));
                        let c = (ops.compliance_new)();
                        black_box(&(ops.verdict_new)(e, c));
                        start.elapsed()
                    }
                    Mode::VerdictOnly => {
                        let e = (ops.evidence_new)(black_box(&*buf));
                        let c = (ops.compliance_new)();
                        let start = Instant::now();
                        black_box(&(ops.verdict_new)(e, c));
                        start.elapsed()
                    }
                    Mode::FullPipelineDurable => {
                        let sink = ops.durable_sink.expect("durable sink");
                        let start = Instant::now();
                        let e = (ops.evidence_new)(black_box(&*buf));
                        let c = (ops.compliance_new)();
                        let v =
                            issue_verdict(e, c, Decision::Deny, "sweep-bench".to_string(), sink)
                                .expect("durable calibration append");
                        black_box(&v);
                        start.elapsed()
                    }
                    Mode::StatusQuoAsyncLog | Mode::StatusQuoDigestLog => {
                        let start = Instant::now();
                        let record = match mode {
                            Mode::StatusQuoAsyncLog => build_status_quo_record(black_box(&*buf)),
                            _ => build_status_quo_digest_record(black_box(&*buf)),
                        };
                        black_box(&record);
                        start.elapsed()
                    }
                }
            };
            let iters = calibrate_iters(
                target_wall,
                &mut cal_payload,
                calibrate_op,
                mode.min_iters(),
            );

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
                                // Per-thread fire-and-forget channel for the
                                // status-quo arms: receiver dropped at setup
                                // (disconnected fast path, O(1) memory).
                                let sq_tx = if mode.uses_channel() {
                                    let (tx, rx) = std::sync::mpsc::channel::<String>();
                                    drop(rx);
                                    Some(tx)
                                } else {
                                    None
                                };
                                run_thread(payload_bytes, mode, iters, ops, sq_tx.as_ref())
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
        "sweep_concurrent provenance: modes=[full_pipeline, full_pipeline_durable, \
         verdict_only, status_quo_async_log, status_quo_digest_log] payloads={:?} \
         thread_counts={:?} trials={} target_wall_per_config={}s (default {}s; override: \
         BTV_SWEEP_TARGET_WALL_SECS)",
        PAYLOAD_SIZES,
        thread_counts,
        TRIALS,
        target_wall_per_config().as_secs(),
        DEFAULT_TARGET_WALL_PER_CONFIG_SECS
    );

    let authority = ComplianceAuthority::new_from_env();

    // OS-07: ONE shared on-disk sink for the durable arm — WAL and
    // synchronous=FULL are effective on a real file. The file lives outside
    // the repository (temp dir) and is removed at the end of the run.
    let durable_path = std::env::temp_dir().join(format!(
        "btv-sweep-durable-{}.sqlite",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let durable_sink = SqliteLogSink::open(durable_path.to_str().expect("utf-8 temp path"))
        .expect("durable sqlite sink");

    let ops = ModeOps {
        evidence_new: &|ctx: &[u8]| EvidenceToken::new(ctx),
        compliance_new: &|| {
            authority
                .issue_token("EU-GDPR", "sweep-policy-1", 720)
                .expect("EU-GDPR is allowlisted above")
        },
        verdict_new: &|e, c| {
            Verdict::new(e, c, Decision::Deny, "sweep-bench".to_string())
                .expect("recognized-authority tokens verify by construction")
        },
        durable_sink: Some(&durable_sink),
    };

    sweep(&thread_counts, &ops);

    drop(durable_sink);
    let _ = std::fs::remove_file(&durable_path);
}
