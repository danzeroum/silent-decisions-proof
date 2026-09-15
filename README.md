# Silent Decisions Are Type Errors — Artifact v2

Reference implementation and test artifact for the IEEE Computer major-revision of *"Silent Decisions Are Type Errors: Enforcing AI Accountability via Linear Resource Types"*.

**Branch:** `artifact-v2` (local; based on `main` @ `6f3cf39`)
**Date:** 2026-08-27

## What's in this artifact

```
artifact-v2/
├── btv-core/                # Rust crate — BTV framework (#![forbid(unsafe_code)])
│   ├── src/lib.rs           # Verdict, EvidenceToken, ComplianceToken, LogSink trait
│   ├── benches/verdict_construction.rs   # Criterion benchmarks (3)
│   └── tests/
│       ├── trybuild.rs      # Test 1: 8 compile-fail UI tests
│       ├── ui/              # .rs + .stderr for each compile-fail case
│       ├── test_partition.rs # Test 5: 5 fail-secure partition tests
│       └── test_load.rs     # Test 6: concurrent load + p50/p95/p99 stats
├── btv-python/              # PyO3 binding for Python orchestrators
│   ├── src/lib.rs           # SealedVerdict, LogConfig, TestingLogConfig
│   └── Cargo.toml           # (cdylib, abi3-py37)
├── tests/
│   └── pyo3/test_binding.py # Test 3: 13 PyO3 unit tests (context manager, forged-hash, etc.)
├── scripts/
│   ├── benchmark_baseline.py        # Test 4: BTV vs OpenTelemetry vs SQLite ACID
│   ├── cargo_geiger_replacement.py  # Test 2: unsafe inventory (cargo-geiger equivalent)
│   ├── compute_crossover.py         # Test 8: TCO/N* reproducibility
│   └── locust_btv.py                # Future use: HTTP load test (not run by pipeline)
├── data/
│   ├── enforcement_cases.csv        # 20 regulatory cases (T1/T2/T3 + fine)
│   ├── policy_parameters.yaml       # ρ, C_fixed, compliance credit deltas
│   ├── n_star_by_regime.csv         # Computed N* per regime (auto-generated)
│   └── tco_plot_data.csv            # Sensitivity grid (auto-generated)
├── reports/
│   ├── tcb_summary.md               # Test 2: TCB + unsafe audit + cargo audit + clippy
│   ├── cargo_audit_raw.txt          # Test 2: cargo audit raw output
│   ├── cargo_geiger_unsafe_inventory.csv  # Test 2: 113 deps, 63 with unsafe
│   ├── cargo_geiger_unsafe_inventory.md
│   ├── clippy_pedantic_raw.txt      # Test 2: clippy output (49 warnings, 0 errors)
│   ├── benchmark_baseline.md        # Test 4: comparative table
│   ├── benchmark_baseline.csv
│   ├── failure_behavior.md          # Test 5: fail-secure analysis
│   ├── load_stats.csv               # Test 6: x86-64 native p50/p95/p99
│   ├── load_stats_arm64_qemu.csv   # Test 7: ARM64 QEMU p50/p95/p99
│   ├── hardware_comparison.md      # Test 7: x86-64 vs ARM64
│   └── tco_summary.md               # Test 8: N* derivation summary
├── paper1/                  # LaTeX manuscript (under revision) + original src/lib.rs
├── paper2/                  # Paper 2 — durable log (related)
├── paper3/                  # Paper 3 — ZK circuits (related)
├── paper4/                  # Paper 4 — TCO/economics (source of N*)
├── paper5/                  # Paper 5 — constitutional mapping
├── paper6/                  # Paper 6 — amendments
├── .github/workflows/ci.yml # CI: fmt, clippy, audit, tests (x86 + ARM64 QEMU), TCO
├── appendix_b_pgfplots.tex  # LaTeX PGFPlots snippet for Appendix B (TCO)
├── appendix_benchmarks.pgfplots.tex  # LaTeX PGFPlots for benchmark figures
├── README.md                # This file
└── RELEASE_NOTES.md         # What was tested and validated
```

## Quick start

### Prerequisites

- Rust stable (1.98+) with `rustfmt`, `clippy`, target `aarch64-unknown-linux-gnu` (optional)
- Python 3.10+
- `maturin`, `pytest`, `opentelemetry-sdk`, `pyyaml`, `numpy`

### Run all tests (x86-64)

**Scope note (G4, COMSI-2026-04-0112 Round 2):** `-D clippy::pedantic` is a
`btv-core`-only gate, both here and in CI — it is the canonical
implementation the manuscript's claims trace to. The README previously
implied this gate held workspace-wide (Test 2's "0 clippy errors" row);
it does not, and `cargo clippy --workspace --all-targets -- -D
clippy::pedantic` was never run as a whole before this round. Doing so
surfaces roughly 40 further findings this repository does not hold to
that bar: 9 in `paper1`'s demo examples (uninlined format args, lossy
numeric casts — style, not correctness, in code that exists to be read
by a human alongside the manuscript), 20 in `paper2`/`btv-transparency`
(a separate, unrelated manuscript's crate sharing this workspace — see
`docs/EVIDENCE-MANIFEST.md`), and 14+ in `btv-python` (missing
`# Errors` doc sections and `#[must_use]` on PyO3 binding methods,
mostly documentation completeness on thin FFI glue rather than defects
in the enforcement logic). `paper1/src/main.rs`'s two findings that
*were* previously reported (missing backticks in its doc comment) are
fixed as of this round; the rest are disclosed, not fixed, because nothing
in that ~40 traces to the accountability guarantees this artifact makes.

