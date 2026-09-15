#!/usr/bin/env python3
"""
gen_section5_tables.py — generate every numeric table of manuscript §5
directly from committed sweep CSVs (OS-10, closes A2).

Rule enforced by construction: NO number in §5 is hand-written. Each table
is emitted here from the CSVs below, and the caption cites the exact file;
the gate is "every numeric cell of §5 traces to a row of a committed CSV".

Inputs (committed):
  - data/sweep_raw_20260915T024430Z_g5fix.csv           (Intel Xeon, 4 vCPU, 5 s target, 5 modes —
    THE ONLY dataset §5 quotes. Every table below comes from this one file, so the three tables
    are mutually consistent by construction: Table 1's Criterion figure for Verdict::new, Table 2's
    full-pipeline row and Table 3's five-mode contrast all describe the same code on the same host.
    COMSI-2026-04-0112 Round 2 G5: recollected after fixing sweep_concurrent.rs's durable-mode
    payload-nonce collision, which previously let OS-03's append-only idempotency absorb repeat
    trials/threads as no-op replays instead of real writes. Supersedes
    data/sweep_raw_20260914T231611Z_chunked5s.csv, whose durable-mode row measured that no-op
    path — see docs/RESPONSE-LETTER.md Part F, G5, for the full account.)

Committed but NOT quoted by §5 (retained as provenance only):
  - data/sweep_raw_20260914T164735Z_runnervmlun5p.csv   (AMD EPYC 9V74, 4 vCPU, 10 s target)
  - data/sweep_raw_20260914T171258Z.csv                 (Intel Xeon, 2 vCPU, 1 s target)
  Both predate OS-02's authority-signature verification inside Verdict::new, so their latency
  figures measure strictly less work than the current code and are not comparable with the tables
  below. A second-platform collection with the current code is due alongside the 90 s run.

Outputs:
  - paper1/section5_tables_generated.tex (\\input{} by section5_benchmarks.tex)

Statistics: median across the 5 trials of each (mode, payload, threads)
configuration; CV% = stdev/mean of the trial medians' p50 values (the
harness reports per-trial aggregates, so CV quantifies trial-to-trial
stability, matching the EVIDENCE-MANIFEST practice).

Epistemic footer: the quoted collection is a REDUCED-footprint VM snapshot
(5 s wall target on containerized overlayfs), NOT the 90 s dedicated-hardware
headline collection the EVIDENCE-MANIFEST still lists as due; §5.1 must say so.
Percentiles are P² estimates (Jain & Chlamtac 1985), not order statistics.
"""

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
OUT = REPO_ROOT / "paper1" / "section5_tables_generated.tex"

# Retained for provenance; deliberately NOT read by any table (see docstring).
SUPERSEDED_PLATFORMS = [
    ("EPYC-9V74-4vCPU", "data/sweep_raw_20260914T164735Z_runnervmlun5p.csv"),
    ("Xeon-2vCPU", "data/sweep_raw_20260914T171258Z.csv"),
]
FIVE_MODE = "data/sweep_raw_20260915T024430Z_g5fix.csv"

