# Release Notes — artifact-v2

**Date:** 2026-08-27
**Branch:** `artifact-v2` (local; based on `main` @ `6f3cf39`)

## Summary

This artifact implements, tests, and documents the BTV (Bill of Materials for Trustworthy Verdicts) framework for the IEEE Computer major-revision of *"Silent Decisions Are Type Errors"*. All 8 tests from the test plan were executed, with full passing status on x86-64 and ARM64 (via QEMU emulation where hardware access was unavailable).

## Breaking change: Python ≥ 3.8 required (pyo3 0.22 → 0.29)

The security upgrade of `pyo3` 0.22 → 0.29 (Round 2 of the audit, clearing
RUSTSEC-2025-0020 and RUSTSEC-2026-0177) raised the minimum supported
Python from 3.7 to **3.8**: the binding now ships an `abi3-py38` wheel
(`btv_python-0.2.0-cp38-abi3-*.whl`). Python 3.7 reached end of life in
June 2023 and is no longer supported by this library. See
`btv-python/README.md` ("Python compatibility") for the policy statement.

## Test execution results

| # | Test | Outcome | Evidence |
|---|---|---|---|
| 1 | trybuild compile-fail (8 UI tests) | ✅ PASS (8/8) | `btv-core/tests/trybuild.rs` + `btv-core/tests/ui/*.stderr` |
| 2 | TCB / unsafe audit | ✅ PASS | `reports/tcb_summary.md` |
| 3 | PyO3 binding (13 unit tests) | ✅ PASS (13/13) | `tests/pyo3/test_binding.py` |
| 4 | Baseline comparative | ✅ PASS | `reports/benchmark_baseline.md`, `reports/benchmark_baseline.csv` |
| 5 | Fail-secure partition | ✅ PASS (5/5) | `reports/failure_behavior.md` |
| 6 | Concurrent load (p50/p95/p99) | ✅ PASS | `reports/load_stats.csv` |
| 7 | Multi-hardware (ARM64) | ✅ PASS (QEMU) | `reports/hardware_comparison.md`, `reports/load_stats_arm64_qemu.csv` |
| 8 | TCO reproducibility | ✅ PASS (N* = 500,000) | `reports/tco_summary.md`, `data/n_star_by_regime.csv` |

## Architecture coverage

| Architecture | Mode | Status |
|---|---|---|
| x86-64 (`x86_64-unknown-linux-gnu`) | Native | ✅ Full test suite + benchmarks |
| ARM64 (`aarch64-unknown-linux-gnu`) | QEMU user-static emulation | ✅ Full test suite (no benchmarks — QEMU numbers not representative) |
| ARM64 native (Graviton, Pi 4, Apple Silicon) | Not executed | ⏸ Out of scope (no hardware access) — CI config provided in `.github/workflows/ci.yml` |

## Key numbers

### Benchmarks (x86-64 native, 2 cores)

| Implementation | p50 | p95 | p99 | throughput |
|---|---:|---:|---:|---:|
| BTV-Rust native (in-memory) | 1.10 μs | 1.10 μs | 1.10 μs | 910k ops/s |
| BTV-PyO3 | 2.23 μs | 3.40 μs | 5.74 μs | 419k ops/s |
| BTV-Rust + SQLite WAL+FULL | 10.31 μs | 10.31 μs | 10.31 μs | 97k ops/s |
| OpenTelemetry pós-hoc (NullExporter) | 17.57 μs | 23.76 μs | 48.96 μs | 48k ops/s |
| SQLite ACID bare (BEGIN..COMMIT) | 8.58 μs | 13.08 μs | 21.10 μs | 102k ops/s |

### Concurrent load (rayon, 2 threads × 1000 ops)

| Arch | p50 | p95 | p99 | throughput |
|---|---:|---:|---:|---:|
| x86-64 native | 14.48 μs | 19.71 μs | 28.77 μs | 63 562 ops/s |
| ARM64 QEMU | 220.02 μs | 431.23 μs | 611.78 μs | 2 934 ops/s |

