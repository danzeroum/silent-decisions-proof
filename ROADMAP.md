# BTV Accountability Stack — Roadmap

> Last updated: March 2026

## Tetralogy Status

| Paper | Title | Venue | Status | Blocker |
|---|---|---|---|---|
| P1 | Silent Decisions Are Type Errors | IEEE Computer | ✅ Ready to submit | None |
| P2 | BTV-Transparency (persistence + 13 tests) | CCS / USENIX | ✅ Ready to submit | Venue opening |
| P3 | Accountable Redaction (ZK + Noir circuit) | PoPETs 2027 | ✅ Sections + circuit complete | **None** — bbup auto-resolves bb/nargo version |
| P4 | What Does Verifiable AI Accountability Cost? | FAccT 2027 | 🔧 6/8 sections written | §2 desk research + §4 fieldwork (N=12 interviews) |

## Submission Timeline

| Date | Action |
|---|---|
| **April 13, 2026** | Submit P1 → IEEE Computer |
| **April–May 2026** | Submit P2 when venue CFP opens |
| **Now (P3)** | Run `bbup` + `./bench_circuit.sh` → update `test_results.txt` with real proof time + size |
| **May–September 2026** | P4 §2: desk research (Obermeyer 2019, Amazon hiring, Meta content moderation) |
| **May–September 2026** | P4 §4: schedule and conduct N=12 compliance officer / DPO interviews (Brazil + EU) |
| **January 2027** | Submit P4 → FAccT 2027 |

## Open Technical Blockers

### P3 — ~~Barretenberg / Nargo compatibility~~ RESOLVED

`bbup` automatically resolves the compatible `bb` version for the installed Nargo:

```bash
# Install bbup
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/refs/heads/master/barretenberg/bbup/install | bash
source ~/.bashrc

# bbup queries nargo version and pulls the matching bb automatically
bbup
bb --version

# Run proof + benchmark
cd paper3/circuits
nargo prove
nargo verify
./scripts/bench_circuit.sh
```

When complete: update `paper3/circuits/test_results.txt` with real proof generation
time and proof size, then P3 is fully ready for submission.

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
