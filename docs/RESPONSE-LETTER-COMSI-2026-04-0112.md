# Response Letter — COMSI-2026-04-0112 (Rev 5, pre-resubmission draft)

**Manuscript:** *Silent Decisions Are Type Errors: Enforcing AI Accountability via Linear Resource Types*
**Venue:** IEEE Computer — Special Issue on AI Governance
**Responding to:** Editor decision of 24-Aug-2026 (major revisions); reviewer reports R1, R2; editorial checklist E1–E3
**Artifact revision:** `review/bench-workspace-ffi` → `main` merge of PR #4 (base audited: `67dd265`; this letter cites every fix's commit hash below)
**Word-count method:** `scripts/count_words.py` (versioned, deterministic; abstract 149/150, body 5,997/6,000 running prose)

> **Method statement.** Every claim below was verified by EXECUTION, not by reading: full `cargo test` suites (unit, integration, trybuild compile-fail, partition, load, append-only, status-quo contrast), `cargo clippy --all-targets -- -D warnings` clean, `cargo audit` (0 RustSec advisories, machine-checkable JSON), RSS probe, 5-mode sweep, and the TCO derivation re-run. Gates that failed during preparation were fixed before this letter was written; one (`verify_reports`) caught a stale toolchain line and is cited as evidence the machinery works.

---

## Part E — Editor's checklist (E1–E3)

### E1 — Machine-checkable proofs and the clause count
**Pedido:** reproducible, machine-checkable evidence tied to a single artifact state.
**Feito:** the canonical proof-clause suite now lives in `btv-core` — `grep -c "fn clause_"` over `btv-core` equals **17**, and §1 and §7 of the manuscript state exactly that number. The suite: 24 lib unit tests + 11 integration proof clauses (`btv-core/tests/test_proof_clauses.rs`, commit `318d4ec`) + 9 trybuild compile-fail cases + partition/load/append-only/status-quo-contrast integration tests. The manuscript's earlier counts (§1/§7 "seven", submitted PDF "fifteen", `paper1/src/lib.rs` 17, `btv-core` 5) are reconciled to the single canonical count; the two divergent reference implementations are reconciled by making `btv-core` canonical and `paper1/src/lib.rs` a documented re-export (`318d4ec`, closes H4/H6).

### E2 — Status-quo baseline
**Pedido:** the contrast experiment demanded by E2.
**Feito:** the fail-open vs. fail-secure contrast (`btv-core/tests/test_status_quo_contrast.rs`) existed in Rev 4; Rev 5 adds the *symmetric* baseline arm demanded by the audit (F10): `status_quo_digest_log` (BLAKE3 digest + metadata) alongside `status_quo_async_log` (full-context JSON). The manuscript now reports the pair as a **sensitivity range** (§5): against a full-context logger BTV-durable is 2.6× faster at 4 KiB; against a digest-only logger it is 7.6× slower — "the difference is a design choice of the baseline, not of BTV" is stated in those terms (`3f4dbf3`, `7551703`).

### E3 — Economics reported qualitatively
**Pedido:** no crossover volume in the Computer submission; keep the economics qualitative.
**Feito (transparency commitment 1):** the economics were **removed** from the manuscript in Rev 4 (verified: zero occurrences of `N^*`, `500,000`, `crossover`, `TCO` across the eight `paper1/*.tex`). Rev 5 went further and corrected the ARTIFACT, because removing the number from the article while the cited artifact still printed it would be worse than not removing it. `scripts/compute_crossover.py` now **derives** ρ from the published primitives (ρ is no longer an input anywhere), estimates E[fine] from the 20-case corpus with bootstrap CIs, applies the corrected credit arithmetic, and the CI asserts internal consistency — never a value of N* (`5c85e4c`, closes F4). The companion `paper4` sections carrying the old numbers are headed SUPERSEDED with a pointer to the canonical model.

---

## Part R1 — reviewer report R1