```bash
# 1. Format + lint + audit
cd btv-core
cargo fmt --check
cargo clippy --all-targets --features test-support -- -D clippy::pedantic
cargo audit --deny warnings

# 2. Unit + integration tests
cargo test --features test-support --lib
cargo test --features test-support --test trybuild
cargo test --features test-support --test test_partition
cargo test --features test-support --test test_load -- --nocapture
cargo test --features test-support --test test_append_only
cargo test --features test-support --test test_proof_clauses
cargo test --features test-support --test test_status_quo_contrast

# 3. Benchmarks
cargo bench --features test-support --bench verdict_construction

# 4. PyO3 binding
cd ../btv-python
maturin build --release --features test-support
pip install --force-reinstall target/wheels/*.whl
cd ..
pytest tests/pyo3/test_binding.py -v

# 5. TCO reproducibility
python3 scripts/compute_crossover.py
```

### Run tests on ARM64 (via QEMU)

Requires `qemu-user-static`, `gcc-aarch64-linux-gnu`, `libc6-dev-arm64-cross`, `libgcc-14-dev-arm64-cross`.

```bash
rustup target add aarch64-unknown-linux-gnu

# Configure cargo:
cat > btv-core/.cargo/config.toml << 'EOF'
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
runner = "qemu-aarch64-static -L /usr/aarch64-linux-gnu"
EOF

cd btv-core
cargo test --target aarch64-unknown-linux-gnu --features test-support --lib
cargo test --target aarch64-unknown-linux-gnu --features test-support --test test_partition
BTV_UNDER_QEMU=1 cargo test --target aarch64-unknown-linux-gnu --features test-support --test test_load -- --nocapture
```

## Test results summary

| # | Test | Status (x86-64) | Status (ARM64 QEMU) |
|---|---|---|---|
| 1 | trybuild compile-fail (8 cases) | ✅ 8/8 | ⚠️ skipped (see `reports/hardware_comparison.md`) |
| 2 | TCB / unsafe audit | ✅ `#![forbid(unsafe_code)]`, 0 vulns, 0 clippy errors (`btv-core`, `-D clippy::pedantic` — the gate CI runs; `btv-python`'s PyO3 binding layer is not held to the same pedantic gate, see note below) | n/a |
| 3 | PyO3 binding (13 tests) | ✅ 13/13 | n/a (PyO3 is x86 only) |
| 4 | Baseline comparative (3 impls) | ✅ see `reports/benchmark_baseline.md` | n/a |
| 5 | Fail-secure partition (5 tests) | ✅ 5/5 | ✅ 5/5 |
| 6 | Concurrent load (p50/p95/p99) | ✅ p99=28.77μs, 63k ops/s | ✅ p99=611.78μs, 2.9k ops/s (QEMU) |
| 7 | Multi-hardware | ✅ native | ✅ QEMU emulation (see note) |
| 8 | TCO reproducibility | ✅ ρ/N* derived from primitives; consistency gate (see `reports/tco_summary.md`) | n/a |

## Trusted Computing Base (TCB)

See `reports/tcb_summary.md` for the full TCB declaration. Summary:

- `rustc` per `rust-toolchain.toml` (single source, OS-09), `std`
- `blake3`, `hmac`, `sha2`, `subtle` (cryptographic primitives)
- `rusqlite` + `libsqlite3-sys` (persistence backend)
- `BTV_HMAC_KEY` / `BTV_AUTHORITY_KEY` (HSM/KMS in production)
- `LogSink` implementation (durability depends on backend)

The `btv-core` crate is `#![forbid(unsafe_code)]`. Transitive dependencies contain 5,146 `unsafe` occurrences across 63 crates (see `reports/cargo_geiger_unsafe_inventory.csv`).

## Epistemic footers

Every test report includes an epistemic footer that documents what is and is NOT being validated:

> *Este teste valida [X] sob hipóteses explícitas de [Y]. Ele NÃO garante [Z] (fora do escopo desta implementação).*

See individual reports for the specific footers.

## License

MIT (see `btv-core/Cargo.toml`).

## Contact

For questions about this artifact, contact the corresponding author of the IEEE Computer submission.

---

**Artifact tag for the Computer resubmission:** `comsi-2026-04-0112-r1`
(every number cited in the manuscript traces to a committed CSV/raw report
by file, line, and commit hash — see `docs/RESPONSE-LETTER.md`). The tag
itself is stale as of Round 2 (G1: it points at a commit ~20 files behind
`main`, predating this round's fixes) and will be moved to the final
submission commit once every other Round 2 item closes — see the response
letter's G1 entry.
