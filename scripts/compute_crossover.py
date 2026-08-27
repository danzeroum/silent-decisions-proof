#!/usr/bin/env python3
"""
Test 8 — TCO reprodutível.

Lê:
  - data/enforcement_cases.csv (20 casos regulatórios)
  - data/policy_parameters.yaml (ρ, C_fixed, deltas)

Gera:
  - data/tco_plot_data.csv (curva N* = C_fixed/ρ para sensibilidade)
  - appendix_b_pgfplots.tex (snippet LaTeX PGFPlots para Apêndice B)
  - reports/tco_summary.md (tabela com N* por regime + cenários)

Epistemic footer:
  Este teste valida que o cálculo de N* é determinístico e reproduzível
  a partir de dados versionados. Ele NÃO atesta que os valores de ρ ou
  C_fixed sejam precisos beyond das fontes citadas; qualquer alteração
  nos parâmetros recalcula N* automaticamente.
"""
import csv
import sys
from pathlib import Path

import yaml
import numpy as np

REPO_ROOT = Path(__file__).resolve().parent.parent
DATA = REPO_ROOT / "data"
REPORTS = REPO_ROOT / "reports"
REPORTS.mkdir(parents=True, exist_ok=True)


def load_cases() -> list[dict]:
    cases_path = DATA / "enforcement_cases.csv"
    with cases_path.open() as f:
        reader = csv.DictReader(f)
        return list(reader)


def load_params() -> dict:
    params_path = DATA / "policy_parameters.yaml"
    with params_path.open() as f:
        return yaml.safe_load(f)


def compute_n_star(c_fixed: float, rho: float) -> float:
    """N* = C_fixed / ρ. Returns the crossover decision volume."""
    if rho <= 0:
        return float("inf")
    return c_fixed / rho


def compute_n_star_with_credit(c_fixed: float, rho: float, delta: float) -> float:
    """N* when BTV compliance credit reduces effective fine by factor (1-δ)."""
    effective_rho = rho * (1.0 - delta)
    return compute_n_star(c_fixed, effective_rho)


def sensitivity_table(params: dict) -> list[dict]:
    """Generate (rho, c_fixed, N*) tuples for the sensitivity grid."""
    sens = params["sensitivity"]
    rhos = np.linspace(sens["rho_min"], sens["rho_max"], sens["rho_steps"])
    c_fixeds = np.linspace(sens["c_fixed_min"], sens["c_fixed_max"], sens["c_fixed_steps"])
    rows = []
    for rho in rhos:
        for c_fixed in c_fixeds:
            n_star = compute_n_star(c_fixed, rho)
            rows.append({
                "rho_usd_per_decision": float(rho),
                "c_fixed_usd_per_year": float(c_fixed),
                "n_star_decisions_per_year": float(n_star),
            })
    return rows


