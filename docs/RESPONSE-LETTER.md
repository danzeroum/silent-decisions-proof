# Response to the Editorial Decision — COMSI-2026-04-0112

**Manuscript:** "Silent Decisions Are Type Errors: Enforcing AI Accountability via Linear Resource Types"
**Author:** Daniel Lau Pereira Soares
**Responding to:** Major-revisions decision (24-Aug-2026) and the pre-resubmission guidance audit of 14-Sep-2026
**Repository state at submission:** `danzeroum/silent-decisions-proof`, branch `claude/brave-curie-22eks7`, commit `6c988e680042cff7d6e008685619d7095fac3f5d`

## How to read this letter

Each item below follows the same structure: **what was asked → what was done → exact location → commit**. Every claim of "done" was checked by execution (`cargo test`, `cargo clippy -D warnings`, `cargo bench`, `python3 scripts/verify_reports.py`, `python3 scripts/compute_crossover.py`, `python3 scripts/count_words.py`) rather than by reading the code and assuming it works — the guidance audit's central finding was that four prior rounds had not done this, and we do not want to repeat that mistake in this letter.

Three commitments up front, because they are easy to bury in a long point-by-point list otherwise:

1. **The economics section was removed from the manuscript, not weakened, and the artifact was cleaned up to match.** §E1 below explains why, and names the one place (a stale corpus fine estimate) that still needs a primary-source check we could not complete without external access.
2. **Eleven numbered defects (F1–F11) and three hygiene inconsistencies (H4–H6) were found in our own audit of the artifact, not by a reviewer, and are now fixed with regression tests or generators that fail if they recur.** Part E below details the five (F1, F2, F3, F5, F6) that are not already the direct subject of an Editor or R2 item; the rest (F4 under E1, F7/F10 under E2, F5's report-generation half under R2.5, F8/F9/F11 and H4–H6 under E2/R2.6/R2.7/H2) are addressed inline where they belong. We say this plainly because a self-reported and self-fixed defect is a different thing, credibility-wise, than the same defect found on the second read of a resubmission — we would rather the record show which one this is.
3. **All four references Reviewer 2 recommended were incorporated** (§R2.7) — none were excluded, so there is no exclusion to justify.

---

## Part A — The Editor's Decision (E1–E3)

### E1 — Headline claim N\* = 500,000 decisions/year must not rest on an inaccessible companion manuscript

**What was asked:** Either import the crossover derivation into the main manuscript with public sources named inline, or visibly weaken the claim.

**What was done:** Weakened, in the specific sense the decision offered as an alternative to importing a companion manuscript — and then something the decision did not ask for, which we did anyway: the artifact's own derivation was corrected, because auditing it for this response surfaced that it was wrong.

The manuscript (`paper1/*.tex`) contains no N\* claim, no crossover volume, and no reference to a companion economics manuscript anywhere — verified by `grep -rn "N\*\|500,000\|500000\|crossover" paper1/*.tex`, zero matches. This was already true before this round (Rev 4, commit `fb6e9d5`); what was not true before this round is that the *artifact* still asserted the old number: `README.md`, `RELEASE_NOTES.md`, `reports/tco_summary.md`, and a CI job all stated N\* = 500,000 even after the paper stopped claiming it, which is arguably worse than not removing it at all — a reviewer who clones the repository (which the review checklist instructs) would find the number the paper doesn't make, standing uncontested in the artifact cited as `[soares2026a]`.

We found, by executing `scripts/compute_crossover.py` against the artifact's own published equation ρ = P_enf × E[fine] / N̄, that the GDPR row — the only row that produced the old headline number — contradicted that equation by a factor of 10.8×: the equation gives ρ ≈ 0.108, the file declared ρ = 0.01, and N\* = 500,000 followed only from the declared (wrong) value. We did not "fix" this by adjusting the declared constant to match the equation; we removed `rho_usd_per_decision` as an input entirely. `scripts/compute_crossover.py` now *derives* ρ for every regime from three primitive parameters (enforcement probability, expected fine, average decisions/year), and a CI job (`tco-reproducibility`) asserts that the derived ρ matches the ρ used downstream — never a target value of N\*, which is exactly the assertion that would have masked this bug (the original CI job asserted `490_000 ≤ N* ≤ 510_000`, which passes if and only if the wrong constant is kept).

Expected fine is now estimated from a 20-case enforcement corpus (`data/enforcement_cases.csv`) via median with a bootstrap confidence interval, rather than from the GDPR statutory cap (a ceiling, not an expectation), and the compliance-credit formula was corrected — the previous version divided by a *reduced* penalty rate, which mathematically makes adopting the credit look *less* attractive, backwards from what a compliance credit should do.

Data: `data/policy_parameters.yaml`, `data/enforcement_cases.csv`, `data/n_star_by_regime.csv`, `data/tco_plot_data.csv`. Script: `scripts/compute_crossover.py`. Report: `reports/tco_summary.md`. Commit: `5c85e4c` (derivation), this branch's HEAD (artifact-wide purge of the old number — verified: `grep -rn "500,000\|500000" README.md RELEASE_NOTES.md reports/ .github/` returns zero lines).

**What is not yet done, honestly:** two of the twenty corpus fines (the SEC 2022 sweep entries) look inconsistent with the commonly-cited "$125M average per institution" figure for that enforcement wave, and `ROADMAP.md` already listed "verify fines against primary sources" as pending before this round. We did not verify all twenty against primary regulatory filings — that is manual legal-document work outside what this response can complete, and we are naming it rather than letting it pass as done.

### E2 — Table 2: single-machine Criterion run; needs repetition on other hardware, variance under load, and a status-quo baseline comparison

**What was asked:** Repeat on different hardware, measure variance under concurrent load, and compare against a status-quo baseline rather than an isolated number.

**What was done:** All three, plus a fourth thing the decision did not ask for but that a defensive read of the artifact required — a second status-quo baseline, because the first one turned out not to be a fair comparison.

- **Different hardware:** two environments of different CPU vendor and model (Intel Xeon 2 vCPU KVM; AMD EPYC 9V74 4 vCPU Azure), full grids of 90 and 135 configurations respectively, 5 trials each. Data: `data/sweep_raw_20260914T171258Z.csv`, `data/sweep_raw_20260914T164735Z_runnervmlun5p.csv`. Commits `146bc6b`, `d1929dd`.
- **Variance under load:** `btv-core/benches/sweep_concurrent.rs`, thread-local P² quantile estimators (Jain & Chlamtac 1985; now cited, `paper1/refs.bib:jain1985p2`), reported CV over 5 trials — under 1% at every payload on both platforms (`paper1/section5_benchmarks.tex`, Table `tab:crossplatform`).
- **Status-quo baseline (first arm):** `Mode::StatusQuoAsyncLog` (commit `09b1d9b`) plus a fail-open/fail-secure contrast test, `btv-core/tests/test_status_quo_contrast.rs`.
- **Status-quo baseline (second arm, this round):** the guidance audit found that the first arm serialized the *entire* decision context hex-encoded while BTV logs a 32-byte digest — a real methodological asymmetry (finding F10), and the artifact's own second baseline script (`scripts/benchmark_baseline.py`) already logged a SHA-256 digest, contradicting the first one. `Mode::StatusQuoDigestLog` (commit `3889491`) now logs a digest exactly as BTV does. Reported as a sensitivity range rather than one number (`paper1/section5_benchmarks.tex`, Table `tab:fivemode`): against the full-context logger, BTV is 2.6× faster at 4 KiB; against the digest-only logger, 4.7× *slower*. Both are shown because which one is "the status quo" is a modeling choice, not a fact about BTV, and hiding the unfavorable one would be exactly the kind of single-baseline cherry-pick E2 objected to in the first place.
- **Durability (F7, found in our own audit, not asked for by E2 but load-bearing for what E2's "realistic" comparison means):** the original durable-persistence benchmark used `SqliteLogSink::open_in_memory()`, where SQLite silently ignores the WAL and `synchronous=FULL` pragmas it claimed to use — the reported "durable ACID" number was a RAM insert. `Mode::FullPipelineDurable` (commit `3889491`) uses a real on-disk file; the honest number is 23× slower than in-memory at the same payload and thread count, and is now the number the manuscript reports rather than the RAM figure.

**Two more measurement-integrity defects, found while preparing this evidence (F8, F9):** `scripts/benchmark_baseline.py` copied Criterion's bootstrap mean into the p50/p95/p99 columns of a table and a PGFPlots figure literally labeled "p95 latency" — Criterion does not report percentiles, only a mean, so those columns now read `--` for Criterion-sourced rows rather than a fabricated triple (commit `2d6907f`). Separately, `btv-core/tests/test_load.rs` wrote `reports/load_stats.csv` as a side effect of `cargo test`, with thread count taken from whatever machine happened to run it and an "ARM64 emulated" label decided by a `p50 > 100µs` latency guess — reproducing this ourselves overwrote the committed evidence with a different machine's numbers under a wrong label, live, while preparing this response. Fixed by splitting the test (measures, asserts, writes nothing) from a new deliberately-run `btv-core/examples/load_report.rs` that produces the committed file explicitly, with an explicit thread count and an emulation label taken only from an explicit flag, never from measured latency (commit `2d6907f`); verified `cargo test --workspace` now leaves `git status --porcelain` empty, which it did not before this fix.

Manuscript: `paper1/section5_benchmarks.tex` (fully rewritten this round, commit `6c988e6`). Remaining obligation, stated in the manuscript's own methodology paragraph rather than hidden: the citable headline dataset should come from a 90-second-per-configuration run on dedicated hardware with a pinned CPU governor; the committed datasets use reduced targets (1–10 s) appropriate to shared/CI-class hardware and are labeled as such in their fingerprints (`data/sweep_env_*.txt`).

### E3 — Polyglot/FFI threat model must move from footnote to main text

**What was asked:** Move the FFI trust boundary from a footnote/appendix into the main text as part of the threat model.

**What was done:** New §4.7 (`paper1/section4_theorem.tex:275-297`, `\label{sec:ffiboundary}`, commit `fb6e9d5`), stating the boundary as a threat-model claim: the PyO3 gateway is the sole authorized crossing point, a Python caller receives only an opaque `SealedVerdict` with read-only accessors, and Theorem 4.6 does not and cannot extend past the Rust enclave — the enforcement burden shifts to the downstream effector, which must independently verify the sealed handle. This is honest about what it does *not* claim: it does not analyze PyO3's own `unsafe` safety preconditions, and it does not cover C++/Java bindings, because none exist in this artifact. The full "what a Python caller cannot do" list lives in `btv-python/README.md` and `btv-python/src/lib.rs`'s doc comment, both pre-existing and now cross-referenced from §4.7.

---

## Part B — Reviewer 1

R1's original review text was not among the working materials available for preparing this response (only the Editor's decision letter's E1–E3 items and the internal guidance audit were), and the guidance audit's own projections of how R1 and R2 would react to an unfixed resubmission are a prediction about this round, not a quotation of R1's original comments from the prior round. We are not willing to write a point-by-point R1 section from a document we have not read: doing so would mean either paraphrasing the decision letter's summary as if it were R1's own words, or inventing specific line items to look thorough, and this letter's whole premise is that a claim not checked against its source does not belong in it. If R1 raised items distinct from E1–E3 that the Editor did not fold into the decision letter, we ask that they be forwarded, and we will respond to them specifically rather than by inference.

---

## Part C — Reviewer 2 (R2.1–R2.7)

### R2.1 — Reformulate the "CAL Theorem" as a conceptual trilemma OR provide full formal semantics + theorem-level proof

**Done.** `paper1/section6_discussion.tex`, §6.3 "CAL Design Trade-Offs" (`\label{sec:cal}`): stated explicitly as "a practical design observation, not a formal impossibility result," with the FLP analogy (`flp1985`) named as an open question rather than a proven result. Commit `67dd265` (Rev 4 compression to one paragraph); tightened further this round (commit `6c988e6`) without changing its claims.

### R2.2 — Strengthen Constitutional Enclosure: explicit semantics, declared assumptions, mechanized verification (preferably)

**Partially done, and we are not claiming the "preferably" part.** `paper1/section4_theorem.tex` states three definitions, three axioms, and a constructive proof by exhaustive case analysis (Theorem 4.6, `\label{thm:enclosure}`), each case backed by a compile-fail test that is actually run: `btv-core/tests/ui/*.rs` (9 fixtures, all exercised by named `clause_N` tests in `btv-core/src/lib.rs`'s test module — 17 in total, matching the count §1 and §7 now cite consistently; see the gate table at the end of this letter). What we do **not** claim is mechanized verification in a proof assistant: `paper1/section6_discussion.tex`'s Future Work item 3 states plainly that Lean 4/Coq elevation has not been done. We are flagging this ourselves so it is not mistaken for an oversight: the machine-checked compile-fail suite is real and executed; a mechanized proof is not, and the manuscript does not say otherwise.

### R2.3 — Affine vs. linear ownership: state it and explain the closing mechanisms

**Done this round.** `paper1/section3_type_system.tex`, new "Affine, not strictly linear" paragraph (commit `6c988e6`): states that Rust ownership is affine (0-or-1 uses) rather than strictly linear (exactly once), citing Walker's *Substructural Type Systems* (`walker2005substructural`, now in `refs.bib`); states that `#[must_use]` + `#![deny(unused_must_use)]` recovers linear behavior at the crate's API surface; and names the three escape hatches precisely — `mem::forget`, a panic between construction and consumption, and process abort — noting that none of the three lets a `Verdict` materialize without evidence (each only discards evidence, which is fail-secure, not a silent decision). This concept existed only in a `btv-core/src/lib.rs` doc comment before this round (verified: the word "affine" did not appear in any `paper1/*.tex` file prior to commit `6c988e6`).

### R2.4 — Extend evaluation: production-realistic loads, distributed failures, multi-language inference stacks

**Concurrency/load and polyglot: done. Distributed failures: scoped as future work, not claimed as solved.** Concurrency is the sweep harness described under E2 (thread counts [1,2,4], variance reported). The polyglot stack is `btv-python/` (PyO3 0.29, `#![forbid(unsafe_code)]`, 13/13 pytest passing in CI), now the subject of §4.7 per E3. Distributed failure evaluation has no artifact behind it; `paper1/section6_discussion.tex` L3/L4 names the single-node evidence-chain limitation and proposes a sidecar pattern as a direction, not a result. We are not claiming this item is closed.

### R2.5 — TCO sensitivity with disclosed assumptions + supply-chain controls for unsafe deps

**Done for the supply-chain half; the TCO half is now qualitative per E1's resolution.** `reports/tcb_summary.md` is regenerated entirely from raw evidence (`scripts/gen_tcb_summary.py`), not hand-written — commit `71fc41e`, closing a defect (F5) where the previous hand-written version listed two dependencies (`ring`, `rustls`) that were absent from `Cargo.lock` at the time, attributed a fabricated total to `blake3` (off by 4× from the CSV beside it), and quoted `cargo audit` output that had no result line. `scripts/verify_reports.py` now reconciles every number in the report against its source and runs in CI; we verified it passes (`VERIFY-REPORTS: OK`) before writing this letter. `data/policy_parameters.yaml` discloses every cost component with a public list-price citation (AWS KMS/EC2/S3/CloudHSM pricing pages, dated 2026-09-15) — the previous "$5,000/yr" figure had no citation and was implausible on its face. Since E1 moved the TCO claim itself to qualitative-only in the manuscript, the sensitivity analysis lives in the artifact (`appendix_b_pgfplots.tex`, `data/tco_plot_data.csv`) rather than as a manuscript figure.

### R2.6 — Moderate categorical superiority claims

**Done.** Swept `zero-cost abstraction`, `practically free`, `eliminates entirely`, `empty by construction`, and `cannot materialize` from every file in `paper1/` (verified: `grep -rniE` for all five patterns across `paper1/*.tex` returns zero matches after commit `6c988e6`). These appeared in the abstract, §1, §5, and §7 and had **not been touched in four prior rounds** despite three of those rounds making other changes to the same sections — the diffs that did land were compression to fit the word limit, not moderation, which is a different edit even when it touches the same lines. Replacements are specific, not vaguer: "cannot materialize" → "cannot be constructed within Safe Rust's type boundaries"; "empty by construction" → "empty within the crate's type perimeter, under the assumptions of [§3.3, "Scope of the Theorem" — `\S\ref{sec:encapsulation}` in the LaTeX source, auto-numbered rather than hand-typed for exactly this reason]"; "zero-cost abstraction" → a statement that the type-level mechanism adds no runtime branch, with the actual measured cost (cryptographic primitives, and durable persistence at a stated multiplier) reported instead of asserted away. §6.1 "Scope and Boundaries of the Guarantee" already carried the strongest existing hedging and was left standing, not weakened further — R2.6 asked for moderation of overclaims, not for the honest limitations section to be cut.

### R2.7 — Engage the four recommended references (or justify exclusion)

**All four incorporated; none excluded, so this is the strongest form of the response the decision letter itself named as available.** `paper1/refs.bib` (now 15/20 entries used):
- DeLine & Fähndrich, *Enforcing High-Level Protocols in Low-Level Software*, PLDI 2001 (`deline2001vault`) — engaged as the closest prior system (Vault), not omitted: `paper1/section2_related_work.tex`'s closing paragraph now names it explicitly and scopes our novelty claim against it, rather than asserting priority as though it did not exist.
- Walker, *Substructural Type Systems*, in *Advanced Topics in Types and Programming Languages* (`walker2005substructural`) — cited in both §2.3 and the new affine/linear paragraph in §3 (R2.3, above).
- Pierce, *Types and Programming Languages* (`pierce2002tapl`) — cited alongside Walker as the textbook grounding for the substructural distinction.
- Ahmed, Dreyer, and Rossberg, *State-Dependent Representation Independence*, POPL 2009 (`ahmed2009statedependent`) — cited in §2.3 as the closest treatment of state-dependent representations, which is how we describe a `ComplianceToken`'s validity depending on the issuing authority's state.

One correction we made in the course of adding these, worth stating plainly: the DOIs for the DeLine/Fähndrich and Ahmed/Dreyer/Rossberg papers, as given to us, each had the same two-digit transposition error relative to the ACM Digital Library's own listing (`.378821`→`.378811` and `.1480915`→`.1480925`). We verified all four DOIs against the publisher/ACM listings before citing them; `refs.bib` carries the corrected values.

---

## Part D — Submission Hygiene (H1–H3)

**H1 — no colored/highlighted text in the main file.** Unchanged from Rev 3: `xcolor` is loaded only for `lstlisting` styling; zero `\textcolor`/`\hl` in prose (`grep`-verified across all `paper1/*.tex`).

**H2 — 4,000–6,000 word limit.** The abstract was itself the reason the desk returned this manuscript once already (13-Apr-2026): it measured 250 words against a 150-word limit that round, and had not been touched since despite three later rounds editing the same file. It is now 144 words. The main body (all seven sections, `\input` resolved, bibliography excluded) is 5,991 words, under the 6,000 ceiling with a small margin rather than exactly at it. Both counts come from a documented, versioned, and runnable method, `scripts/count_words.py` (this round; previously the count existed only as a one-off `pandoc` invocation whose command was never committed) — we consider an undocumented counting method its own finding, separate from whether the count itself passed, since a method a reviewer cannot re-run is not evidence.

**H3 — response letter with a summary of changes.** This document. `docs/EVIDENCE-MANIFEST.md` (commit `6d03016`, extended this round) is the underlying traceability map this letter draws from; every claim above should be checkable against it and against the cited commits without re-deriving anything from prose alone.

---

## Part E — Findings from our own audit of the artifact (not raised by any reviewer)

We ran the artifact rather than reading it, because the point of this round was to stop trusting our own prior claims. Five defects surfaced that no reviewer had flagged, because they lived entirely in the repository, not in the manuscript text a reviewer would read:

- **F1 — the persisted record's integrity seal never verified.** `Verdict::compute_hmac` and `VerdictRecord::verify_integrity` authenticated different byte sequences, so every record that crossed the process boundary failed its own integrity check, silently, with zero test coverage catching it across four rounds. Fixed by unifying both onto one `seal()` function with length-prefixed, domain-separated fields (commit `38697a4`); verified by a round-trip test plus six field-level tamper tests plus one field-boundary-ambiguity test, all of which we re-ran before writing this letter (`record_seal_roundtrip`, `record_tamper_*` ×6, `seal_is_unambiguous` — 8/8 pass).
- **F2 — the "signed" `ComplianceToken` was not signed.** `ComplianceAuthority.signing_key` was dead code (`cargo clippy` warned about it in the artifact's own published lint report, uncategorized); the manuscript's §6.2 described a cryptographic mechanism the code did not implement. Fixed: `ComplianceAuthority` now signs `(jurisdiction, policy_version, deadline_hours)` with HMAC, and `Verdict::new` verifies the signature, rejecting a forged or foreign-authority token (commit `e859e9b`; tests `forged_token_signature_rejected`, `rogue_authority_token_rejected`, `signed_token_happy_path`, `compliance_signature_is_unambiguous` — 4/4 pass). `paper1/section6_discussion.tex`'s L2 paragraph is corrected to describe this mechanism accurately (this round).
- **F3 — the audit log was overwritable.** `SqliteLogSink::append` used `INSERT OR REPLACE`, so any caller could silently rewrite a previously-logged verdict by reusing its `evidence_id` — the opposite of append-only. Fixed: plain `INSERT`, with byte-identical replays accepted as idempotent and any divergent replay rejected as `BtvError::LogConflict` (commit `8529b6d`; test `append_same_id_different_payload_is_rejected` confirms the original record survives a conflicting write attempt).
- **F5 — the TCB report contained fabricated numbers.** Covered under R2.5 above.
- **F6 — the fail-secure path leaked memory, defended by a misreading of the compiler's own warning.** `issue_verdict` called `mem::forget` on rejected tokens, measured at ~64 bytes leaked per rejected decision (~14 GB/hour at the artifact's own claimed throughput); the design defense cited a clippy lint name that does not exist. Fixed: `drop(...)` in place of `mem::forget` (commit `813e901`); we re-ran the RSS probe ourselves before writing this letter — 4 kB delta over 200,000 rejected decisions, against a 1 MB budget (`cargo run --release --features test-support --example rss_probe_fail_secure`).

We report these here, in our own words, rather than waiting for a reviewer to find them, for the reason stated at the top of this letter.

---

## Portões (acceptance gates) verified by execution for this letter

| Gate | Command | Result |
|---|---|---|
| Full workspace test suite | `cargo test --workspace --features test-support` | pass (0 failures) |
| btv-core clippy, deny warnings | `cargo clippy -p btv-core --all-targets --features test-support -- -D warnings` | clean |
| Report reconciliation | `python3 scripts/verify_reports.py` | `VERIFY-REPORTS: OK` |
| TCO internal consistency | `python3 scripts/compute_crossover.py` | derived ρ, no hand-set value |
| RSS probe (F6) | `cargo run --release --features test-support --example rss_probe_fail_secure` | delta 4 kB / 200k calls |
| Abstract word count | `python3 scripts/count_words.py --abstract` | 144 (≤150) |
| Body word count | `python3 scripts/count_words.py` | 5,991 (≤6,000) |
| Clause count matches manuscript | `grep -c "fn clause_" btv-core/src/lib.rs` | 17 (matches §1/§7) |
| Banned phrases | `grep -rniE "zero-cost\|practically free\|eliminates entirely\|empty by construction\|cannot materialize" paper1/*.tex` | zero matches |
| Old headline number purged from artifact | `grep -rn "500,000\|500000" README.md RELEASE_NOTES.md reports/ .github/` | zero matches |

**Not run: a full PDF compile.** This response was prepared in an environment without a LaTeX toolchain; the checks above are structural (balanced `\begin`/`\end` pairs and braces in every `paper1/*.tex` file, every `\ref`/`\label` pair resolved, every `\cite` key present in `refs.bib`) rather than a compiled-PDF check. We recommend a compile pass before final submission and will address any errors it surfaces.

**Two implementations, reconciled.** `paper1/src/lib.rs` and `btv-core/src/lib.rs` diverged over four rounds — most visibly, the `paper1` copy never received the F2 signing fix, because a fix applied to one crate had no way to reach the other. `btv-core` is now canonical; `paper1/src/lib.rs` re-exports it (`pub use btv_core::*;`, commit `449cdfc`). All four worked examples (`paper1/examples/*.rs`) and the demo binary (`paper1/src/main.rs`) were verified to still run end-to-end against the unified implementation before this letter was written.
