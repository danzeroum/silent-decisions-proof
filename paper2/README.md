# BTV-Transparency — Paper 2

**Title:** *Type-Safe Verifiability: Bridging Linear Types and Transparency Logs for AI Accountability*

**Target venue:** ACM CCS / USENIX Security

## Structure

```
paper2/
├── main.tex                    # Master LaTeX file
├── abstract.tex
├── section1_introduction.tex
├── section2_related_work.tex
├── section3_formal_model.tex   # Definitions 3.1–3.2, Axiom 3.3, Theorem 3.4
├── section4_implementation.tex
├── section5_benchmarks.tex     # Table populated after btv-log benchmarks
├── section6_discussion.tex
├── section7_conclusion.tex
├── refs.bib
├── Cargo.toml                  # btv-transparency crate
├── src/
│   ├── lib.rs                  # InclusionReceipt, DeliveryToken, LogClient
│   └── log_server.rs           # btv-log reference implementation
├── tests/
│   └── sui/                    # Compile-fail tests (Protection A/B/C + Enclosure)
│       ├── receipt_struct_literal.rs
│       ├── receipt_consume_external.rs
│       ├── receipt_drop_silent.rs
│       └── delivery_without_receipt.rs
└── benches/
    └── delivery_bench.rs
```

## Type Law

```
DeliveryToken ⊸ (Verdict ⊗ InclusionReceipt)
```

A `DeliveryToken` — the only type that authorises releasing a decision to an
end-user — can only be constructed by linearly consuming a `Verdict` (Paper 1)
and an `InclusionReceipt` (a cryptographic proof that the decision was logged
by an independent authority before delivery).

## Central Theorem

**Persistence Enclosure Theorem:** In any Safe Rust program using
`btv-transparency`, the set of Ephemeral Verdicts — decisions delivered
without a verifiable persistence record — is empty by construction.

## Relationship to Paper 1

| Layer | Paper | Guarantee | Enforced by |
|---|---|---|---|
| Memory | Paper 1 (BTV) | No verdict without evidence | Compiler (type system) |
| Persistence | Paper 2 (BTV-Transparency) | No delivery without log proof | Compiler (type system) |

## Status

- [x] Section 1 — Introduction (final)
- [x] Section 3 — Formal model (Definitions, Axiom, Theorem sketch)
- [x] Section 2, 4, 5, 6, 7 — drafted, benchmarks TBD
- [x] Rust crate skeleton with compile-fail test suite
- [ ] `LogClient::submit_and_await` — HTTP integration pending `btv-log` server
- [ ] Benchmarks — pending `btv-log` running on localhost
- [ ] Full case-by-case proof (Section 4) — pending compile-fail test execution

## Build

```bash
# Compile the crate (type-checks the linear invariants)
cargo build --manifest-path paper2/Cargo.toml

# Run compile-fail tests (verifies the enclosure perimeter)
cargo test --manifest-path paper2/Cargo.toml

# Run benchmarks (placeholder — replace with btv-log integration)
cargo bench --manifest-path paper2/Cargo.toml
```

## Next Steps

1. Implement `LogClient::submit_and_await` (HTTP to `btv-log` server)
2. Implement `btv-log` HTTP endpoints (`POST /submit`, `GET /proof/:index`)
3. Run benchmarks and populate Table 1 in `section5_benchmarks.tex`
4. Expand Section 4 proof to full case-by-case analysis (mirroring Paper 1 §4.4)
