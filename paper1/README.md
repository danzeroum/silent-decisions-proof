# Silent Decisions Are Type Errors

**Proof repository for the Constitutional Enclosure Theorem**  
*Companion to: "Silent Decisions Are Type Errors: Enforcing AI Accountability via Linear Resource Types"*  
*IEEE Computer — submitted April 2026*

---

## The Theorem

> **Constitutional Enclosure Theorem:** In any system satisfying the BTV type invariant,
> the set of materialized silent decisions is empty.

A **silent decision** is an AI Verdict that carries legal consequences but has no
non-repudiable evidence chain. This repository proves, via the Rust type system,
that silent decisions are *type errors* — not runtime omissions, not process failures.

---

## The Core Law

```
V ⊸ (E ⊗ C)
```

A Verdict (`V`) can only be produced by **linearly consuming**:
- `E` — an `EvidenceToken` (forensic BLAKE3 hash of the decision context)  
- `C` — a `ComplianceToken` (contestability deadline, jurisdiction, policy version)

This is encoded in the Rust type system via:
1. **Private struct fields** — `Verdict { ... }` literal syntax is blocked outside the module
2. **Linear ownership** — `EvidenceToken` is not `Clone`, not `Copy`, marked `#[must_use]`
3. **No public constructors** for `Blake3Hash` or `HmacSha256`
4. **Single construction path** — `Verdict::new(evidence, compliance, ...)` consumes both tokens

---

## Difference from `assert(evidence != null)`

| Property | `assert(evidence != null)` | BTV Linear Types |
|----------|---------------------------|-----------------|
| When checked | Runtime | **Compile time** |
| Can be disabled | Yes (`-DNDEBUG`, prod flags) | **No** |
| Can be forgotten | Yes (developer omission) | **No** (compiler error) |
| Forgeable | Yes (pass non-null garbage) | **Harder** (token provenance) |
| Analogy | C with careful `free()` | **Rust ownership** |

---

## Running the Proof

```bash
git clone https://github.com/YOUR_USERNAME/silent-decisions-proof
cd silent-decisions-proof
cargo test        # all 6 proof clauses
cargo run         # live demo
cargo test 2>&1 | grep -E "(PASSED|FAILED|error)"
```

Expected output:
```
running 6 tests
test proof::clause_1_valid_verdict_can_be_constructed ... ok
test proof::clause_2_evidence_token_is_linear ... ok
test proof::clause_3_verdict_struct_literal_is_blocked ... ok
test proof::clause_4_blake3hash_has_no_public_constructor ... ok
test proof::clause_5_tampered_verdict_fails_integrity_check ... ok
test proof::clause_6_dropped_token_produces_compiler_warning ... ok
test result: ok. 6 passed; 0 failed
```

---

## What the Proof Does NOT Claim

- ❌ That explanations are semantically truthful (undecidable)
- ❌ That the AI decision is correct (a separate problem)
- ❌ That compliance metadata is accurate (policy authoring problem)

## What the Proof DOES Claim

- ✅ Every materialized Verdict consumed exactly one EvidenceToken
- ✅ Every materialized Verdict consumed exactly one ComplianceToken
- ✅ No Verdict can exist without passing through `Verdict::new()`
- ✅ Tampering with a materialized Verdict is detectable

---

## Worked Examples

Three executable examples demonstrate BTV in semi-realistic scenarios:

| Example | Domain | Jurisdiction | Scenario |
|---------|--------|-------------|---------|
| `credit_scoring` | Finance | BR-LGPD | Credit denial with 30-day appeal window; reproducibility check |
| `hiring_screening` | HR | EU AI Act Art. 86 | High-risk CV screening; multi-candidate loop |
| `health_triage` | Healthcare | BR-LGPD | Emergency triage; Allow decisions also require evidence |

```bash
cargo run --example credit_scoring
cargo run --example hiring_screening
cargo run --example health_triage
```

Each example constructs verdicts from domain-specific payloads and verifies integrity.
Source code is in `examples/`.

---

## Connection to BuildToValue (BTV)

This repository isolates the type-theoretic proof from the full BTV system.  
The production kernel is at: [github.com/danzeroum/BuildToValueGovernance](https://github.com/danzeroum/BuildToValueGovernance)

---

## License
MIT
