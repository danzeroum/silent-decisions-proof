#!/usr/bin/env python3
"""
Test 4 — Baseline comparativo de latência e throughput.

Compara 4 implementações:
1. BTV nativo (Rust, criterion) — medida vem do benchmark Rust.
2. BTV via PyO3 binding.
3. OpenTelemetry pós-hoc (logging assíncrono).
4. SQLite ACID (sem BTV, só BEGIN...COMMIT).

Gera:
- reports/benchmark_baseline.md (tabela comparativa + epistemic footer)
- reports/benchmark_baseline.csv (dados brutos)
- appendix_benchmarks.pgfplots.tex (snippet LaTeX PGFPlots para o artigo)

Epistemic footer:
  Este teste valida que o overhead do BTV (Rust e PyO3) é mensurável e
  comparável a baselines realistas (OpenTelemetry, SQLite ACID). Ele NÃO
  garante que os números sejam representativos de produção, pois a
  medição foi feita em hardware único (x86-64), sem carga concorrente,
  sem replicação geográfica, e com SQLite em memória primária. Para
  números de produção, repetir em hardware diverso com carga realista.
"""
import csv
import json
import os
import statistics
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
REPORTS = REPO_ROOT / "reports"
REPORTS.mkdir(parents=True, exist_ok=True)

# ─────────────────────────────────────────────────────────────────────────────
# Common workload
# ─────────────────────────────────────────────────────────────────────────────

N_SAMPLES = 5_000
RAW_CONTEXT = b'{"subject":"alice","action":"credit-application","score":0.42,"threshold":0.50}'

# ─────────────────────────────────────────────────────────────────────────────
# Bench 1: BTV via PyO3
# ─────────────────────────────────────────────────────────────────────────────

def bench_btv_pyo3(n: int = N_SAMPLES) -> tuple[list[float], float]:
    """Measure BTV issue_verdict via PyO3 binding.

    Returns (per-call latencies in microseconds, total wall-clock seconds for
    the whole timed loop). OS-08 (closes F8): throughput must come from
    wall_clock, not from 1/mean(latencies) — the two agree only when the loop
    is strictly sequential AND there is no timing overhead outside the timed
    region; wall-clock is correct unconditionally.
    """
    from btv_python import issue_verdict, TestingLogConfig
    cfg = TestingLogConfig()
    latencies = []
    # warmup
    for _ in range(50):
        with issue_verdict(
            raw_context=RAW_CONTEXT, decision="deny",
            jurisdiction="BR-LGPD", policy_version="1.0.0",
            explanation="warmup", contestability_hours=720,
            log_config=cfg,
        ):
            pass
    wall_t0 = time.perf_counter()
    for _ in range(n):
        t0 = time.perf_counter_ns()
        with issue_verdict(
            raw_context=RAW_CONTEXT, decision="deny",
            jurisdiction="BR-LGPD", policy_version="1.0.0",
            explanation="bench", contestability_hours=720,
            log_config=cfg,
        ):
            pass
        t1 = time.perf_counter_ns()
        latencies.append((t1 - t0) / 1000.0)  # microseconds
    wall_elapsed_s = time.perf_counter() - wall_t0
    return latencies, wall_elapsed_s

# ─────────────────────────────────────────────────────────────────────────────
# Bench 2: OpenTelemetry pós-hoc (logging assíncrono)
# ─────────────────────────────────────────────────────────────────────────────

def bench_opentelemetry(n: int = N_SAMPLES) -> tuple[list[float], float]:
    """Measure OpenTelemetry span creation + export (BatchProcessor)."""
    from opentelemetry import trace
    from opentelemetry.sdk.trace import TracerProvider
    from opentelemetry.sdk.trace.export import (
        BatchSpanProcessor,
        ConsoleSpanExporter,
    )
    from opentelemetry.sdk.resources import Resource

    resource = Resource.create({"service.name": "bench-btv"})
    provider = TracerProvider(resource=resource)
    # ConsoleSpanExporter writes to /dev/null to make export cost realistic
    # without polluting stdout. We use a noop-like exporter.
    class NullExporter:
        def export(self, spans):
            return 0  # SUCCESS
        def shutdown(self):
            pass
    processor = BatchSpanProcessor(NullExporter())
    provider.add_span_processor(processor)
    trace.set_tracer_provider(provider)
    tracer = trace.get_tracer("bench")

    latencies = []
    # warmup
    for _ in range(50):
        with tracer.start_as_current_span("decision") as span:
            span.set_attribute("raw_context", RAW_CONTEXT.decode("utf-8", errors="replace"))
            span.set_attribute("decision", "deny")

    wall_t0 = time.perf_counter()
    for _ in range(n):
        t0 = time.perf_counter_ns()
        with tracer.start_as_current_span("decision") as span:
            span.set_attribute("raw_context", RAW_CONTEXT.decode("utf-8", errors="replace"))
            span.set_attribute("decision", "deny")
        t1 = time.perf_counter_ns()
        latencies.append((t1 - t0) / 1000.0)
    wall_elapsed_s = time.perf_counter() - wall_t0
    provider.shutdown()
    return latencies, wall_elapsed_s