# Labels are deliberately terse: IEEEtran's column measure is ~3.5 in, and
# the descriptive forms these replace pushed the table 99.7 pt past the
# column edge (COMSI-2026-04-0112 Round 3). The expansion lives in the
# caption, where it costs nothing.
MODE_LABELS = {
    "full_pipeline": "BTV, in-memory sink",
    "full_pipeline_durable": "BTV, durable SQLite",
    "verdict_only": "BTV, prebuilt binding",
    "status_quo_async_log": "Status quo, full context",
    "status_quo_digest_log": "Status quo, digest only",
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

    # ── Table 1: thread scaling, RAM-sink vs durable arm, 4 KiB ─────────────
    #
    # COMSI-2026-04-0112 Round 3 (editorial closure): this table used to
    # compare two OLDER platform snapshots
    # (sweep_raw_20260914T164735Z_runnervmlun5p.csv, EPYC 4 vCPU; and
    # sweep_raw_20260914T171258Z.csv, Xeon 2 vCPU). Both were collected
    # BEFORE OS-02 made Verdict::new verify the compliance token's
    # authority signature, so their full-pipeline figures (1.72 us at 4 KiB)
    # measure strictly less work than the current code and sit 4.5x BELOW
    # Table~\ref{tab:construction}'s Criterion measurement of the same
    # operation (7.65 us) in the same manuscript. Publishing a component
    # that costs more than the pipeline containing it is a contradiction a
    # reader hits without running anything. The thread-scaling story is
    # therefore told from the headline collection, which is internally
    # consistent with both other tables; the two older snapshots remain
    # committed as provenance, and §5.1 states that a second-platform
    # re-collection with the current code is due alongside the 90 s run.
    fm = group(load(FIVE_MODE))
    w("% ── Table: thread scaling, both persistence postures (generated; do not edit) ──")
    # table* (spans both columns): seven data columns do not fit IEEEtran's
    # ~3.5 in single-column measure even at \small — it overflowed by 44 pt.
    w("\\begin{table*}[t]")
    w("\\caption{Thread scaling at 4~KiB payloads, in-memory versus durable")
    w("persistence. Each cell is the MEDIAN across five trials of the")
    w("per-trial aggregate; CV\\% is the trial-to-trial coefficient of")
    w("variation. Percentiles are $P^2$ estimates (Jain \\& Chlamtac 1985),")
    w("not order statistics. Provenance:")
    w("\\texttt{data/sweep\\_raw\\_20260915T024430Z\\_g5fix.csv}")
    w("(Intel Xeon, 4~vCPU, 5\\,s target).}")
    w("\\label{tab:platforms}")
    w("\\small")
    w("\\begin{tabular}{lrrrrrr}")
    w("\\hline")
    w(" & \\multicolumn{3}{c}{In-memory sink} & "
      "\\multicolumn{3}{c}{Durable SQLite (WAL, FULL)}\\\\")
    w("Threads & p50 (\\textmu s) & p99 (\\textmu s) & CV\\% & "
      "p50 (\\textmu s) & p99 (\\textmu s) & CV\\%\\\\")
    w("\\hline")
    thread_counts = sorted({t for (_, p, t) in fm if p == 4096})
    for t in thread_counts:
        cells = []
        for mode in ("full_pipeline", "full_pipeline_durable"):
            rows = fm.get((mode, 4096, t))
            if not rows:
                cells += ["--", "--", "--"]
                continue
            p50, cv50 = med_cv(rows, "p50_ns")
            p99, _ = med_cv(rows, "p99_ns")
            cells += [f"{p50/1e3:.2f}", f"{p99/1e3:.2f}", f"{cv50:.1f}"]
        w(f"{t} & {cells[0]} & {cells[1]} & {cells[2]} & {cells[3]} & {cells[4]} & {cells[5]}\\\\")
    w("\\hline")
    w("\\end{tabular}")
    w("\\end{table*}")
    w("")

    # ── Table 2: five-mode contrast (OS-07), 4 KiB, 1 thread ────────────────
    w("% ── Table: five-mode contrast (generated; do not edit) ──")
    # table* for the same reason: the row labels plus three numeric columns
    # overflowed the single-column measure by 38 pt at \small.
    w("\\begin{table*}[t]")
    w("\\caption{Five-mode accountability contrast at 4~KiB payloads, one")
    w("thread (Xeon 4~vCPU, 5\\,s target; reduced-footprint snapshot). Each")
    w("cell is the median across five trials; percentiles are $P^2$")
    w("estimates. The durable arm uses a real on-disk SQLite log (WAL,")
    w("\\texttt{synchronous=FULL}); its container-storage numbers are a")
    w("LOWER bound on bare-metal fsync cost. Every durable-mode operation")
    w("issued a real row (end-of-run sanity gate: rows persisted == operations")
    w("issued), closing a prior round's silent idempotent-replay defect")
    w("(COMSI-2026-04-0112 Round 2, G5). Provenance:")
    w("\\texttt{data/sweep\\_raw\\_20260915T024430Z\\_g5fix.csv}. Rows:")
    w("BTV in-memory sink; BTV with durable on-disk SQLite; BTV binding with")
    w("tokens built outside the timed region; status quo digest plus metadata;")
    w("status")
    w("quo logging the full context as JSON, fire-and-forget.}")
    w("\\label{tab:fivemode}")
    w("\\small")
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
    w("\\end{table*}")
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
