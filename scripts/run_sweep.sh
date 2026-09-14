#!/usr/bin/env bash
# scripts/run_sweep.sh — Reproducible execution wrapper for sweep_concurrent.
#
# Records the full hardware/OS/toolchain fingerprint alongside the raw CSV
# so that Section 5.1 of the paper can cite this file as the exact
# provenance of every reported number.
set -euo pipefail

# Always run from the repository root: the cargo invocation, the data/
# output directory, and `git rev-parse` below all assume it.
cd "$(dirname "$0")/.."

OUT_DIR="data"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
CSV_PATH="${OUT_DIR}/sweep_raw_${TIMESTAMP}.csv"
ENV_PATH="${OUT_DIR}/sweep_env_${TIMESTAMP}.txt"

mkdir -p "${OUT_DIR}"

{
    echo "timestamp_utc: ${TIMESTAMP}"
    echo "--- rustc ---"
    rustc --version
    echo "--- cargo ---"
    cargo --version
    echo "--- kernel ---"
    uname -a
    echo "--- cpu ---"
    lscpu 2>/dev/null || sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "cpu info unavailable"
    echo "--- governor ---"
    cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "governor info unavailable (non-Linux or no cpufreq)"
    echo "--- turbo/no_turbo ---"
    cat /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || echo "turbo state unavailable"
    echo "--- git commit ---"
    git rev-parse HEAD
} > "${ENV_PATH}"

echo "Environment fingerprint written to ${ENV_PATH}"
echo "Reminder: for the numbers reported in the paper, set the CPU governor"
echo "to 'performance' and disable turbo boost BEFORE running this script:"
echo "  sudo cpupower frequency-set -g performance"
echo "  echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo"
echo "  taskset -c 0-\$(( \$(nproc) - 1 )) \$0 (re-run pinned, if not already)"
echo ""
echo "MEMORY PROFILE (Round 2): percentiles are estimated online with the"
echo "P2 algorithm (Jain & Chlamtac 1985) — O(1) memory per thread. The"
echo "harness no longer stores one Duration per iteration and no longer"
echo "prebuilds token pairs, so resident memory stays in the tens of MB"
echo "regardless of iteration count or thread count. No special RAM"
echo "precautions are needed."
echo ""
echo "Running sweep — this will take multiple hours. Output: ${CSV_PATH}"

cargo run --release --features sweep-bench --bin sweep_concurrent > "${CSV_PATH}"

echo "Sweep complete. Raw data: ${CSV_PATH}"
echo "Environment fingerprint: ${ENV_PATH}"
