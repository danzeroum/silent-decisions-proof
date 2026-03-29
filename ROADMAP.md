# BTV Accountability Stack — Roadmap

> Last updated: March 2026

## Tetralogy Status

| Paper | Title | Venue | Status | Blocker |
|---|---|---|---|---|
| P1 | Silent Decisions Are Type Errors | IEEE Computer | ✅ Ready to submit | None |
| P2 | BTV-Transparency (persistence + 13 tests) | CCS / USENIX | ✅ Ready to submit | Venue opening |
| P3 | Accountable Redaction (ZK + Noir circuit) | PoPETs 2027 | ✅ **Ready to submit** | None — CI green, proof verified |
| P4 | What Does Verifiable AI Accountability Cost? | FAccT 2027 | 🔧 6/8 sections written | §2 desk research + §4 fieldwork (N=12 interviews) |

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
| **April–May 2026** | Submit P2 when venue CFP opens |
| **April 2026** | Submit P3 → PoPETs 2027 (CI green ✅) |
| **May–September 2026** | P4 §2: desk research (Obermeyer 2019, Amazon hiring, Meta content moderation) |
| **May–September 2026** | P4 §4: schedule and conduct N=12 compliance officer / DPO interviews (Brazil + EU) |
| **January 2027** | Submit P4 → FAccT 2027 |

## Open Technical Blockers

### P3 — RESOLVED ✅
CI workflow green. Proof generated and verified on GitHub Actions.
See `.github/workflows/zk-proof.yml` and `paper3/circuits/test_results.txt`.

### P4 — Empirical data collection
- §2 (`section2_voluntary_failure.tex`): desk research on documented cases of opacity in AI systems
  - Obermeyer et al. 2019 (healthcare risk scores)
  - Amazon hiring algorithm (Reuters 2018)
  - Meta content moderation opacity (2021–2023)
- §4 (`section4_interviews.tex`): semi-structured interviews
  - Protocol documented in stub
  - Target: 6 financial services + 4 healthcare + 2 public sector
  - Geography: Brazil (7) + EU (5)
  - Estimated duration: 45–60 min each

## Repository Structure

```
silent-decisions-proof/
├── .github/workflows/
│   └── zk-proof.yml     # CI: compile + prove + verify (UltraHonk)
├── paper1/              # P1 — IEEE Computer
├── paper2/              # P2 — CCS/USENIX
├── paper3/              # P3 — PoPETs 2027
│   └── circuits/
│       ├── src/main.nr          # Noir ZK circuit
│       ├── Prover.toml          # test inputs (Δ=0.0167 < ε=0.05)
│       └── test_results.txt     # real proof metrics from CI
└── paper4/              # P4 — FAccT 2027
```