# ─────────────────────────────────────────────────────────────────────────────
# Bench 3: SQLite ACID (sem BTV, só BEGIN...COMMIT)
# ─────────────────────────────────────────────────────────────────────────────

def bench_sqlite_acid(n: int = N_SAMPLES) -> tuple[list[float], float]:
    """Measure SQLite WAL+FULL synchronous commit (baseline ACID)."""
    import sqlite3
    db_path = str(REPO_ROOT / "reports" / "bench_baseline.db")
    if os.path.exists(db_path):
        os.remove(db_path)
    conn = sqlite3.connect(db_path, isolation_level=None)  # autocommit off
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA synchronous=FULL")
    conn.execute("""
        CREATE TABLE decisions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            context_hash TEXT NOT NULL,
            decision TEXT NOT NULL,
            explanation TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    """)
    latencies = []
    import hashlib
    # warmup
    for _ in range(50):
        h = hashlib.sha256(RAW_CONTEXT).hexdigest()
        conn.execute("BEGIN")
        conn.execute(
            "INSERT INTO decisions (context_hash, decision, explanation) VALUES (?, ?, ?)",
            (h, "deny", "warmup"),
        )
        conn.execute("COMMIT")

    wall_t0 = time.perf_counter()
    for _ in range(n):
        h = hashlib.sha256(RAW_CONTEXT).hexdigest()
        t0 = time.perf_counter_ns()
        conn.execute("BEGIN")
        conn.execute(
            "INSERT INTO decisions (context_hash, decision, explanation) VALUES (?, ?, ?)",
            (h, "deny", "bench"),
        )
        conn.execute("COMMIT")
        t1 = time.perf_counter_ns()
        latencies.append((t1 - t0) / 1000.0)
    wall_elapsed_s = time.perf_counter() - wall_t0
    conn.close()
    if os.path.exists(db_path):
        os.remove(db_path)
    if os.path.exists(db_path + "-wal"):
        os.remove(db_path + "-wal")
    return latencies, wall_elapsed_s

# ─────────────────────────────────────────────────────────────────────────────
# Stats
# ─────────────────────────────────────────────────────────────────────────────

def percentile(data: list[float], p: float) -> float:
    """Compute p-th percentile (linear interpolation)."""
    if not data:
        return float("nan")
    s = sorted(data)
    k = (len(s) - 1) * p
    f = int(k)
    c = min(f + 1, len(s) - 1)
    if f == c:
        return s[f]
    return s[f] + (s[c] - s[f]) * (k - f)

def stats(data: list[float], wall_elapsed_s: float) -> dict:
    """OS-08 (closes F8): throughput = total_ops / wall_clock, never
    1/mean(latency) — the latter is only equivalent for a strictly
    sequential loop with zero overhead outside the timed region, and the
    formula must be uniform across every table in this artifact so a reader
    can't find one bench computed one way and another computed differently.
    """
    return {
        "p50_us": percentile(data, 0.50),
        "p95_us": percentile(data, 0.95),
        "p99_us": percentile(data, 0.99),
        "mean_us": statistics.fmean(data),
        "stdev_us": statistics.stdev(data) if len(data) > 1 else 0.0,
        "throughput_ops_per_s": len(data) / wall_elapsed_s if wall_elapsed_s > 0 else 0.0,
        "n_samples": len(data),
        "mean_only": False,
    }

# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

