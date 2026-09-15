#!/usr/bin/env python3
"""
gen_section5_tables.py — generate every numeric table of manuscript §5
directly from committed sweep CSVs (OS-10, closes A2).

Rule enforced by construction: NO number in §5 is hand-written. Each table
is emitted here from the CSVs below, and the caption cites the exact file;
the gate is "every numeric cell of §5 traces to a row of a committed CSV".

Inputs (committed):
  - data/sweep_raw_20260914T164735Z_runnervmlun5p.csv   (AMD EPYC 9V74, 4 vCPU, 10 s target)
  - data/sweep_raw_20260914T171258Z.csv                 (Intel Xeon, 2 vCPU, 1 s target)
  - data/sweep_raw_20260915T024430Z_g5fix.csv           (Intel Xeon, 4 vCPU, 5 s target, 5 modes —
    COMSI-2026-04-0112 Round 2 G5: recollected after fixing sweep_concurrent.rs's durable-mode
    payload-nonce collision, which previously let OS-03's append-only idempotency absorb repeat
    trials/threads as no-op replays instead of real writes. Supersedes
    data/sweep_raw_20260914T231611Z_chunked5s.csv, whose durable-mode row measured that no-op
    path — see docs/RESPONSE-LETTER.md Part F, G5, for the full account.)

Outputs:
  - paper1/section5_tables_generated.tex (\\input{} by section5_benchmarks.tex)

Statistics: median across the 5 trials of each (mode, payload, threads)
configuration; CV% = stdev/mean of the trial medians' p50 values (the
harness reports per-trial aggregates, so CV quantifies trial-to-trial
stability, matching the EVIDENCE-MANIFEST practice).

Epistemic footer: both platform runs are REDUCED-footprint VM snapshots
(10 s and 1 s wall targets), NOT the 90 s dedicated-hardware headline
collection the EVIDENCE-MANIFEST still lists as due; §5.1 must say so.
Percentiles are P² estimates (Jain & Chlamtac 1985), not order statistics.
"""

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
OUT = REPO_ROOT / "paper1" / "section5_tables_generated.tex"

PLATFORMS = [
    ("EPYC-9V74-4vCPU", "data/sweep_raw_20260914T164735Z_runnervmlun5p.csv"),
    ("Xeon-2vCPU", "data/sweep_raw_20260914T171258Z.csv"),
]
FIVE_MODE = "data/sweep_raw_20260915T024430Z_g5fix.csv"

MODE_LABELS = {
    "full_pipeline": "BTV full pipeline (RAM sink)",
    "full_pipeline_durable": "BTV full pipeline (durable SQLite, WAL+FULL)",
    "verdict_only": "BTV verdict construction only",
    "status_quo_async_log": "Status quo: full-context JSON, fire-and-forget",
    "status_quo_digest_log": "Status quo: digest+metadata log",
}


def load(path: str) -> list[dict]:
    return list(csv.DictReader(open(REPO_ROOT / path)))


def med_cv(rows: list[dict], key: str) -> tuple[float, float]:
    vals = [float(r[key]) for r in rows]
    med = statistics.median(vals)
    mean = statistics.fmean(vals)
    cv = statistics.stdev(vals) / mean * 100 if len(vals) > 1 and mean else 0.0
    return med, cv


def group(rows: list[dict]) -> dict:
    g = defaultdict(list)
    for r in rows:
        g[(r["mode"], int(r["payload_bytes"]), int(r["threads"]))].append(r)
    return g


