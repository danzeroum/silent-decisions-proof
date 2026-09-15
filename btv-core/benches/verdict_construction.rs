#![allow(clippy::pedantic)]

use btv_core::{
    issue_verdict, ComplianceAuthority, Decision, EvidenceToken, InMemoryLogSink, SqliteLogSink,
    Verdict,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_verdict_construction(c: &mut Criterion) {
    // Three context sizes characterize the BLAKE3 throughput curve (the
    // payload-scaling story of Section 5). Contexts are filled with
    // distinct byte patterns; construction includes authority-signed token
    // issuance + signature verification (OS-02), which is the real cost.
    let make_context = |len: usize| -> Vec<u8> { (0..len).map(|i| (i % 251) as u8).collect() };
    for (label, len) in [("64B", 64), ("512B", 512), ("4KiB", 4096)] {
        let context = make_context(len);
        c.bench_function(&format!("verdict_construction_{label}"), |b| {
            b.iter(|| {
                let token = EvidenceToken::new(black_box(context.as_slice()));
                let authority = ComplianceAuthority::new_for_test();
                let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
                let v = Verdict::new(
                    token,
                    compliance,
                    Decision::Deny,
                    "Below threshold".to_string(),
                )
                .expect("new_for_test authority holds the recognized key");
                black_box(v);
            })
        });
    }
}

fn bench_issue_verdict_inmemory(c: &mut Criterion) {
    c.bench_function("issue_verdict_in_memory_sink", |b| {
        let sink = InMemoryLogSink::new();
        let auth = ComplianceAuthority::new_for_test();
        // Unique context per decision: the log is append-only (OS-03), so
        // replays of an identical verdict are idempotent no-ops — the bench
        // measures the real pipeline on unique evidence, like production.
        let counter = std::sync::atomic::AtomicU64::new(0);
        b.iter(|| {
            let n = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let token = EvidenceToken::new(format!("ctx-{n}").as_bytes());
            let compliance = auth.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
            issue_verdict(token, compliance, Decision::Allow, "ok".to_string(), &sink).unwrap();
        })
    });
}

fn bench_issue_verdict_sqlite(c: &mut Criterion) {
    // OS-07 (closes F7): DURABLE means an actual file. The pre-audit bench
    // used `SqliteLogSink::open_in_memory()`, where SQLite silently ignores
    // `journal_mode=WAL` and `synchronous=FULL` is meaningless without a
    // file — the reported "persistencia duravel ACID" number was a RAM
    // insert. This benchmark opens the sink on a real file in the temp dir
    // (WAL + FULL effective); the number is expected to be orders of
    // magnitude slower and that is the honest result.
    c.bench_function("issue_verdict_sqlite_wal_full", |b| {
        let path = std::env::temp_dir().join(format!(
            "btv-bench-durable-{}.sqlite",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        let path_str = path.to_str().expect("utf-8 temp path").to_string();
        let auth = ComplianceAuthority::new_for_test();
        let counter = std::sync::atomic::AtomicU64::new(0);
        b.iter_with_setup(
            || SqliteLogSink::open(&path_str).expect("durable sqlite sink"),
            |sink| {
                let n = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let token = EvidenceToken::new(format!("ctx-{n}").as_bytes());
                let compliance = auth.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
                issue_verdict(token, compliance, Decision::Allow, "ok".to_string(), &sink).unwrap();
            },
        );
        let _ = std::fs::remove_file(&path);
    });
}

criterion_group!(
    benches,
    bench_verdict_construction,
    bench_issue_verdict_inmemory,
    bench_issue_verdict_sqlite,
);
criterion_main!(benches);
