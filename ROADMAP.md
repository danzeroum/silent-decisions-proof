# BTV Accountability Stack — Roadmap

> Last updated: March 2026

## Tetralogy Status

| Paper | Title | Venue | Status | Blocker |
|---|---|---|---|---|
| P1 | Silent Decisions Are Type Errors | IEEE Computer | ✅ Ready to submit | None |
| P2 | BTV-Transparency (persistence + 13 tests) | CCS / USENIX | ✅ Ready to submit | Venue opening |
| P3 | Accountable Redaction (ZK + Noir circuit) | PoPETs 2027 | ✅ Ready to submit | None — CI green |
| P4 | What Does Verifiable AI Accountability Cost? | FAccT 2027 | ✅ **Sections complete** | Verify §4 fine amounts against primary sources before submission |

## ZK Proof Benchmark (CI verified 2026-03-29)

| Metric | Value |
|---|---|
| Nargo | 1.0.0-beta.19 |
| Barretenberg (bb) | 4.0.0-nightly.20260120 |
| Scheme | UltraHonk |
| Proof generation | 697 ms |
| Proof size | 16,256 bytes (15.9 KB) |
| Verification | 19 ms |
| Result | ✅ PASS |

CI run: https://github.com/danzeroum/silent-decisions-proof/actions/runs/23699236920

## Submission Timeline

| Date | Action |
|---|---|
| **April 13, 2026** | Submit P1 → IEEE Computer |
| **April 2026** | Submit P3 → PoPETs 2027 |
| **April–May 2026** | Submit P2 when venue CFP opens |
| **Before Jan 2027** | Verify §4 fine amounts in primary sources (EDPB, SEC, ANPD) |
| **January 2027** | Submit P4 → FAccT 2027 |

## Paper 4 — Final Structure

| § | File | Type | Status |
|---|---|---|---|
| 1 | section1_introduction.tex | Argumentative | ✅ |
| 2 | section2_case_studies.tex | Desk research (Obermeyer / Amazon / Meta) | ✅ |
| 3 | section3_tco_model.tex | Math model | ✅ |
| 4 | section4_forensic_analysis.tex | Hard data (20 enforcement decisions) | ✅ verify facts |
| 5 | section5_crossover.tex | Math model | ✅ |
| 6 | section6_compliance_credit.tex | Math + legal | ✅ |
| 7 | section7_discussion.tex | Analytical | ✅ |
| 8 | section8_conclusion.tex | Synthesis | ✅ |

## Open Verification Items (P4 pre-submission)

Before submitting P4, verify the following §4 figures against primary sources:

- [ ] GDPR cases: cross-check fine amounts with [enforcementtracker.com](https://www.enforcementtracker.com)
- [ ] SEC cases: verify against [SEC litigation releases](https://www.sec.gov/litigation/litreleases.shtml)
- [ ] ANPD/CVM cases: verify against [ANPD sanctions page](https://www.gov.br/anpd/pt-br/assuntos/noticias)
- [ ] Cohen's κ inter-rater reliability: conduct actual coding exercise before submission
- [ ] `meta-settlement-analysis` citation: replace with peer-reviewed source if available

## Repository Structure

```
silent-decisions-proof/
├── .github/workflows/
│   └── zk-proof.yml         # CI: compile + prove + verify (UltraHonk)
├── paper1/                  # P1 — IEEE Computer
├── paper2/                  # P2 — CCS/USENIX
├── paper3/                  # P3 — PoPETs 2027
│   └── circuits/
│       ├── src/main.nr          # Noir ZK circuit
│       ├── Prover.toml          # test inputs (Δ=0.0167 < ε=0.05)
│       └── test_results.txt     # real proof metrics from CI
└── paper4/                  # P4 — FAccT 2027
    ├── main.tex
    ├── abstract.tex
    ├── section1_introduction.tex
    ├── section2_case_studies.tex
    ├── section3_tco_model.tex
    ├── section4_forensic_analysis.tex
    ├── section5_crossover.tex
    ├── section6_compliance_credit.tex
    ├── section7_discussion.tex
    ├── section8_conclusion.tex
    └── refs.bib
```