### TCO crossover

| Regime | ρ ($/decision) | N* (no credit) | N* (with credit δ) |
|---|---:|---:|---:|
| GDPR | $0.01 | **500,000** ✓ | 555,556 (δ=10%) |
| EU AI Act | $0.02 | 250,000 | **500,000** (δ=50%) |
| SEC | $1.04 | 4,808 | 4,808 (no credit) |
| BR-LGPD | $0.005 | 1,000,000 | 1,000,000 (no credit) |

## What was validated

1. **Compile-time enforcement of linear ownership** for `Verdict`, `EvidenceToken`, `ComplianceToken`, `OperatorToken`, `EscalatedVerdict` — all 8 attack classes (struct literal, token reuse, external consume, silent drop, escalated struct literal, escalated token reuse, escalated consume external, escalated operator token drop) fail to compile with the expected `rustc` error.
2. **`#![forbid(unsafe_code)]`** is enforced at the crate level; no `unsafe` may be introduced in `btv-core` source.
3. **PyO3 binding** rejects all attempts to pass pre-computed hashes (`TypeError` for `str`/`dict`/`None`); only `bytes` is accepted.
4. **Fail-secure behavior** under log partition: `issue_verdict` returns `Err(BtvError::LogUnavailable)` without constructing a `Verdict`; `EvidenceToken` is consumed via `mem::forget` to prevent retry.
5. **Concurrent correctness**: 2-thread rayon workload of 2000 ops completes with all `Verdict`s integrity-valid and 0 dropped.
6. **ARM64 compatibility**: full test suite passes under QEMU emulation of `aarch64-unknown-linux-gnu`.
7. **TCO determinism**: `scripts/compute_crossover.py` regenerates N* = 500,000 (GDPR) from versioned `data/enforcement_cases.csv` + `data/policy_parameters.yaml`.

## What was NOT validated (and is documented as such)

1. **Native ARM64 performance numbers** — only QEMU emulation was available. The `reports/hardware_comparison.md` documents this and the CI config provides for native ARM64 execution when hardware is available.
2. **End-to-end non-repudiation across processes** — the PyO3 binding validates the polyglot boundary but does not include a deployed HTTP server. `scripts/locust_btv.py` is provided for future load testing.
3. **Mechanized proof of the Constitutional Enclosure Theorem** — out of scope per the test plan (no Coq/Lean). The `trybuild` suite provides compile-time enforcement at the API surface, which is what the manuscript claims.
4. **Soundness of `unsafe` in transitive dependencies** — `cargo geiger` (or its replacement) inventories; `cargo audit` confirms zero known advisories. Full soundness audit of `blake3`, `rusqlite`, `ring`, `rustls`, etc. is out of scope.
5. **Geographic replication / Byzantine fault tolerance of `LogSink`** — the `LogSink` trait is pluggable but the artifact only ships `InMemoryLogSink` and `SqliteLogSink`.

## Push to remote

The branch `artifact-v2` is local only. To push to the remote (`danzeroum/silent-decisions-proof`), the repository owner should:

```bash
cd /path/to/silent-decisions-proof  # after extracting the ZIP
git checkout artifact-v2
git push origin artifact-v2
```

A `git bundle` is included in the ZIP (`artifact-v2.bundle`) that can be used to apply the branch:

```bash
cd /path/to/silent-decisions-proof
git fetch /path/to/artifact-v2.bundle artifact-v2:artifact-v2
git checkout artifact-v2
```

## CI

The `.github/workflows/ci.yml` file defines 4 jobs:

1. `fmt-clippy-audit` — format check, clippy pedantic, cargo audit
2. `test-x86-64` — Rust tests + PyO3 tests + benchmarks
3. `test-arm64-qemu` — cross-compile + QEMU test execution
4. `tco-reproducibility` — recompute N* and verify GDPR = 500,000

These will run on push to `artifact-v2` or `main`, and on PRs against either branch.