def main() -> int:
    out = []
    w = out.append

    # ── Table 1: two platforms, full_pipeline, 4 KiB, thread scaling ────────
    w("% ── Table: two-platform comparison (generated; do not edit) ──")
    w("\\begin{table}[t]")
    w("\\caption{Thread scaling of the BTV full pipeline at 4~KiB payloads,")
    w("two virtualized platforms. Each cell is the MEDIAN across five trials")
    w("of the per-trial aggregate; CV\\% is the trial-to-trial coefficient of")
    w("variation of the mean latency. Percentiles are $P^2$ estimates")
    w("(Jain \\& Chlamtac 1985), not order statistics. Provenance:")
    w("\\texttt{data/sweep\\_raw\\_20260914T164735Z\\_runnervmlun5p.csv} (EPYC,")
    w("10\\,s target) and")
    w("\\texttt{data/sweep\\_raw\\_20260914T171258Z.csv} (Xeon, 1\\,s target).}")
    w("\\label{tab:platforms}")
    w("\\begin{tabular}{lrrrrrr}")
    w("\\hline")
    w(" & \\multicolumn{3}{c}{EPYC 9V74 (4 vCPU, 10\\,s target)} & "
      "\\multicolumn{3}{c}{Xeon (2 vCPU, 1\\,s target)}\\\\")
    w("Threads & p50 (\\textmu s) & p99 (\\textmu s) & CV\\% & "
      "p50 (\\textmu s) & p99 (\\textmu s) & CV\\%\\\\")
    w("\\hline")
    epyc = group(load(PLATFORMS[0][1]))
    xeon = group(load(PLATFORMS[1][1]))
    thread_counts = sorted({t for (_, _, t) in epyc} | {t for (_, _, t) in xeon})
    for t in thread_counts:
        cells = []
        for g in (epyc, xeon):
            rows = g.get(("full_pipeline", 4096, t))
            if not rows:
                cells += ["--", "--", "--"]
                continue
            p50, cv50 = med_cv(rows, "p50_ns")
            p99, _ = med_cv(rows, "p99_ns")
            cells += [f"{p50/1e3:.2f}", f"{p99/1e3:.2f}", f"{cv50:.1f}"]
        w(f"{t} & {cells[0]} & {cells[1]} & {cells[2]} & {cells[3]} & {cells[4]} & {cells[5]}\\\\")
    w("\\hline")
    w("\\end{tabular}")
    w("\\end{table}")
    w("")

    # ── Table 2: five-mode contrast (OS-07), 4 KiB, 1 thread ────────────────
    fm = group(load(FIVE_MODE))
    w("% ── Table: five-mode contrast (generated; do not edit) ──")
    w("\\begin{table}[t]")
    w("\\caption{Five-mode accountability contrast at 4~KiB payloads, one")
    w("thread (Xeon 4~vCPU, 5\\,s target; reduced-footprint snapshot). Each")
    w("cell is the median across five trials; percentiles are $P^2$")
    w("estimates. The durable arm uses a real on-disk SQLite log (WAL,")
    w("\\texttt{synchronous=FULL}); its container-storage numbers are a")
    w("LOWER bound on bare-metal fsync cost. Every durable-mode operation")
    w("issued a real row (end-of-run sanity gate: rows persisted == operations")
    w("issued), closing a prior round's silent idempotent-replay defect")
    w("(COMSI-2026-04-0112 Round 2, G5). Provenance:")
    w("\\texttt{data/sweep\\_raw\\_20260915T024430Z\\_g5fix.csv}.}")
    w("\\label{tab:fivemode}")
    w("\\begin{tabular}{lrrr}")
    w("\\hline")
    w("Mode & p50 (\\textmu s) & p99 (\\textmu s) & throughput (k ops/s)\\\\")
    w("\\hline")
    order = [
        "full_pipeline",
        "full_pipeline_durable",
        "verdict_only",
        "status_quo_digest_log",
        "status_quo_async_log",
    ]
    values = {}
    for mode in order:
        rows = fm.get((mode, 4096, 1))
        if not rows:
            continue
        p50, _ = med_cv(rows, "p50_ns")
        p99, _ = med_cv(rows, "p99_ns")
        thr, _ = med_cv(rows, "throughput_ops_s")
        values[mode] = (p50, thr)
        w(f"{MODE_LABELS[mode]} & {p50/1e3:.2f} & {p99/1e3:.2f} & {thr/1e3:.1f}\\\\")
    w("\\hline")
    w("\\end{tabular}")
    w("\\end{table}")
    w("")

    # Contrast sentence numbers (also computed, not hand-written).
    #
    # COMSI-2026-04-0112 Round 2 (G5): this used to hardcode "FASTER" for
    # the async-log comparison and "SLOWER" for the digest-log one,
    # because at the time the durable-mode data happened to come out that
    # way. That data was wrong (a payload-nonce collision let OS-03's
    # append-only idempotency absorb most durable "writes" as no-op
    # replays), and once fixed the direction inverted — durable is now
    # slower than BOTH baselines. Hardcoding the word instead of deriving
    # it from the sign of the ratio is exactly how a wrong number ships
    # with a description that still sounds right: fixed to report
    # whichever direction the fresh ratio actually shows, for both
    # comparisons, every time this script runs.
    def describe_ratio(numerator_label: str, num: float, denom_label: str, denom: float) -> str:
        ratio = num / denom
        if ratio >= 1:
            return f"{numerator_label} is {ratio:.1f}x SLOWER than {denom_label}"
        return f"{numerator_label} is {1 / ratio:.1f}x FASTER than {denom_label}"

    durable_vs_async = None
    durable_vs_digest = None
    if "full_pipeline_durable" in values and "status_quo_async_log" in values:
        durable_vs_async = describe_ratio(
            "BTV-durable", values["full_pipeline_durable"][0],
            "the full-context status quo", values["status_quo_async_log"][0],
        )
    if "full_pipeline_durable" in values and "status_quo_digest_log" in values:
        durable_vs_digest = describe_ratio(
            "BTV-durable", values["full_pipeline_durable"][0],
            "the digest-only status quo", values["status_quo_digest_log"][0],
        )

    OUT.write_text("\n".join(out) + "\n")
    print(f"Wrote {OUT}", file=sys.stderr)
    if durable_vs_async and durable_vs_digest:
        print(
            f"Contrast @4KiB/1t: {durable_vs_async}; {durable_vs_digest}.",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
