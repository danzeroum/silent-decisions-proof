#!/usr/bin/env bash
# scripts/collect_load_stats.sh — produce reports/load_stats.csv WITH provenance.
#
# OS-08 (COMSI-2026-04-0112, closes F9): tests must never write into
# reports/. The ONLY path to committed load evidence is this script, which:
#   1. runs the load test (cargo test ... -- --nocapture),
#   2. extracts the [load_stats] line from the test output,
#   3. writes reports/load_stats.csv (+ the ARM64/QEMU variant when applicable),
#   4. writes a fingerprint file (command, toolchain, kernel, CPU, git commit,
#      BTV_TEST_THREADS) next to it.
#
# Usage: bash scripts/collect_load_stats.sh [threads]
#   threads defaults to BTV_TEST_THREADS if set, else 2.
set -euo pipefail
cd "$(dirname "$0")/.."

THREADS="${1:-${BTV_TEST_THREADS:-2}}"
export BTV_TEST_THREADS="${THREADS}"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"

OUT="reports/load_stats.csv"
OUT_QEMU="reports/load_stats_arm64_qemu.csv"
ARCH="$(rustc -vV | grep '^host:' | awk '{print $2}')"

echo "Running load test (BTV_TEST_THREADS=${THREADS})..."
cargo test --features test-support --test test_load -- --nocapture | tee /tmp/load_stats_run.txt

# Extract the single [load_stats] line into key=value pairs.
LINE="$(grep -o '\[load_stats\].*' /tmp/load_stats_run.txt | tail -1)"
[ -n "${LINE}" ] || { echo "no [load_stats] line found" >&2; exit 1; }

# Extract key=value, stripping the display unit suffix (e.g. "42.29us" -> 42.29).
kv() { printf '%s\n' "${LINE#\[load_stats\] }" | tr ' ' '\n' | grep "^$1=" | cut -d= -f2- | sed 's/[a-z\/]*$//'; }

P50="$(kv p50)"; P95="$(kv p95)"; P99="$(kv p99)"; MEAN="$(kv mean)"
STDEV="$(kv stdev)"; WALL_MS="$(kv wall_ms)"; THR="$(kv throughput)"; OPS="$(kv ops)"
T="$(kv threads)"

# Target-triple-based routing (no latency heuristics — OS-08).
if [[ "${ARCH}" == aarch64* ]]; then
  OUT="${OUT_QEMU}"
fi

{
  echo "# Provenance (OS-08): produced by scripts/collect_load_stats.sh"
  echo "# timestamp_utc: ${TIMESTAMP}"
  echo "# command: cargo test --features test-support --test test_load -- --nocapture (BTV_TEST_THREADS=${THREADS})"
  echo "# throughput_definition: total_ops / wall_clock (OS-08)"
  echo "# git_commit: $(git rev-parse HEAD)"
  echo "# host_triple: ${ARCH}"
  rustc --version | sed 's/^/# rustc: /'
  uname -srm | sed 's/^/# kernel: /'
  (lscpu 2>/dev/null | grep -m1 'Model name' || echo '# cpu: unavailable') | sed 's/^//'
  echo "metric,value"
  echo "arch,${ARCH}"
  echo "threads,${T}"
  echo "total_ops,${OPS}"
  echo "p50_us,${P50}"
  echo "p95_us,${P95}"
  echo "p99_us,${P99}"
  echo "mean_us,${MEAN}"
  echo "stdev_us,${STDEV}"
  echo "wall_ms,${WALL_MS}"
  echo "throughput_ops_per_s,${THR}"
  echo "throughput_definition,total_ops_per_wall_clock"
} > "${OUT}"

echo "Wrote ${OUT} (threads=${T}, arch=${ARCH})"
echo "Done. Re-run whenever the pipeline changes; the fingerprint above ties the numbers to their provenance."
