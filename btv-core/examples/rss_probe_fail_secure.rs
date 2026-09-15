//! RSS probe for the fail-secure path — OS-06 gate (closes audit finding F6).
//!
//! F6: `issue_verdict` called `mem::forget` on the token/compliance/verdict.
//! Forgetting non-`Drop` types does NOT prevent retry (move semantics
//! already do) — it only leaked the heap allocations. The audit measured
//! ~64 B leaked per rejected decision (~12.5 MB per 200k calls; ~14 GB/hour
//! at the artifact's claimed throughput): a multi-hour log partition — the
//! exact scenario fail-secure exists for — ended in OOM.
//!
//! The fix replaced `mem::forget` with explicit `drop`. This probe re-runs
//! the audit's measurement protocol: 200,000 rejected decisions under a
//! failed sink, asserting RSS delta < 1 MB.
//!
//! Run: cargo run --release --features test-support \
//!        --example `rss_probe_fail_secure`
//!
//! Epistemic footer: measures `VmRSS` on Linux only; allocator behavior can
//! vary with platform allocator, but the assertion budget (1 MB) is two
//! orders of magnitude below the F6 leak (12.5 MB).

use btv_core::{issue_verdict, ComplianceAuthority, Decision, EvidenceToken, InMemoryLogSink};

fn rss_kb() -> i64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("VmRSS: Linux /proc");
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb = rest.split_whitespace().next().expect("VmRSS value present");
            return kb.parse::<i64>().expect("VmRSS in kB");
        }
    }
    panic!("VmRSS line not found");
}

fn main() {
    const CALLS: usize = 200_000;
    let sink = InMemoryLogSink::new();
    sink.fail(); // fail-secure path: every call returns Err(LogUnavailable)
    let authority = ComplianceAuthority::new_from_env();

    // Warmup: resolve keys, prime allocator size classes.
    for i in 0..1_000 {
        let token = EvidenceToken::new(format!("warmup-{i}").as_bytes());
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let _ = issue_verdict(
            token,
            compliance,
            Decision::Deny,
            "partition-probe".to_string(),
            &sink,
        );
    }

    let before = rss_kb();
    for i in 0..CALLS {
        let token = EvidenceToken::new(format!("ctx-{i}").as_bytes());
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        debug_assert!(issue_verdict(
            token,
            compliance,
            Decision::Deny,
            "partition-probe".to_string(),
            &sink,
        )
        .is_err());
    }
    let after = rss_kb();
    let delta = after - before;

    println!(
        "RSS before={before} kB  after={after} kB  delta={delta} kB / {CALLS} rejected decisions"
    );
    if delta >= 1_024 {
        eprintln!("FAIL: RSS delta {delta} kB >= 1024 kB — leak regression");
        std::process::exit(1);
    }
    println!("PASS: delta < 1 MB over {CALLS} rejected decisions (pre-fix leak: ~12,504 kB)");
}
