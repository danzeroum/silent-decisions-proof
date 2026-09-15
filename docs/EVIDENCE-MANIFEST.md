# Evidence Manifest — Editorial Decision COMSI-2026-04-0112 (Round 3)

**Purpose.** This file maps every requirement of the editorial decision
letter (E1–E3 from the Editor-in-Chief, R2.1–R2.7 from Reviewer 2 as
elevated to major revision, H1–H3 submission hygiene) to the exact
repository artifact that supports it, so the manuscript rewrite phase can
cite every number and empirical claim without re-excavating this
round's audit context. It is the bridge between repository work
(Rounds 1–3) and the textual rewrite.

**How to read it.** Each entry carries: the audit status assigned by the
Round 3 traceability audit (before any Phase-B closure work), the
artifacts (file path, commit SHA, line numbers where useful), and what
remains for the rewrite. Status vocabulary: ✅ resolved in code/data,
📝 support artifact exists / manuscript text pending, ⚠️ partial, ❌ not
resolved.

**Branch:** `review/bench-workspace-ffi` (PR #4). Round 3 commits:
`09b1d9b` (B1 baseline mode + contrast test), `b1850d1` (B4 geiger note),
`355122d`/`7e6e284`/`1a06125` (B3 workflow + marker + script fix),
`5bf0dc0` (rustls RUSTSEC bump), `146bc6b` (B2 sandbox dataset),
`d1929dd` (B3 runner dataset).

---

## E1 — Headline claim N* = 500,000 decisions/year must not rest on an inaccessible companion manuscript

**Audit status: ❌ (pure writing task; no draft exists).**

| Artifact | Where | Notes |
|---|---|---|
| Full derivation with public sources | `paper4/section5_crossover.tex` | ρ = P_enf × E[fine]/N̄_controller per regime; GDPR via DLA Piper 2025 enforcement data (€10M ≈ $10.8M median Art. 22 fine, ~10⁶ decisions/yr → ρ = $0.01); N* = C_fixed/ρ = $5,000/$0.01 = 500,000. **This is the complementary manuscript itself — exactly the dependency the Editor rejected as sole support.** |
| Versioned parameters + sources | `data/policy_parameters.yaml` | All ρ/C_fixed/δ values with per-regime provenance comments; edit → recompute. |
| Computed N* per regime | `data/n_star_by_regime.csv` | GDPR row: `n_star_no_credit = 500000.0`. |
| Sensitivity grid data | `data/tco_plot_data.csv` | Corrected N* = C_fixed/(rho − c_variable) curves for the appendix plot (OS-04). |
| Enforcement case data | `data/enforcement_cases.csv` | 20 regulatory cases backing the penalty model. |
| Reproducible recompute | `scripts/compute_crossover.py` | Deterministic; rho DERIVED from primitives (OS-04), corpus-based E[fine] with bootstrap CIs; regenerates CSV + appendix snippet + `reports/tco_summary.md`. |
| CI guard (executable evidence) | `.github/workflows/ci.yml`, job `tco-reproducibility` | OS-04: asserts INTERNAL CONSISTENCY (rho used == rho derived; N* == C/(rho-c_var)) — never a particular N* value. |
| Pre-built appendix snippet | `appendix_b_pgfplots.tex` (repo root, from `abcf186`) | PGFPlots sensitivity figure; comment says "Append to IEEE Computer manuscript Appendix B" — **NOT wired into `paper1/main.tex`** (which inputs no appendix). |

**What the rewrite must do (choice the Editor offered):** import the
derivation (parameters + per-regime estimation + N* = C_fixed/ρ + the
sensitivity plot) into the main manuscript as an appendix, with the
public sources named inline — or visibly weaken the claim. Note: the
current `paper1` draft in this repository contains **no** N* claim in
any section (verified by grep across all `.tex`); the claim the letter
quotes from "Section 6" exists in the submitted manuscript, not in this
tree. All material needed to re-state it self-containedly is listed
above.

---

## E2 — Table 2: single-machine Criterion run; needs repetition on other hardware, variance under load, and a status-quo baseline comparison

**Audit status before Phase B: ❌ aggregate (baseline ❌ / real data ⚠️ /
multi-hardware ⚠️). After Round 3 closure: baseline ✅, real data ✅
(reduced target, two environments), multi-hardware ✅ for collection
breadth — headline numbers still require the 90 s dedicated-hardware
run.**

| Artifact | Where | Notes |
|---|---|---|
| Concurrency sweep harness (p50/p95/p99 under load) | `btv-core/benches/sweep_concurrent.rs` (Round 1 `3eab997`; P² O(1) memory Round 2 `814a5c0`) | Thread-local P² estimators (Jain & Chlamtac 1985), no synchronization in the measured path; methodology documented in the file header for Section 5.1. |
| **Status-quo baseline mode** (Round 3, B1) | same file, `Mode::StatusQuoAsyncLog` + `StatusQuoRecord`/`build_status_quo_record` (commit `09b1d9b`) | Same decision data, JSON serialization with FULL context hex-embedded, fire-and-forget `std::sync::mpsc` hand-off, **no linear-type discipline**. Channel discipline: thread-local pair per Rayon worker (no shared-queue contention — anti-strawman), receiver dropped at setup (dummy sink; O(1) memory), send error explicitly ignored (`let _ =`) — fail-open. |
| **Fail-open vs. fail-secure experiment** | `btv-core/tests/test_status_quo_contrast.rs` (`09b1d9b`) | (A) dead logger: decision "succeeds", record destroyed in the discarded `SendError`; (B) backlogged logger: 1000 decisions all succeed, records vanish undrained; (C) the JSON record is well-formed (SIEM-usable); (D) BTV contrast: `Verdict` exists only through consumed tokens; `evidence_id()` re-verified against an independent BLAKE3 re-hash. Runs in CI (x86-64 job). |
| **Sandbox dataset** (Round 3, B2) | `data/sweep_raw_20260914T171258Z.csv` + `data/sweep_env_20260914T171258Z.txt` (`146bc6b`) | Intel Xeon (Model 173), 2 vCPU, KVM, kernel 5.10, rustc 1.98.1. Complete grid: 3 payloads × 3 modes × [1,2] threads × 5 trials = 90 configs; ≥1M timed ops/thread/run. `BTV_SWEEP_TARGET_WALL_SECS=1` (documented in fingerprint + commit). |
| **Hosted-runner dataset** (Round 3, B3) | `data/sweep_raw_20260914T164735Z_runnervmlun5p.csv` + `data/sweep_env_20260914T164735Z_runnervmlun5p.txt` (`d1929dd`) | AMD EPYC 9V74, 4 vCPU (Azure), kernel 6.17, rustc 1.98.1 — different CPU vendor AND model. Complete grid: 135 configs (threads [1,2,4]). Collected via workflow run 34870780617 (`.github/workflows/sweep-dispatch.yml`). 10 s/config. |
| Multi-architecture validation (pre-existing) | `reports/hardware_comparison.md`, `reports/load_stats.csv`, `reports/load_stats_arm64_qemu.csv`, CI job `Test (ARM64 via QEMU)` | Full test suite green on x86-64 native + ARM64 QEMU. **QEMU latency numbers are explicitly non-representative** (15–30× translation overhead; the report's own epistemic footer says so). |
| Existing Criterion single-run | `paper1/section5_benchmarks.tex` (Table `tab:new`/`tab:verify`), `reports/benchmark_baseline.{csv,md}` | The original Table 2 evidence the letter cites. |

**Numbers the rewrite may cite** (p50, 1 thread, from the two committed
datasets):

| payload | full_pipeline (Xeon/EPYC) | verdict_only | status_quo_async_log | sq/full |
|---|---|---|---|---|
| 64 B | 378 / 410 ns | ~303 / ~330 ns | 701 / 450 ns | 1.9× / 1.1× |
| 512 B | 734 / 881 ns | ~304 / ~330 ns | 3571 / 1962 ns | 4.9× / 2.2× |
| 4096 B | 1273 / 1722 ns | ~304 / ~330 ns | 26532 / 14148 ns | 20.8× / 8.2× |

`verdict_only` is payload-independent on both machines (only
`Verdict::new`: HMAC over a 32-byte digest); the status-quo record
embeds the full context and scales with it — the exact structural
contrast the baseline exists to expose. The status-quo/full ratio
*widens* on the throttled 2-vCPU sandbox: the guarantee's relative price
is hardware-sensitive and should be reported as a range across the two
environments, not a single number.

**Remaining repo-side obligation:** the paper's headline dataset must be
a 90 s/config run (`scripts/run_sweep.sh` with no override) on dedicated
hardware with the CPU governor pinned to `performance`; the committed
reduced runs are labeled as such in their fingerprints.

---

## E3 — Polyglot/FFI threat model must move from footnote to main text

**Audit status: 📝 (support prose complete in repo; manuscript text pending).**

| Artifact | Where | Notes |
|---|---|---|
| Citable threat-model prose | `btv-python/README.md` § "Trust boundary (L4 — polyglot boundary)" (Round 1 `f8bcbf6`) | The five things a Python orchestrator cannot do (forge evidence hash; bypass authority; reuse `EvidenceToken`; silently drop a verdict in flight; inspect/mutate internals), what it can do, the downstream-effector obligation, honest out-of-scope list. |
| In-code threat model | `btv-python/src/lib.rs` doc comment "Threat model (L4 — polyglot boundary)" (line 13) | Same content at the code boundary. |
| Fail-secure behavior evidence | `btv-python/README.md` end ("BTVError when the log sink is unavailable — fail-secure: no verdict is emitted"); pytest suite 13/13 in CI | Runtime counterpart to the E2 contrast test. |
| Trust-assumption alignment | `paper1/section6_discussion.tex` L1 paragraph (BTV_HMAC_KEY ↔ Log Authority HSM) | Written for the same boundary. |

**Rewrite:** lift the README section into the main text (target: the
Section 4.4 it was written to accompany), keeping the numbered
cannot-list structure.

---

## R2.1 — Reformulate the "CAL Theorem" as a conceptual trilemma OR provide full formal semantics + theorem-level proof

**Audit status: 📝 (reformulation already drafted in the tree; keep and sharpen).**

| Artifact | Where | Notes |
|---|---|---|
| CAL as design trade-off (not theorem) | `paper1/section6_discussion.tex` §6.3 "CAL Design Trade-Offs" (line 182) | "We present this as a practical design observation rather than a formal impossibility result." FLP analogy explicitly flagged as open question, not claimed. Header comment documents the demotion (Rev 2, "Gap 4: CAL demoted to design trade-off; CAP analogy removed"). |

**Rewrite:** verify no residual "Theorem CAL" phrasing anywhere; the
response letter should point to §6.3 and cite FLP honestly
(`refs.bib: flp1985`).

---

## R2.2 — Strengthen Constitutional Enclosure: explicit semantics, declared assumptions, mechanized verification (preferably)

**Audit status: 📝 (semantics + assumptions + machine-checkable compile-fail suite exist; proof-assistant mechanization does not — declared future work).**

| Artifact | Where | Notes |
|---|---|---|
| Theorem with explicit axioms | `paper1/section4_theorem.tex` | Three definitions, three axioms (Resource Introduction; Resource Uniqueness and Anti-Weakening; Encapsulation Boundary), constructive proof, Safe Rust memory-model scope statement. |
| Machine-checkable counterpart | `btv-core/tests/ui/` (8 compile-fail fixtures) + `btv-core/tests/trybuild.rs`; CI job step "Run trybuild compile-fail tests" | Dropped token, token reuse, struct-literal construction, external consume — each the compile-time mirror of an axiom violation. |
| Mechanization status | `paper1/section6_discussion.tex` Future Work item 3 | Lean 4/Coq elevation is explicitly future work — the response letter must NOT claim mechanized verification; it can claim machine-checked compile-fail evidence. |
| Protected implementation | `btv-core/src/lib.rs` (`EvidenceToken`, `ComplianceToken`, `Verdict::new`, protections A/B/C — untouched through Rounds 1–3 by constraint) | The API the axioms describe. |

---

## R2.3 — Affine vs. linear ownership: state it and explain the closing mechanisms

**Audit status: 📝 (concept fully documented in code; article text pending).**

| Artifact | Where | Notes |
|---|---|---|
| Affine-vs-linear caveat | `btv-core/src/lib.rs` doc comment "## Affine-vs-linear caveat (R2)" (line 355) | Rust ownership is *affine* (0-or-1 uses), not strictly *linear* (exactly-once). `#[must_use]` + `#![deny(unused_must_use)]` escalate unused-token warnings to compile errors, recovering linearity **at this crate's API surface**. Escape hatches listed honestly: `mem::forget`, panic-during-unwind, process abort. |
| `#[must_use]` inventory | throughout `btv-core/src/lib.rs` (tokens, `Verdict`, accessors) | The mechanism itself. |
| Runtime counterpart | `reports/tcb_summary.md` (referenced by the caveat) | Threat-model treatment of the escape hatches. |

**Rewrite:** the current `paper1` draft never uses the word "affine"
(verified by grep) — add the explanation with the mechanisms above.

---

## R2.4 — Extend evaluation: production-realistic loads, distributed failures, multi-language inference stacks

**Audit status: ⚠️ (concurrency-load ✅ after Round 3; polyglot stack ✅; distributed failures ❌ — acknowledged limitation, scope decision for the rewrite).**

| Artifact | Where | Notes |
|---|---|---|
| Load/concurrency | E2 artifacts above (sweep harness + two datasets, [1,2,4] threads) | Variance under load = the p95/p99/p99_max columns. |
| Multi-language inference stack | `btv-python/` (PyO3 0.29 gateway, `#![forbid(unsafe_code)]`, abi3-py38 wheel, 13/13 pytest in CI) | Real FFI boundary, CI-tested every push. |
| Distributed failure evaluation | none | `paper1/section6_discussion.tex` L3/L4 (single-node evidence chain; sidecar pattern as proposal). The response letter must scope this honestly as future work — no artifact claims otherwise. |

---

## R2.5 — TCO sensitivity with disclosed assumptions + supply-chain controls for unsafe deps (cargo-geiger inventories, does not prove soundness)

**Audit status: ⚠️ → ✅ for the caveat (Round 3, B4); sensitivity/assumptions 📝.**

| Artifact | Where | Notes |
|---|---|---|
| Unsafe inventory (post-workspace) | `reports/cargo_geiger_unsafe_inventory.csv` + `.md` (Round 2 `ad771d0`) | 232 packages / 139 with unsafe / 21,062 occurrences; regenerated from the root lock. |
| **Mandatory scope note** (Round 3, B4) | `reports/cargo_geiger_unsafe_inventory.md` top (commit `b1850d1`, line 5) | Verbatim citable: cargo-geiger inventories unsafe blocks, does not prove absence of unsoundness; zero-unsafe policy reduces known attack surface but is not a formal proof that Theorem 4.1 is preserved under composition with any specific dependency. |
| TCO assumptions disclosure | `data/policy_parameters.yaml` (component-level C_fixed: HSM $1.5k, SIEM $1k, WORM storage $1.5k, audit pipeline $1k) | Integration/storage/maintenance/audit/migration assumptions are enumerated. |
| Sensitivity analysis | `appendix_b_pgfplots.tex` + `data/tco_plot_data.csv` (ρ × C_fixed grid, 4 fixed-cost scenarios) | Ready-made appendix figure. |
| TCO narrative | `paper4/section3_tco_model.tex`, `paper4/section7_discussion.tex` | Source material for the rewrite's assumption disclosure. |

---

## R2.6 — Moderate categorical superiority claims

**Audit status: 📝 (writing task).**

Existing hedging to preserve/strengthen: `paper1/section6_discussion.tex`
§6.1 "Scope and Boundaries of the Guarantee" (semantic truthfulness gap,
unsafe Rust policy framing). The rewrite should sweep the abstract and
introduction for categorical claims and tie each empirical number to its
dataset (E2 table above) and environment.

---

## R2.7 — Engage the four recommended references (or justify exclusion)

**Audit status: 📝 (none of the four is cited anywhere in the repository yet — verified by grep over all `.bib`/`.tex`/`.md`; concepts partially documented in code).**

| Reference | In repo? | Concept anchor for the rewrite |
|---|---|---|
| Ahmed, Dreyer, Rossberg — POPL 2009 | no | Related-work positioning of regional/linear resource disciplines. |
| DeLine & Fähndrich — PLDI 2001 (Vault) | no | **Prior art of FFI/type-enforced protocols — the closest historical system; must be engaged, not omitted.** No code comment mentions Vault yet. |
| Pierce — TAPL | no | Textbook grounding for the type-system exposition. |
| Walker — Substructural Type Systems | no | The canonical survey for linear/affine semantics — pairs directly with the R2.3 affine-vs-linear caveat (already in `btv-core/src/lib.rs:355`). |

Existing related work cites Girard 1987, Wadler 1991, RustBelt
(`paper1/section2_related_work.tex`, `paper1/refs.bib`) — the four
additions slot into the same section.

---

## H1 — Main file without colored/highlighted text

**Audit status: ✅ in source.** `paper1/main.tex` loads `xcolor` solely
for `lstlisting` styles (keyword/comment/string/background colors —
lines 32–43); zero `\textcolor`/`\hl` in any prose (grep-verified across
all `paper1/*.tex`). Verify the final PDF against the venue's listing
rules before submission.

## H2 — 4,000–6,000 word limit

**Audit status: ⚠️.** Raw `wc -w` over `main.tex + abstract + section1–7`
with comment lines stripped = **6,529 words** (markup residue inflates
this by roughly 10%; the true prose count is near or slightly above the
6,000 ceiling). The rewrite must trim — the E1 appendix import and E3
threat-model import add text, so cuts elsewhere are mandatory.

## H3 — Response letter with a summary of changes

**Audit status: ❌ → 📝.** No draft exists in the repository. This
manifest + the Round 3 audit matrix are the traceability backbone for
it: every claim in the response letter should point at one row above.

---

## Standing repo-side obligations (not writable, only runnable, by the author)

1. **90 s/config sweep on dedicated hardware** (`scripts/run_sweep.sh`,
   no env override, CPU governor `performance`, turbo disabled) — the
   citable headline dataset. The committed datasets remain reduced-target
   and labeled as such.
2. **PAT rotation** — the repository access token was re-exposed in chat
   during Rounds 1–3; rotate it now. The PR #4/#5 merge this obligation
   said to precede has already happened, so this is overdue, not merely
   pending, and cannot be closed by any commit — only by revoking the
   token in GitHub's own settings.

Resolved this round (G3, COMSI-2026-04-0112 Round 2): the two `paper2`
`.stderr` fixtures noted above as "retained for human review, diff
substantive" were re-examined — `git diff` on the regenerated files shows
exactly one `= note: ... originates in the macro \`todo\`` line removed
per file, a diagnostic-verbosity change under the newer pinned toolchain,
not a change to what's being asserted. The Round 2 audit independently
reached the same conclusion ("o diff é de uma linha de `note`, não
semântico") and instructed committing it; done.

---

## Rodada 4 — Cirurgia textual da Seção 6 (14 set 2026)

Commit: `fb6e9d5` (branch `review/bench-workspace-ffi`, PR #4).

| ID | O que mudou | Arquivo:linha (pós-edição) |
|---|---|---|
| E1 | Claim de crossover quantitativo removido; reformulado como observação qualitativa sobre ponto de inflexão de custo marginal | paper1/section6_discussion.tex (§6.3, linhas 158–180; header Rev 4, linhas 17–21) |
| R2.1 | CAL comprimido a um parágrafo de trade-off de design; framing FLP mantido como questão aberta | paper1/section6_discussion.tex:158–180, `\label{sec:cal}` |
| E3 | Threat model FFI promovido para o corpo principal, citando a arquitetura real do btv-python | paper1/section4_theorem.tex:275–297, `\label{sec:ffiboundary}` (novo §4.7) |
| R4b/A | Overhead empírico do EscalatedVerdict (160B vs 152B, x86-64) restaurado no proof do Corolário 4.8 — o datum constava apenas no parágrafo CAL removido e estava ausente do restante do manuscrito (Art.~14 e a type law já estavam no Corolário) | paper1/section4_theorem.tex:253–256 |
| — | Contagem de palavras final | 5.938 / 6.000 (pandoc, `main.tex` com `\input` resolvido) |
| — | Verificação soares2026b (R4b, 14 set 2026) | `github.com/danzeroum/BuildToValueGovernance` verificado **público e acessível sem autenticação** (HTTP 200; API GitHub `private:false`; branch default `main`). Conteúdo confere com a entrada bib: framework BTV — interceptação de chamadas LLM, validação LGPD/GDPR/EU AI Act, evidência criptográfica HMAC. A entrada não fixa branch/tag/commit específico. |

---

## OS-01..OS-13 traceability (COMSI-2026-04-0112, Rev 5)

| Ordern | Achado | Artefato gerado | Evidência executável |
|---|---|---|---|
| OS-01 | F1 | `seal()` unificada em `btv-core/src/lib.rs` | `record_seal_roundtrip` + 6 tamper + `seal_is_unambiguous` |
| OS-02 | F2 | `ComplianceToken` assinado; `Verdict::new -> Result` | `forged_token_signature_rejected`, `rogue_authority_token_rejected`; clippy `-D warnings` limpo |
| OS-03 | F3 | append-only `LogSink` + `LogConflict` | `btv-core/tests/test_append_only.rs` |
| OS-04 | F4 | ρ derivado; E[fine] do corpus; crédito corrigido; C_fixed com fontes | `python3 scripts/compute_crossover.py`; CI: consistência interna |
| OS-05 | F5 | `reports/tcb_summary.md` gerado por `scripts/gen_tcb_summary.py` | `python3 scripts/verify_reports.py` na CI |
| OS-06 | F6 | `drop` no fail-secure | `reports/rss_probe_fail_secure.txt` (delta 0 kB/200k); `tests/ui/token_reuse_after_fail_secure.rs` (E0382) |
| OS-07 | F7,F10 | `full_pipeline_durable` (disco real) + `status_quo_digest_log` | `data/sweep_raw_20260914T231611Z_chunked5s.csv` + fingerprint (5 modos, 5 trials) |
| OS-08 | F8,F9 | testes não escrevem em `reports/`; n_threads por env; arquitetura por target triple; vazão = ops/wall_clock | `scripts/collect_load_stats.sh` → `reports/load_stats.csv` + fingerprint |
| OS-09 | H4,H5,H6 | btv-core canônico; paper1 re-export; 17 cláusulas; `rust-toolchain.toml` | `grep -c "fn clause_" btv-core/src/lib.rs btv-core/tests/test_proof_clauses.rs` == 6+11 == 17 == §1 == §7; ambos os arquivos agora executados na CI (`--test test_proof_clauses` adicionado pós-merge PR#4→PR#5, junto com `--test test_append_only`, que também nunca era executado) |
| OS-10 | A2 | §5 reescrita dos CSVs; tabelas geradas | `scripts/gen_section5_tables.py` → `paper1/section5_tables_generated.tex` |
| OS-11 | A4 | claims moderados; abstract 149 palavras | `scripts/count_words.py` (PASS); grep de frases banidas vazio |
| OS-12 | F11 | 4 refs de R2 com DOIs verificados; §2.3 reescopada; §3 afim×linear | `grep -c "^@" paper1/refs.bib` == 15 (pós-merge PR#4→PR#5: 3 chaves duplicadas removidas, `jain1985p2` religada a `\cite`); todas as 15 citadas, zero duplicatas |
| OS-13 | D1–D4 | `docs/RESPONSE-LETTER.md` (única carta desde G7, Rodada 2 — `docs/RESPONSE-LETTER-COMSI-2026-04-0112.md` foi consolidada nela e removida) | carta ponto a ponto com arquivo:linha e hash |

## G1–G8 traceability (COMSI-2026-04-0112, Rodada 2)

| Ordem | Achado | Artefato gerado | Evidência executável |
|---|---|---|---|
| G8 | segurança | escalado ao autor — não é uma ação de commit | rotação do PAT no próprio GitHub (pendente, ver obrigações permanentes acima) |
| G6 | CI vermelha no `main` | componentes explícitos e sequenciais nos 3 jobs de CI que rodam cargo | run verde em `753c7f3` (4/4 jobs, nenhum skipped) |
| G3 | `cargo test --workspace` falhava | 2 fixtures `.stderr` de `paper2` regeneradas (diff de 1 linha `note:`, não semântico) | `cargo test --workspace --features test-support` == exit 0 |
| G4 | `cargo clippy --workspace` falhava; README implicava escopo workspace-wide | 2 lints reais corrigidos em `paper1/src/main.rs`; ~40 restantes (paper1 examples, paper2, btv-python) divulgados, não forçados; README escopado | `cargo clippy -p btv-core ...` == exit 0 (o portão que a CI de fato roda) |
| G5 | benchmark durável media replay idempotente da OS-03, não escrita real | nonce global (`AtomicU64`) em `sweep_concurrent.rs`; `SqliteLogSink::count_rows()`; gate `assert_eq!` ao final do run | `expected_rows == actual_rows` no fim do sweep |
| G2 | `corpus_median` de BR-LGPD nunca tocou o corpus (`regime` CSV="BR" vs YAML="BR_LGPD") | CSV normalizado para `BR_LGPD`; gate estendido em `compute_crossover.py` | `python3 scripts/compute_crossover.py` imprime `Corpus provenance: OK` |
| G7 | duas cartas-resposta; "Part R1" numerava R1.1–R1.3 sem fonte verificada | consolidadas em `docs/RESPONSE-LETTER.md`; numeração R1.x removida | uma única carta no repositório |
| G1 | tag `comsi-2026-04-0112-r1` (`abb7ca2`) 20+ arquivos atrás do `main` | — | `git diff --stat <tag> HEAD` (pendente — ver nota G1 na carta-resposta) |