**R1.1 (reproducibility of Table 2 / benchmarks).**
Single-execution Table 2 replaced: §5 now contains (i) Criterion construction latency over three payload sizes with 95% CIs on the pinned toolchain; (ii) two-platform thread-scaling table (AMD EPYC 9V74 4 vCPU; Intel Xeon 2 vCPU) with medians and CV over 5 trials; (iii) the five-mode contrast above; (iv) explicit P²-estimator disclosure. Every numeric cell traces to a committed CSV via `scripts/gen_section5_tables.py` (`7551703`, closes A2).

**R1.2 (durable persistence claim).**
The "durable" benchmark measured an in-memory SQLite database; WAL is silently ignored for `:memory:` and `synchronous=FULL` is meaningless without a file. Corrected: real on-disk sink, WAL+FULL effective — 220 µs mean (189–240, 95% CI) per durably persisted verdict on the reference container, ~160× the in-memory path; reported in §5 under "Durability is not free" (`3f4dbf3`, closes F7).

**R1.3 (statistical practice).**
Fabricated percentiles removed: Criterion provides a mean, so the CSV/table now carry `latency_stat = "mean (Criterion)"` with EMPTY percentile cells instead of the mean copied three times; sweep percentiles are labeled P² estimates; throughput is `total_ops / wall_clock` (`e9e9458`, closes F8). Tests can no longer overwrite committed evidence: load-test output goes to `CARGO_TARGET_TMPDIR`, and committed `reports/load_stats.csv` is produced only by `scripts/collect_load_stats.sh` beside a full fingerprint (`e9e9458`, closes F9). Reduced-footprint sweeps are labeled as such in their fingerprints; the 90 s dedicated-hardware collection remains tracked as due in `docs/EVIDENCE-MANIFEST.md` — we prefer an honest gap over a fabricated number.

## Part R2 — reviewer report R2

**R2.1 (affine vs. linear distinction).**
The distinction now appears in the manuscript itself (§3, `sec:affine-linear`), citing Walker: `#[must_use]` + `deny(unused_must_use)` + `pub(crate)` destructors recover linearity at the API surface, with `mem::forget` (by external callers), panic-unwind, and process abort documented as known escapes that bound liveness, not construction-time soundness (`1bf8ead`).

**R2.2 (positioning against prior linear-type work).**
§2.3 is rewritten: prior work is credited as establishing that non-functional obligations can be carried by types; claim C1 is rescoped to *instantiating* that discipline over regulatory obligations — "from resource management to regulatory accountability, not from nothing to regulatory accountability." §1's C1 is harmonized (no bare "first" claim) (`1bf8ead`).

