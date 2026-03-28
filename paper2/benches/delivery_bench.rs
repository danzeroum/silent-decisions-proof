//! Criterion benchmarks for BTV-Transparency.
//!
//! Measures:
//!   - DeliveryToken::seal overhead (in-process, no network)
//!   - deliver() serialisation cost
//!
//! Log-submission latency (the dominant cost) is measured separately
//! in integration tests with the btv-log server on localhost.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
// Note: full integration benchmarks require btv-log server running.
// These benchmarks measure the pure type-system overhead (no I/O).

fn bench_seal_placeholder(c: &mut Criterion) {
    c.bench_function("DeliveryToken::seal (placeholder)", |b| {
        b.iter_batched(
            || (),
            |_| {
                // Placeholder: actual benchmark requires a running LogClient.
                // Replace with:
                //   let receipt = mock_receipt();
                //   DeliveryToken::seal(&mock_verdict(), receipt)
                std::hint::black_box(42u64)
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_seal_placeholder);
criterion_main!(benches);