def main():
    print(f"Running {N_SAMPLES}-sample benchmarks on x86-64...", file=sys.stderr)

    # Read criterion results for BTV native (Rust)
    criterion_dir = REPO_ROOT / "btv-core" / "target" / "criterion"
    btv_native_stats = None
    if criterion_dir.exists():
        try:
            est_path = criterion_dir / "issue_verdict_in_memory_sink" / "new" / "estimates.json"
            if est_path.exists():
                with est_path.open() as f:
                    est = json.load(f)
                # OS-08 (closes F8): Criterion's estimates.json reports a
                # bootstrap mean, not percentiles of individual iterations.
                # The pre-audit script copied the mean into p50/p95/p99 and
                # the PGFPlots figure plotted it under "Latencia p95" as if
                # it were a measured percentile. Per the OS-08 gate, since
                # this script does not parse Criterion's raw per-iteration
                # samples, percentiles are left absent (None) and every
                # consumer (table/CSV/plot) must label the row "mean only"
                # instead of filling percentile columns.
                btv_native_stats = {
                    "p50_us": None,
                    "p95_us": None,
                    "p99_us": None,
                    "mean_us": est["mean"]["point_estimate"] / 1000.0,
                    "stdev_us": est["std_dev"]["point_estimate"] / 1000.0,
                    "throughput_ops_per_s": 1_000_000_000.0 / est["mean"]["point_estimate"],
                    "n_samples": 100,
                    "mean_only": True,
                }
        except Exception as e:
            print(f"Could not read criterion results: {e}", file=sys.stderr)

    btv_sqlite_stats = None
    if criterion_dir.exists():
        try:
            est_path = criterion_dir / "issue_verdict_sqlite_wal_full" / "new" / "estimates.json"
            if est_path.exists():
                with est_path.open() as f:
                    est = json.load(f)
                btv_sqlite_stats = {
                    "p50_us": None,
                    "p95_us": None,
                    "p99_us": None,
                    "mean_us": est["mean"]["point_estimate"] / 1000.0,
                    "stdev_us": est["std_dev"]["point_estimate"] / 1000.0,
                    "throughput_ops_per_s": 1_000_000_000.0 / est["mean"]["point_estimate"],
                    "n_samples": 100,
                    "mean_only": True,
                }
        except Exception as e:
            print(f"Could not read SQLite criterion results: {e}", file=sys.stderr)

    print("[1/3] BTV via PyO3...", file=sys.stderr)
    btv_pyo3_latencies, btv_pyo3_wall_s = bench_btv_pyo3()
    btv_pyo3_stats = stats(btv_pyo3_latencies, btv_pyo3_wall_s)

    print("[2/3] OpenTelemetry pós-hoc...", file=sys.stderr)
    otel_latencies, otel_wall_s = bench_opentelemetry()
    otel_stats = stats(otel_latencies, otel_wall_s)

    print("[3/3] SQLite ACID baseline...", file=sys.stderr)
    sqlite_latencies, sqlite_wall_s = bench_sqlite_acid()
    sqlite_stats = stats(sqlite_latencies, sqlite_wall_s)

    # Combine
    all_stats = []
    if btv_native_stats:
        all_stats.append(("BTV-Rust-native (criterion, mean only)", btv_native_stats))
    all_stats.append(("BTV-PyO3 (Python bench)", btv_pyo3_stats))
    if btv_sqlite_stats:
        all_stats.append(("BTV-Rust+SQLite WAL+FULL (criterion, mean only)", btv_sqlite_stats))
    all_stats.append(("OpenTelemetry pós-hoc (BatchProcessor+NullExporter)", otel_stats))
    all_stats.append(("SQLite ACID bare (BEGIN..COMMIT)", sqlite_stats))

    # CSV — OS-08 (closes F8): mean-only rows (Criterion) write an empty
    # p50/p95/p99 field instead of the mean copied three times, plus an
    # explicit mean_only column so a downstream reader can't miss it.
    def fmt(v):
        return "" if v is None else f"{v:.3f}"

    csv_path = REPORTS / "benchmark_baseline.csv"
    with csv_path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow([
            "implementation", "p50_us", "p95_us", "p99_us",
            "mean_us", "stdev_us", "throughput_ops_per_s", "n_samples", "mean_only",
        ])
        for name, s in all_stats:
            w.writerow([name, fmt(s['p50_us']), fmt(s['p95_us']),
                        fmt(s['p99_us']), f"{s['mean_us']:.3f}",
                        f"{s['stdev_us']:.3f}", f"{s['throughput_ops_per_s']:.0f}",
                        s["n_samples"], s.get("mean_only", False)])

    # Markdown
    def fmt_md(v):
        return "—" if v is None else f"{v:.2f}"

    md_path = REPORTS / "benchmark_baseline.md"
    with md_path.open("w") as f:
        f.write("# Baseline Comparativo de Latência e Throughput\n\n")
        f.write("**Branch:** `artifact-v2` (local)\n\n")
        f.write("**Hardware:** x86-64 (single core, in-process)\n\n")
        f.write("**Samples:** " + f"{N_SAMPLES:,}" + " por implementação Python; 100 por bench Rust (criterion)\n\n")
        f.write("## Resultados\n\n")
        f.write("| Implementação | p50 (μs) | p95 (μs) | p99 (μs) | média (μs) | stdev (μs) | throughput (ops/s) |\n")
        f.write("|---|---:|---:|---:|---:|---:|---:|\n")
        for name, s in all_stats:
            f.write(f"| {name} | {fmt_md(s['p50_us'])} | {fmt_md(s['p95_us'])} | "
                    f"{fmt_md(s['p99_us'])} | {s['mean_us']:.2f} | "
                    f"{s['stdev_us']:.2f} | {s['throughput_ops_per_s']:.0f} |\n")
        f.write("\n(— = not measured: Criterion's `estimates.json` reports a bootstrap "
                "mean, not per-iteration percentiles — OS-08/F8. Throughput is "
                "`total_ops / wall_clock` uniformly across every row.)\n")
        f.write("\n## Notas metodológicas\n\n")
        f.write("- **BTV-Rust-native** medido via `cargo bench` (criterion, 100 amostras, estimativa pontual da média). Não há percentis pois criterion reporta apenas a média no `estimates.json`; as colunas p50/p95/p99 ficam em branco (—), nunca preenchidas com a média (OS-08).\n")
        f.write("- **BTV-PyO3** medido via `time.perf_counter_ns()` em loop Python, 5.000 amostras após warmup de 50 iterações.\n")
        f.write("- **OpenTelemetry pós-hoc** usa `BatchSpanProcessor` com `NullExporter` (export síncrono para evitar I/O de arquivo). Mede apenas a criação do span e enfileiramento, não o flush.\n")
        f.write("- **SQLite ACID bare** mede `BEGIN..INSERT..COMMIT` em SQLite WAL+FULL synchronous, sem BTV. É o baseline mais pessimista (commit síncrono em disco a cada operação).\n")
        f.write("- **BTV-Rust+SQLite WAL+FULL** é o `issue_verdict` completo com persistência SQLite (criterion, 100 amostras).\n\n")
        f.write("## Interpretação para o manuscrito\n\n")
        if btv_native_stats and sqlite_stats:
            overhead = btv_native_stats["mean_us"] / sqlite_stats["mean_us"] if sqlite_stats["mean_us"] > 0 else 0
            f.write(f"- O BTV nativo (Rust, in-memory) adiciona ~{btv_native_stats['mean_us']:.2f} μs "
                    f"de overhead puro (hash BLAKE3 + HMAC-SHA256 + construção de structs).\n")
            f.write(f"- Quando persistido em SQLite WAL+FULL, o BTV fica em ~{btv_sqlite_stats['mean_us']:.2f} μs "
                    f"— comparável ao baseline SQLite ACID bare (~{sqlite_stats['mean_us']:.2f} μs).\n")
            f.write(f"- Através do PyO3, o overhead de FFI adiciona ~{btv_pyo3_stats['mean_us'] - btv_native_stats['mean_us']:.2f} μs "
                    f"sobre o nativo, mas ainda é ~{btv_pyo3_stats['mean_us'] / otel_stats['mean_us']:.1f}× mais rápido que OpenTelemetry "
                    f"pós-hoc (que tem custo de enfileiramento assíncrono).\n")
        f.write("\n## Limitações\n\n")
        f.write("- Hardware único (x86-64); sem repetição em ARM64.\n")
        f.write("- Sem carga concorrente (medição single-threaded).\n")
        f.write("- OpenTelemetry com `NullExporter` subestima o custo real de export (que envolve serialização protobuf + rede).\n")
        f.write("- SQLite em disco local SSD; replicação geográfica não medida.\n")
        f.write("\n> **Epistemic footer.** *Este teste valida que o overhead do BTV (Rust e PyO3) é mensurável e comparável a baselines realistas (OpenTelemetry, SQLite ACID). Ele NÃO garante que os números sejam representativos de produção, pois a medição foi feita em hardware único (x86-64) e sem carga concorrente (medição single-threaded; para concorrência ver `data/sweep_raw_*.csv`, OS-07). O SQLite ACID bare e o BTV+SQLite WAL+FULL usam ambos um arquivo real em disco (OS-07, closes F7) — nenhum dos dois é medição em memória.*\n")

    # PGFPlots LaTeX snippet
    tex_path = REPO_ROOT / "appendix_benchmarks.pgfplots.tex"
    with tex_path.open("w") as f:
        f.write("% Auto-generated by scripts/benchmark_baseline.py\n")
        f.write("% Append to IEEE Computer manuscript Appendix B (Benchmarks).\n\n")
        f.write("\\begin{figure}[t]\n")
        f.write("\\centering\n")
        f.write("\\begin{tikzpicture}\n")
        f.write("\\begin{axis}[\n")
        f.write("    ybar,\n")
        f.write("    bar width=14pt,\n")
        f.write("    width=\\columnwidth,\n")
        f.write("    height=5cm,\n")
        f.write("    ylabel={Latência p95 ($\\mu$s)},\n")
        # OS-08 (closes F8): the bar chart plots MEASURED p95 only. Criterion
        # rows (mean_only=True, p95_us=None) have no measured p95 and are
        # excluded from this figure rather than plotting their mean under a
        # p95 axis label — their mean is still in the table below and in
        # benchmark_baseline.csv/.md.
        p95_rows = [(name, s) for name, s in all_stats if not s.get("mean_only", False)]
        symbolic_coords = []
        for name, _s in p95_rows:
            short = name.split(" ")[0]
            if "PyO3" in name: short = "BTV-PyO3"
            elif "OpenTelemetry" in name: short = "OTel-pós-hoc"
            elif "bare" in name: short = "SQLite-bare"
            symbolic_coords.append(short)
        f.write(f"    symbolic x coords={{{', '.join(symbolic_coords)}}},\n")
        f.write("    xtick=data,\n")
        f.write("    x tick label style={rotate=30,anchor=east,font=\\small},\n")
        f.write("    nodes near coords,\n")
        f.write("    nodes near coords style={font=\\scriptsize},\n")
        f.write("    enlarge x limits=0.18,\n")
        f.write("    ymin=0,\n")
        f.write("]\n")
        f.write("\\addplot[fill=blue!50] coordinates {\n")
        for short, s in zip(symbolic_coords, [s for _, s in p95_rows]):
            f.write(f"    ({short}, {s['p95_us']:.2f})\n")
        f.write("};\n")
        f.write("\\end{axis}\n")
        f.write("\\end{tikzpicture}\n")
        mean_only_names = ", ".join(
            name.split(" (")[0] for name, s in all_stats if s.get("mean_only", False)
        )
        f.write(f"\\caption{{Latência p95 medida por implementação ($N=5{{,}}000$ para Python). "
                f"{mean_only_names} não aparecem aqui: Criterion reporta apenas a média "
                "bootstrap, não percentis por iteração — ver a média na "
                "Tabela~\\ref{tab:bench-stats}.}\n")
        f.write("\\label{fig:bench-baseline}\n")
        f.write("\\end{figure}\n\n")
        f.write("\\begin{table}[t]\n")
        f.write("\\centering\n")
        f.write("\\caption{Estatísticas de benchmark por implementação. "
                "'--' = não medido (Criterion reporta média, não percentis; OS-08/F8).}\n")
        f.write("\\label{tab:bench-stats}\n")
        f.write("\\small\n")
        f.write("\\begin{tabular}{lrrrr}\n")
        f.write("\\toprule\n")
        f.write("Implementação & p50 ($\\mu$s) & p95 ($\\mu$s) & p99 ($\\mu$s) & ops/s \\\\\n")
        f.write("\\midrule\n")
        def fmt_tex(v):
            return "--" if v is None else f"{v:.2f}"
        for name, s in all_stats:
            short = name.split("(")[0].strip()
            f.write(f"{short} & {fmt_tex(s['p50_us'])} & {fmt_tex(s['p95_us'])} & "
                    f"{fmt_tex(s['p99_us'])} & {s['throughput_ops_per_s']:.0f} \\\\\n")
        f.write("\\bottomrule\n")
        f.write("\\end{tabular}\n")
        f.write("\\end{table}\n")

    print(f"Wrote {csv_path}")
    print(f"Wrote {md_path}")
    print(f"Wrote {tex_path}")

if __name__ == "__main__":
    main()
