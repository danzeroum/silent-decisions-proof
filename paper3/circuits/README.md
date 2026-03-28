# Accountable Redaction — ZK Circuit

Test circuit for Paper 3.

## Quick start

```bash
# Install Noir
curl -L noirup.dev | bash && noirup

# Run all tests
cd paper3/circuits
bash scripts/bench_circuit.sh
```

## What the tests prove

| Command | What it proves | Expected |
|---|---|---|
| `nargo check` | Circuit compiles | exit 0 |
| `nargo info` | Reports real constraint count | Number reported |
| `nargo prove` | Valid batch generates proof | `proof` file created |
| `nargo verify` | Proof verifies | exit 0 |
| `nargo prove` with ε=0 | Invalid batch rejected (soundness) | exit ≠ 0 |

## Test scenario

- 100 decisions, 2 groups (50 each), 60% approval rate
- Redact 2 entries (indices 10,11): group 0, both approvals
- Before: group 0 = 30/50 = 0.600, group 1 = 30/50 = 0.600
- After: group 0 = 28/48 = 0.583, group 1 = 30/50 = 0.600
- Δ = 0.0167 < ε = 0.05 → valid ✓

## Note on Prover.toml

Merkle roots and Pedersen commitments are placeholders (`"0"`).
For the circuit to pass `nargo prove`, the Noir `pedersen_hash`
must compute the actual values. The analyst should:

1. First run `nargo check` to confirm compilation
2. Then compute the actual commitments by running a helper
   Noir program or updating the values after a first proving attempt

The statistical logic (Phase 2-3) is independent of the Merkle/commitment
values and can be tested by temporarily commenting out Phase 1 and Phase 4 asserts.
