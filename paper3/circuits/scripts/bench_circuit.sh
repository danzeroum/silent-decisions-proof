#!/usr/bin/env bash
# bench_circuit.sh — generate and verify ZK proof, capture timing and size
# Usage: cd paper3/circuits && ./scripts/bench_circuit.sh
#
# Prerequisites:
#   curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/refs/heads/master/barretenberg/bbup/install | bash
#   source ~/.bashrc && bbup   # auto-resolves bb version compatible with installed nargo

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CIRCUIT_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_FILE="$CIRCUIT_DIR/test_results.txt"

cd "$CIRCUIT_DIR"

echo "=========================================" | tee -a "$RESULTS_FILE"
echo "BTV Paper 3 — ZK Circuit Benchmark" | tee -a "$RESULTS_FILE"
echo "Date: $(date -u '+%Y-%m-%d %H:%M:%S UTC')" | tee -a "$RESULTS_FILE"
echo "Nargo: $(nargo --version 2>&1)" | tee -a "$RESULTS_FILE"
echo "bb:    $(bb --version 2>&1)" | tee -a "$RESULTS_FILE"
echo "=========================================" | tee -a "$RESULTS_FILE"

# 1. Compile
echo "[1/4] Compiling circuit..." | tee -a "$RESULTS_FILE"
time nargo compile 2>&1 | tee -a "$RESULTS_FILE"

# 2. Execute (witness generation)
echo "[2/4] Generating witness..." | tee -a "$RESULTS_FILE"
time nargo execute 2>&1 | tee -a "$RESULTS_FILE"

# 3. Prove
echo "[3/4] Generating proof..." | tee -a "$RESULTS_FILE"
PROVE_START=$(date +%s%N)
bb prove -b ./target/redaction_circuit.json -w ./target/redaction_circuit.gz -o ./target/proof
PROVE_END=$(date +%s%N)
PROVE_MS=$(( (PROVE_END - PROVE_START) / 1000000 ))
PROOF_SIZE=$(wc -c < ./target/proof)
echo "  Proof generation time : ${PROVE_MS} ms" | tee -a "$RESULTS_FILE"
echo "  Proof size            : ${PROOF_SIZE} bytes" | tee -a "$RESULTS_FILE"

# 4. Verify
echo "[4/4] Verifying proof..." | tee -a "$RESULTS_FILE"
VERIFY_START=$(date +%s%N)
bb verify -k ./target/vk -p ./target/proof
VERIFY_END=$(date +%s%N)
VERIFY_MS=$(( (VERIFY_END - VERIFY_START) / 1000000 ))
echo "  Verification time     : ${VERIFY_MS} ms" | tee -a "$RESULTS_FILE"
echo "  Result                : PASS" | tee -a "$RESULTS_FILE"

echo "=========================================" | tee -a "$RESULTS_FILE"
echo "Benchmark complete. Results appended to test_results.txt" | tee -a "$RESULTS_FILE"
