use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, BatchSize};
use silent_decisions_proof::{ComplianceToken, Decision, EvidenceToken, Verdict};

fn bench_verdict_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("Verdict::new");
    let payloads: &[(&str, &[u8])] = &[
        ("64B",  &[b'x'; 64]),
        ("512B", &[b'x'; 512]),
        ("4KB",  &[b'x'; 4096]),
    ];

    for (label, payload) in payloads {
        group.bench_with_input(BenchmarkId::new("context_size", label), payload, |b, ctx| {
            let jur  = "BR-LGPD".to_string();
            let pol  = "1.0.0".to_string();
            let expl = "Credit score below threshold.".to_string();
            let mut seed = 0u8;

            b.iter_batched(
                || {
                    seed = seed.wrapping_add(1); // impede LLVM de cachear o hash
                    let mut ctx_owned = ctx.to_vec();
                    ctx_owned[0] = seed;
                    (ctx_owned, jur.clone(), pol.clone(), expl.clone())
                },
                |(ctx_owned, jur_owned, pol_owned, expl_owned)| {
                    let token = EvidenceToken::new(black_box(&ctx_owned));
                    let compliance = ComplianceToken::new(jur_owned, pol_owned, 720);
                    Verdict::new(token, compliance, Decision::Deny, expl_owned)
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_verify_integrity(c: &mut Criterion) {
    let token = EvidenceToken::new(b"benchmark-context-integrity-check");
    let compliance = ComplianceToken::new("BR-LGPD", "1.0.0", 720);
    let verdict = Verdict::new(
        token, compliance, Decision::Deny,
        "Credit score below threshold.".to_string(),
    );
    c.bench_function("Verdict::verify_integrity", |b| {
        b.iter(|| black_box(verdict.verify_integrity()));
    });
}

criterion_group!(benches, bench_verdict_new, bench_verify_integrity);
criterion_main!(benches);
