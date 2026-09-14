# btv-python — PyO3 gateway for btv-core

Python bindings for the [BTV framework](../btv-core) core. This crate is
the **only authorized path** by which a Python-side orchestrator (e.g., a
LangChain/FastAPI agent pipeline) can cause a BTV `Verdict` to be
materialized. The Rust type-level guarantee of Theorem 4.1 applies strictly
to code inside this crate and `btv-core`; it does **not** extend to the
Python caller, which is treated as untrusted.

This README is the citable prose companion of the threat-model comment at
the top of [`src/lib.rs`](src/lib.rs) — cross-reference for **Section 4.4
(Adversarial Boundaries and Polyglot Environments)** of the accompanying
paper.

## Trust boundary (L4 — polyglot boundary)

The Python orchestrator **cannot**:

1. **Forge an evidence hash.** `issue_verdict()` only accepts `bytes` as
   `raw_context`; the BLAKE3 hash is computed inside Rust. There is no API
   that accepts a pre-computed hash (threat class F).
2. **Bypass the compliance authority.** `ComplianceToken` has no public
   constructor in `btv-core`; tokens are issued only through
   `ComplianceAuthority::issue_token()`, which validates the jurisdiction
   allowlist before a `Verdict` can exist.
3. **Reuse an `EvidenceToken`.** The token is consumed inside Rust before
   Python ever sees a result; linear consumption (`V ⊸ (E ⊗ C)`) is
   enforced at the Rust type level.
4. **Silently drop a verdict in flight.** `SealedVerdict` supports
   `with`-style context management (`__enter__`/`__exit__`) for
   deterministic teardown; after `__exit__`, every accessor raises
   `RuntimeError("verdict already consumed")`.
5. **Inspect or mutate verdict internals.** `SealedVerdict` exposes only
   read-only getters (`hash_hex`, `decision`, `jurisdiction`,
   `policy_version`, `appeal_deadline_hours`, `explanation`) plus
   `verify_integrity()`; the underlying `EvidenceToken`, `consume()`, and
   struct fields are never exposed.

The Python orchestrator **can**: pass arbitrary `bytes`, read the sealed
result, and call `verify_integrity()` (HMAC re-check) to detect tampering
of the serialized form.

The downstream effector (the credit-decision API, the DB write that
actually blocks/approves) **must** be configured to require a valid sealed
`SealedVerdict` from this gateway and reject any request that attempts to
bypass it. Enforcing that requirement is an infrastructure obligation
outside the scope of this crate.

## Maturity state (honest inventory)

- **Compiles and is CI-tested**: `maturin build --release --features
  test-support` + `pytest tests/pyo3/test_binding.py` run on every push
  (see `.github/workflows/ci.yml`). `#![forbid(unsafe_code)]` is enforced
  in this crate.
- **abi3-py38** stable-ABI wheel: one build covers CPython 3.8+ (see
  [Python compatibility](#python-compatibility) below for the breaking
  change from 3.7).
- **Not published to PyPI**; consume via `maturin develop` / `maturin
  build` from this repository.
- **Out of scope** (documented, deliberate): end-to-end non-repudiation
  across process boundaries; protection against a compromised Python
  interpreter (a malicious extension could call `btv-core` directly via
  FFI); durability of the `LogSink` beyond what the configured backend
  provides. The signing key is proof-of-concept unless `BTV_AUTHORITY_KEY`
  is injected from an HSM/KMS in production.

## Python compatibility

As of version 0.2.0, this binding requires **Python ≥ 3.8** (wheel
`abi3-py38`), due to the pyo3 0.22 → 0.29 upgrade (Round 2 of the audit,
clearing RUSTSEC-2025-0020 and RUSTSEC-2026-0177). Python 3.7 reached
end of life in June 2023 and is no longer supported by this library.

## Usage

```python
import btv_python

cfg = btv_python.LogConfig.in_memory()          # or LogConfig.sqlite(path)
with btv_python.issue_verdict(
    raw_context=b"subject:alice|score:0.42|threshold:0.50",
    decision="deny",
    jurisdiction="BR-LGPD",
    policy_version="1.0.0",
    explanation="Credit score below threshold.",
    contestability_hours=720,
    log_config=cfg,
) as verdict:
    assert verdict.verify_integrity()
    print(verdict.hash_hex, verdict.decision, verdict.appeal_deadline_hours)
```

Invalid inputs raise at the boundary: `TypeError` for non-`bytes`
contexts, `ValueError` for unknown decisions/jurisdictions, and
`btv_python.BTVError` when the log sink is unavailable (fail-secure: no
verdict is emitted).