**R2.3 (suggested references).**
**All four suggested references are incorporated** (transparency commitment 3 — the strongest available answer): DeLine & Fähndrich (PLDI 2001), Walker (ATTAPL 2005), Ahmed–Dreyer–Rossberg (POPL 2009), Pierce (TAPL 2002). `refs.bib` holds 14 entries, all cited in the body, under the 20 limit. **DOI integrity note:** the letter's DOIs were verified against CrossRef before citation and three were corrected — Vault is `10.1145/378795.378811` (the letter's `378795.378821` resolves to Berger/Zorn/McKinley's memory-allocator paper), Walker is `10.7551/mitpress/1104.003.0003` (the letter's `…0013` is the ATTAPL module-systems chapter), Ahmed et al. is `10.1145/1480881.1480925` (the letter's `1480915` resolves to "Equality Saturation"). TAPL has no CrossRef-registered DOI and is cited by ISBN. The corrections are documented in `refs.bib` (`1bf8ead`).

**R2.4 (L2 / signed compliance token).**
The manuscript described `C_signed`; the code did not implement it (`signing_key` was never read — the `dead_code` warning sat in the published clippy raw). Implemented (option a): `ComplianceToken` now carries an HMAC signature over `(jurisdiction, policy_version, deadline_hours)` with domain separation; `issue_token` signs with the recognized key (`BTV_AUTHORITY_KEY`/fallback); `Verdict::new` verifies it and returns `Result`, rejecting forged or foreign-authority tokens with `BtvError::InvalidTokenSignature`. The type law `V ⊸ (E ⊗ C_signed)` is now true in code (`e859e9b`, closes F2).

---

## Part H — hygiene items

| Item | Resolution |
|---|---|
| H1 (abstract 250 > 150) | Rewritten to **149 words** by the versioned method `scripts/count_words.py` (`c2075cc`) |
| H2 (references 10/20) | Now 14/20 with all four R2 references added (`1bf8ead`) |
| H3 (body at ceiling; method undocumented) | Method committed and versioned; body 5,997 words by that method, within 4,000–6,000 |
| H4 (four clause counts) | Single canonical count 17 == §1 == §7 (`318d4ec`) |
| H5 (four toolchains) | `rust-toolchain.toml` pins 1.98.1; §5.1, README, and `reports/tcb_summary.md` cite it (`318d4ec`, `6a51fd3`) |
| H6 (two implementations) | `btv-core` canonical; `paper1/src/lib.rs` = documented re-export; duplicate bench/UI trees removed (`318d4ec`) |

---

## Self-audit disclosure (transparency commitment 2)

During preparation of this revision we audited our own artifact by execution and found five defects that the manuscript's text contradicted. We report them here, before a reviewer can find them:

1. **F1 — the persisted-record seal never verified.** `Verdict::compute_hmac` and `VerdictRecord::verify_integrity` authenticated different preimages, so *no* persisted record ever verified. Fixed by a single `seal()` over the wire encoding with a domain tag and length prefixes; round-trip, six per-field tamper tests, and a field-boundary ambiguity test now enforce it (`38697a4`).
2. **F2 — the signed token did not exist.** Implemented for real (R2.4 above; `e859e9b`).
3. **F3 — the audit log was rewritable.** `INSERT OR REPLACE` allowed silent mutation of persisted verdicts. Now plain `INSERT` with byte-level conflict detection: identical replay is `Ok(())` (true idempotency), any divergence is `Err(LogConflict)` with the original row untouched (`8529b6d`).
4. **F4 — the crossover derivation contradicted its own equation by 10.8×.** ρ is derived, not declared; the corpus feeds the model; the credit arithmetic is corrected; the CI verifies consistency, never a conclusion (`5c85e4c`).
5. **F5 — the TCB report contained numbers its own evidence files contradicted.** Every figure in `reports/tcb_summary.md` is now generated by `scripts/gen_tcb_summary.py` and re-verified in CI by `scripts/verify_reports.py`, which re-derives each number from the committed raw evidence and fails on any divergence (`71fc41e`).
6. **F6 — the fail-secure path leaked memory.** `mem::forget` on non-`Drop` types (12,504 kB leaked per 200k rejected decisions) replaced by `drop`; RSS probe now shows **0 kB** delta (`813e901`).

Each defect ships with the tests that prevent recurrence. A defect we report and fix is credibility; the same defect found by a reviewer is rejection.

---

## Reconciliation of the submitted PDF (prerequisite item)

The attached submission (`13622128`, generated 24-Mar-2026, 10 references, "fifteen clauses") does not correspond exactly to any tree in this repository: it cites a §6 economics section with a crossover volume and reference [20], both absent from `main @ ebfa542` and from PR #4 @ `67dd265`, and its clause count matches neither implementation. We were unable to reconstruct which local working state produced that PDF. Resolution adopted for this resubmission: **the Rev 5 tree is the canonical artifact** — every number in the new submission traces to a committed CSV or raw report by file, line, and commit hash as listed above; the submitted PDF itself will be regenerated from `paper1/main.tex` at the exact commit cited on the submission form, so the reviewer can diff the letter's claims against a reproducible state. We flag the unreproducible prior PDF openly rather than paper over it.

## Remaining known gaps (stated, not hidden)

- The 90 s dedicated-hardware sweep (EVIDENCE-MANIFEST, pending item) has not replaced the reduced-footprint VM snapshots; §5.1 labels the snapshots as such and the table generator consumes the same CSV format, so the swap is mechanical.
- Mechanized proof in Lean 4/Coq remains future work (§6), as agreed in the editorial round.
- Durable-persistence absolute numbers are container-storage figures (documented lower bound); bare-metal numbers land with the dedicated-hardware sweep.

---
*Prepared per the audit method "execução, não leitura": every gate cited above was executed in the revision branch and re-executed in CI.*