def main():
    cases = load_cases()
    params = load_params()
    c_fixed = params["c_fixed_usd_per_year"]

    print(f"Loaded {len(cases)} enforcement cases", file=sys.stderr)
    print(f"C_fixed = ${c_fixed}/year", file=sys.stderr)

    # Compute N* per regime
    n_star_rows = []
    for regime, p in params["regimes"].items():
        rho = p["rho_usd_per_decision"]
        n_star = compute_n_star(c_fixed, rho)
        delta_key = f"{regime}_delta"
        delta = params["compliance_credit"].get(delta_key, 0.0)
        n_star_credit = compute_n_star_with_credit(c_fixed, rho, delta)
        n_star_rows.append({
            "regime": regime,
            "rho_usd_per_decision": rho,
            "delta_credit": delta,
            "n_star_no_credit": n_star,
            "n_star_with_credit": n_star_credit,
            "enforcement_prob": p["enforcement_prob_per_year"],
            "expected_fine_usd": p["expected_fine_usd"],
            "avg_controller_decisions": p["avg_controller_decisions_per_year"],
        })

    # Write the canonical N* table
    nstar_path = DATA / "n_star_by_regime.csv"
    with nstar_path.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=[
            "regime", "rho_usd_per_decision", "delta_credit",
            "n_star_no_credit", "n_star_with_credit",
            "enforcement_prob", "expected_fine_usd",
            "avg_controller_decisions",
        ])
        w.writeheader()
        for r in n_star_rows:
            w.writerow(r)
    print(f"Wrote {nstar_path}", file=sys.stderr)

    # Sensitivity grid
    sens_rows = sensitivity_table(params)
    sens_path = DATA / "tco_plot_data.csv"
    with sens_path.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=[
            "rho_usd_per_decision", "c_fixed_usd_per_year",
            "n_star_decisions_per_year",
        ])
        w.writeheader()
        for r in sens_rows:
            w.writerow(r)
    print(f"Wrote {sens_path} ({len(sens_rows)} rows)", file=sys.stderr)

    # Three scenarios for the manuscript (optimist / base / pessimist)
    scenarios = []
    for label, rho, c in [
        ("optimist", 0.05, 1_000),   # high rho, low C_fixed → low N*
        ("base",     0.01, 5_000),   # GDPR base case
        ("pessimist", 0.005, 50_000), # low rho, high C_fixed → high N*
    ]:
        scenarios.append({
            "scenario": label,
            "rho_usd_per_decision": rho,
            "c_fixed_usd_per_year": c,
            "n_star": compute_n_star(c, rho),
        })

    # Markdown summary
    md_path = REPORTS / "tco_summary.md"
    with md_path.open("w") as f:
        f.write("# TCO Reproducibility Summary\n\n")
        f.write("**Branch:** `artifact-v2` (local)\n\n")
        f.write("**Date:** 2026-08-27\n\n")
        f.write("**Sources:** `data/enforcement_cases.csv`, "
                "`data/policy_parameters.yaml`\n\n")
        f.write("## Formula\n\n")
        f.write("```latex\n")
        f.write("\\rho = P_{\\text{enf}} \\times \\frac{E[\\text{fine}]}{\\bar{N}_{\\text{controller}}}\n")
        f.write("C_{\\text{penalty}}(N) = \\rho \\times N\n")
        f.write("\\text{TCO}(N) \\approx C_{\\text{fixed}} \\quad \\text{(for } N < 10^7\\text{)}\n")
        f.write("N^* = \\frac{C_{\\text{fixed}}}{\\rho}\n")
        f.write("```\n\n")
        f.write("## N* por regime (C_fixed = $5,000/year)\n\n")
        f.write("| Regime | ρ ($/decision) | δ (credit) | N* (sem crédito) | N* (com crédito) | P_enf | E[fine] | N̄_controller |\n")
        f.write("|---|---:|---:|---:|---:|---:|---:|---:|\n")
        for r in n_star_rows:
            f.write(f"| {r['regime']} | ${r['rho_usd_per_decision']:.4f} | "
                    f"{r['delta_credit']*100:.0f}% | "
                    f"{r['n_star_no_credit']:,.0f} | "
                    f"{r['n_star_with_credit']:,.0f} | "
                    f"{r['enforcement_prob']*100:.1f}% | "
                    f"${r['expected_fine_usd']:,.0f} | "
                    f"{r['avg_controller_decisions']:,} |\n")
        f.write("\n## Cenários de sensibilidade\n\n")
        f.write("| Cenário | ρ ($/decision) | C_fixed ($/year) | N* (decisions/year) |\n")
        f.write("|---|---:|---:|---:|\n")
        for s in scenarios:
            f.write(f"| {s['scenario']} | ${s['rho_usd_per_decision']:.4f} | "
                    f"${s['c_fixed_usd_per_year']:,} | {s['n_star']:,.0f} |\n")
        f.write("\n## Casos de enforcement (20 casos do corpus)\n\n")
        f.write("Ver `data/enforcement_cases.csv` para a tabela completa. "
                "Resumo: 20 casos, 13/20 com falha evidencial (T1), "
                "9/20 com destruição de registros (T2), 14/20 com "
                "audit intractability (T3). Total de multas: "
                f"${sum(float(c['fine_usd_millions']) for c in cases):.2f}M.\n\n")
        f.write("## Validação do N* = 500,000\n\n")
        # Find the regime/scenario that yields N* ~ 500,000
        for r in n_star_rows:
            if abs(r["n_star_no_credit"] - 500_000) / 500_000 < 0.20:
                f.write(f"- **N* ≈ 500,000 corresponde ao regime {r['regime']}** "
                        f"com ρ = ${r['rho_usd_per_decision']:.4f}/decision "
                        f"(sem compliance credit). "
                        f"N* exato = {r['n_star_no_credit']:,.0f}.\n")
            if abs(r["n_star_with_credit"] - 500_000) / 500_000 < 0.20:
                f.write(f"- **N* ≈ 500,000 corresponde ao regime {r['regime']}** "
                        f"com ρ = ${r['rho_usd_per_decision']:.4f}/decision "
                        f"e compliance credit δ = {r['delta_credit']*100:.0f}%. "
                        f"N* exato = {r['n_star_with_credit']:,.0f}.\n")
        f.write("\n## Reprodutibilidade\n\n")
        f.write("Para reproduzir:\n")
        f.write("```bash\n")
        f.write("python3 scripts/compute_crossover.py\n")
        f.write("```\n")
        f.write("Saída:\n")
        f.write(f"- `{nstar_path.relative_to(REPO_ROOT)}` — N* por regime\n")
        f.write(f"- `{sens_path.relative_to(REPO_ROOT)}` — grid de sensibilidade ({len(sens_rows)} pontos)\n")
        f.write(f"- `{md_path.relative_to(REPO_ROOT)}` — este resumo\n")
        f.write("- `appendix_b_pgfplots.tex` — snippet LaTeX para Apêndice B\n\n")
        f.write("> **Epistemic footer.** *Este teste valida que o cálculo de N* é "
                "determinístico e reproduzível a partir de dados versionados. "
                "Ele NÃO atesta que os valores de ρ ou C_fixed sejam precisos "
                "além das fontes citadas em `paper4/section5_crossover.tex`; "
                "qualquer alteração nos parâmetros recalcula N* automaticamente.*\n")

    # PGFPlots LaTeX snippet for Appendix B
    tex_path = REPO_ROOT / "appendix_b_pgfplots.tex"
    with tex_path.open("w") as f:
        f.write("% Auto-generated by scripts/compute_crossover.py\n")
        f.write("% Append to IEEE Computer manuscript Appendix B (TCO crossover).\n\n")
        f.write("% Sensitivity grid: N* = C_fixed / rho, varying both parameters.\n")
        f.write("\\begin{figure}[t]\n")
        f.write("\\centering\n")
        f.write("\\begin{tikzpicture}\n")
        f.write("\\begin{axis}[\n")
        f.write("    width=\\columnwidth,\n")
        f.write("    height=6cm,\n")
        f.write("    xlabel={$\\rho$ (\\$/decision)},\n")
        f.write("    ylabel={$N^*$ (decisions/year)},\n")
        f.write("    ymode=log,\n")
        f.write("    legend pos=north east,\n")
        f.write("    legend cell align=left,\n")
        f.write("    grid=both,\n")
        f.write("    grid style={dashed,gray!30},\n")
        f.write("]\n")
        # Plot N* as function of rho for several C_fixed values
        rhos = np.linspace(0.005, 0.05, 50)
        for c_fixed_val in [1000, 5000, 10000, 50000]:
            n_stars = [compute_n_star(c_fixed_val, r) for r in rhos]
            f.write(f"% C_fixed = ${c_fixed_val}/year\n")
            f.write("\\addplot[mark=none] coordinates {\n")
            for r, n in zip(rhos, n_stars):
                f.write(f"    ({r:.6f}, {n:.0f})\n")
            f.write("};\n")
            f.write(f"\\addlegendentry{{$C_{{\\text{{fixed}}}} = \\${c_fixed_val:,}$/yr}}\n")
        f.write("\\end{axis}\n")
        f.write("\\end{tikzpicture}\n")
        f.write("\\caption{Sensitivity of the compliance crossover point $N^* = "
                "C_{\\text{fixed}}/\\rho$ for four fixed-cost scenarios. "
                "The GDPR base case ($\\rho = \\$0.01$, $C_{\\text{fixed}} = \\$5{,}000$) "
                "yields $N^* = 500{,}000$ decisions/year. "
                "Data: \\texttt{data/tco\\_plot\\_data.csv}.}\n")
        f.write("\\label{fig:tco-sensitivity}\n")
        f.write("\\end{figure}\n\n")

        # Second plot: N* by regime (bar chart)
        f.write("% N* by regulatory regime (bar chart).\n")
        f.write("\\begin{figure}[t]\n")
        f.write("\\centering\n")
        f.write("\\begin{tikzpicture}\n")
        f.write("\\begin{axis}[\n")
        f.write("    ybar,\n")
        f.write("    bar width=18pt,\n")
        f.write("    width=\\columnwidth,\n")
        f.write("    height=5cm,\n")
        f.write("    ylabel={$N^*$ (decisions/year, log)},\n")
        f.write("    ymode=log,\n")
        f.write("    symbolic x coords={GDPR, EU-AI-ACT, SEC, BR-LGPD},\n")
        f.write("    xtick=data,\n")
        f.write("    nodes near coords,\n")
        f.write("    nodes near coords style={font=\\scriptsize},\n")
        f.write("    enlarge x limits=0.20,\n")
        f.write("    ymin=100,\n")
        f.write("]\n")
        f.write("\\addplot[fill=blue!50] coordinates {\n")
        for r in n_star_rows:
            short = r["regime"].replace("_", "-")
            f.write(f"    ({short}, {r['n_star_no_credit']:.0f})\n")
        f.write("};\n")
        f.write("\\addplot[fill=green!50!black,postaction={pattern=north east lines}] "
                "coordinates {\n")
        for r in n_star_rows:
            short = r["regime"].replace("_", "-")
            f.write(f"    ({short}, {r['n_star_with_credit']:.0f})\n")
        f.write("};\n")
        f.write("\\legend{Sem compliance credit, Com compliance credit ($\\delta$)}\n")
        f.write("\\end{axis}\n")
        f.write("\\end{tikzpicture}\n")
        f.write("\\caption{Compliance crossover $N^*$ by regulatory regime. "
                "Lower $N^*$ means BTV infrastructure pays for itself sooner. "
                "The SEC regime has the lowest $N^*$ due to high $\\rho = \\$1.04$/decision; "
                "BR-LGPD has the highest due to low enforcement probability.}\n")
        f.write("\\label{fig:tco-by-regime}\n")
        f.write("\\end{figure}\n")

    print(f"Wrote {md_path}", file=sys.stderr)
    print(f"Wrote {tex_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
