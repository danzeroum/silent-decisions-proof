#![allow(clippy::pedantic)]

use btv_core::{
    issue_verdict, ComplianceAuthority, Decision, EvidenceToken, InMemoryLogSink,
    SqliteLogSink, Verdict,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_verdict_construction(c: &mut Criterion) {
    c.bench_function("verdict_construction_in_memory", |b| {
        b.iter(|| {
            let token = EvidenceToken::new(black_box(b"score:0.42|threshold:0.50"));
            let authority = ComplianceAuthority::new_for_test();
            let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
            let v = Verdict::new(
                token, compliance, Decision::Deny,
                "Below threshold".to_string(),
            );
            black_box(v);
        })
    });
}

fn bench_issue_verdict_inmemory(c: &mut Criterion) {
    c.bench_function("issue_verdict_in_memory_sink", |b| {
        b.iter_with_setup(
            || {
                let sink = InMemoryLogSink::new();
                let auth = ComplianceAuthority::new_for_test();
                let compliance = auth.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
                (sink, compliance)
            },
            |(sink, compliance)| {
                let token = EvidenceToken::new(b"ctx");
                issue_verdict(
                    token, compliance, Decision::Allow,
                    "ok".to_string(), &sink,
                ).unwrap();
            },
        )
    });
}

fn bench_issue_verdict_sqlite(c: &mut Criterion) {
    c.bench_function("issue_verdict_sqlite_wal_full", |b| {
        b.iter_with_setup(
            || {
                let sink = SqliteLogSink::open_in_memory().unwrap();
                let auth = ComplianceAuthority::new_for_test();
                let compliance = auth.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
                (sink, compliance)
            },
            |(sink, compliance)| {
                let token = EvidenceToken::new(b"ctx");
                issue_verdict(
                    token, compliance, Decision::Allow,
                    "ok".to_string(), &sink,
                ).unwrap();
            },
        )
    });
}

criterion_group!(
    benches,
    bench_verdict_construction,
    bench_issue_verdict_inmemory,
    bench_issue_verdict_sqlite,
);
criterion_main!(benches);
