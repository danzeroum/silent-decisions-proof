#!/bin/bash
# bench_circuit.sh — Test and benchmark the Accountable Redaction ZK circuit
# 
# Prerequisites:
#   curl -L noirup.dev | bash && noirup
#
# Usage:
#   cd paper3/circuits
#   bash ../scripts/bench_circuit.sh

set -euo pipefail

echo "=== Accountable Redaction — ZK Circuit Tests ==="
echo ""

# Step 1: Check Noir installation
if ! command -v nargo &> /dev/null; then
    echo "ERROR: nargo not found. Install via: curl -L noirup.dev | bash && noirup"
    exit 1
fi
echo "[1/6] Noir version: $(nargo --version)"

# Step 2: Compile circuit
echo ""
echo "[2/6] Compiling circuit (nargo check)..."
time nargo check
echo "       ✓ Circuit compiles"

# Step 3: Report constraint count
echo ""
echo "[3/6] Constraint count (nargo info)..."
nargo info 2>&1 | tee /tmp/nargo_info.txt
echo ""

# Step 4: Generate proof (valid batch — should succeed)
echo "[4/6] Generating proof for VALID redaction batch (nargo prove)..."
time nargo prove
echo "       ✓ Proof generated"

# Step 5: Verify proof
echo ""
echo "[5/6] Verifying proof (nargo verify)..."
time nargo verify
echo "       ✓ Proof verified"

# Step 6: Test soundness — modify Prover.toml to create invalid batch
echo ""
echo "[6/6] Soundness test: attempting proof with ε-violating batch..."
# Backup valid Prover.toml
cp Prover.toml Prover.toml.bak

# Create invalid scenario: set epsilon to 0 (no tolerance)
# The current batch has Δ=0.0167, so ε=0 should fail
sed -i 's/epsilon_scaled = 500/epsilon_scaled = 0/' Prover.toml

if nargo prove 2>/dev/null; then
    echo "       ✗ FAIL — proof should NOT have succeeded with ε=0"
    SOUNDNESS="FAIL"
else
    echo "       ✓ PASS — proof correctly rejected (ε=0 violated)"
    SOUNDNESS="PASS"
fi

# Restore valid Prover.toml
mv Prover.toml.bak Prover.toml

# Summary
echo ""
echo "=== SUMMARY ==="
echo "Compilation:     PASS"
echo "Proof gen:       PASS"
echo "Verification:    PASS"
echo "Soundness:       $SOUNDNESS"
echo ""
echo "Constraint count: $(grep -oP 'circuit size: \K\d+' /tmp/nargo_info.txt || echo 'see nargo info output above')"
echo ""
echo "NOTE: Prover.toml uses placeholder values for Merkle roots and"
echo "Pedersen commitments. For production benchmarks, these must be"
echo "computed from the actual pedersen_hash implementation in Noir."
