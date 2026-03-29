# BTV Accountability Stack — Roadmap

> Last updated: March 2026

## Tetralogy Status

| Paper | Title | Venue | Status | Blocker |
|---|---|---|---|---|
| P1 | Silent Decisions Are Type Errors | IEEE Computer | ✅ Ready to submit | None |
| P2 | BTV-Transparency (persistence + 13 tests) | CCS / USENIX | ✅ Ready to submit | Venue opening |
| P3 | Accountable Redaction (ZK + Noir circuit) | PoPETs 2027 | ✅ Sections + circuit complete | `bb prove` / Barretenberg compat with Nargo 1.0 |
| P4 | What Does Verifiable AI Accountability Cost? | FAccT 2027 | 🔧 6/8 sections written | §2 desk research + §4 fieldwork (N=12 interviews) |

## Submission Timeline

| Date | Action |
|---|---|
| **April 13, 2026** | Submit P1 → IEEE Computer |
| **April–May 2026** | Submit P2 when venue CFP opens |
| **May–September 2026** | P4 §2: desk research (Obermeyer 2019, Amazon hiring, Meta content moderation) |
| **May–September 2026** | P4 §4: schedule and conduct N=12 compliance officer / DPO interviews (Brazil + EU) |
| **When available** | Run `bb prove` / `bb verify` on P3 Noir circuit once Barretenberg is compatible with Nargo 1.0 |
| **January 2027** | Submit P4 → FAccT 2027 |

## Open Technical Blockers

### P3 — Barretenberg / Nargo compatibility
- Circuit: `paper3/circuits/src/main.nr`
- Test inputs: `paper3/circuits/Prover.toml` (Δ=0.0167 < ε=0.05)
- Script: `paper3/circuits/scripts/bench_circuit.sh`
- Blocked on: `barretenberg` proving backend compatible with Nargo ≥ 1.0
- Action: re-run `./bench_circuit.sh` when Aztec releases compatible bb build

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
├── paper1/          # P1 — IEEE Computer
├── paper2/          # P2 — CCS/USENIX
├── paper3/          # P3 — PoPETs 2027
│   └── circuits/    # Noir ZK circuit + bench script
└── paper4/          # P4 — FAccT 2027
```
