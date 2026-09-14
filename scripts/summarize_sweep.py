#!/usr/bin/env python3
"""
OS-10 (COMSI-2026-04-0112, closes A2) — summarize the committed sweep CSVs
into the exact median/CV numbers section5_benchmarks.tex cites.

Every numeric cell in the rewritten Section 5 must trace to a row this
script prints, which itself traces to a committed CSV in data/. Run it
and copy the printed values into the manuscript by hand (no LaTeX
auto-generation harness exists yet) rather than retyping numbers from
memory.

Environments (see docs/EVIDENCE-MANIFEST.md and data/sweep_env_*.txt):
  A = sandbox, Intel Xeon (Model 173) 2 vCPU KVM, 90 configs, 1s/config
  B = hosted runner, AMD EPYC 9V74 4 vCPU (Azure), 135 configs, 10s/config
  C = this round, Intel Xeon 4 vCPU KVM, 225 configs (5 modes), 2s/config
"""
import csv
import statistics
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DATA = REPO_ROOT / "data"

ENVS = {
    "A (Xeon 2vCPU sandbox)": DATA / "sweep_raw_20260914T171258Z.csv",
    "B (EPYC 4vCPU hosted-runner)": DATA / "sweep_raw_20260914T164735Z_runnervmlun5p.csv",
    "C (Xeon 4vCPU, 5-mode)": DATA / "sweep_raw_20260914T231541Z.csv",
}


def load(path):
    with path.open() as f:
        return list(csv.DictReader(f))


def group(rows):
    g = {}
    for r in rows:
        key = (r["mode"], int(r["threads"]), int(r["payload_bytes"]))
        g.setdefault(key, []).append(r)
    return g


def cv(values):
    m = statistics.mean(values)
    if m == 0:
        return 0.0
    return (statistics.pstdev(values) / m) * 100.0


def main():
    for env_name, path in ENVS.items():
        if not path.exists():
            print(f"MISSING: {path}")
            continue
        rows = load(path)
        g = group(rows)
        print(f"\n=== {env_name} ({path.name}) ===")
        for (mode, threads, payload), trials in sorted(g.items()):
            mean_ns = [float(t["mean_ns"]) for t in trials]
            p95_ns = [float(t["p95_ns"]) for t in trials]
            throughput = [float(t["throughput_ops_s"]) for t in trials]
            print(
                f"  mode={mode:22s} threads={threads} payload={payload:5d}B  "
                f"n={len(trials)}  "
                f"median_mean_ns={statistics.median(mean_ns):10.1f}  "
                f"CV%={cv(mean_ns):5.2f}  "
                f"median_p95_ns={statistics.median(p95_ns):10.1f}  "
                f"median_throughput_ops_s={statistics.median(throughput):12.1f}"
            )


if __name__ == "__main__":
    main()
