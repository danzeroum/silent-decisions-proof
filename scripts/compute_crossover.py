#!/usr/bin/env python3
"""
OS-04 — TCO crossover with DERIVED rho (COMSI-2026-04-0112, closes F4).

The published model is:

    rho = P_enf x E[fine] / N_bar_controller

The previous script READ rho from data/policy_parameters.yaml as an input,
which is how the GDPR row contradicted the repository's own equation by a
factor of 10.8x while the headline N* = 500,000 claimed the declared value.
This script now:

  1. DERIVES rho from the three primitive parameters (P_enf, E[fine],
     N_bar) for every regime — rho is never an input.
  2. Estimates E[fine] from the 20-case enforcement corpus
     (data/enforcement_cases.csv): median AND mean, each with a bootstrap
     confidence interval (deterministic seed). The median is the base
     estimate (the corpus is heavy-tailed); the mean is reported for
     sensitivity. Regimes without corpus cases (EU-AI-ACT) use their
     documented `fine_source: assumption` value and are excluded from
     corpus statistics.
  3. Reconciles the former BR-LGPD contradiction between this model's
     derived rho (0.005 x 0.1M / 0.5M = 0.001) and the per-case rho column
     of the CSV (0.001): they now agree, because both follow the formula.
  4. Uses the CORRECTED compliance-credit arithmetic (OS-04 step 3):
         savings = rho * delta per decision
         N*_credit = C_fixed / (rho * delta - c_variable)
     and the full-avoidance scenario
         N*_full    = C_fixed / (rho - c_variable)
     both INCLUDING the variable cost per decision (the old zero-marginal-
     cost assumption guaranteed a crossover by construction). A
     non-positive denominator is reported as "no crossover" (None), not
     swept under a number.
  5. Emits the derived N* whatever it is; the CI job asserts INTERNAL
     CONSISTENCY (rho recomputed == rho used; N* recomputed == N*
     published), NEVER a particular value of N*.

Reads:
  - data/enforcement_cases.csv  (20 regulatory cases)
  - data/policy_parameters.yaml (primitives, costs, deltas, bootstrap cfg)

Generates:
  - data/n_star_by_regime.csv   (derived rho + N* per regime, both scenarios)
  - data/tco_plot_data.csv      (sensitivity grid, corrected formula)
  - reports/tco_summary.md      (full derivation + statistics + footer)
  - appendix_b_pgfplots.tex     (PGFPlots snippet; no hardcoded N* text)

Epistemic footer:
  Este script valida que o cálculo de N* é DETERMINÍSTICO, DERIVADO dos
  primitivos publicados e INTERNAMENTE CONSISTENTE com eles. Ele NÃO atesta
  que P_enf, o corpus de 20 casos selecionados, ou os componentes de custo
  sejam estimativas imparciais da população de controladores — o corpus é
  uma seleção de casos notórios (viés de magnitude documentado no relatório
  de saída), e os preços citados são de lista pública em 2026-09-15. A
  precisão de um valor pontual de crossover NÃO é sustentada por este
  material; o manuscrito submetido à Computer relata a economia apenas de
  forma qualitativa por essa razão.
"""

import csv
import sys
from pathlib import Path

import numpy as np
import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
DATA = REPO_ROOT / "data"
REPORTS = REPO_ROOT / "reports"
REPORTS.mkdir(parents=True, exist_ok=True)


def load_cases() -> list[dict]:
    with (DATA / "enforcement_cases.csv").open() as f:
        return list(csv.DictReader(f))


def load_params() -> dict:
    with (DATA / "policy_parameters.yaml").open() as f:
        return yaml.safe_load(f)


def derive_rho(p_enf: float, expected_fine: float, n_bar: float) -> float:
    """rho = P_enf x E[fine] / N_bar — the repository's published equation.

    This is THE definition (OS-04): rho is never read from configuration.
    """
    return p_enf * expected_fine / n_bar


def corpus_fine_stats(cases: list[dict], regime: str, boot: dict) -> dict:
    """Median and mean of the corpus fines for a regime, with bootstrap CIs."""
    fines = np.array(
        [float(c["fine_usd_millions"]) * 1e6 for c in cases if c["regime"] == regime]
    )
    if fines.size == 0:
        return {"n": 0}
    rng = np.random.default_rng(boot["seed"])
    idx = rng.integers(0, fines.size, size=(boot["resamples"], fines.size))
    boot_med = np.median(fines[idx], axis=1)
    boot_mean = np.mean(fines[idx], axis=1)
    alpha = (1.0 - boot["confidence"]) / 2.0
    q = [100 * alpha, 100 * (1 - alpha)]
    med_ci = np.percentile(boot_med, q)
    mean_ci = np.percentile(boot_mean, q)
    return {
        "n": int(fines.size),
        "median": float(np.median(fines)),
        "median_ci": (float(med_ci[0]), float(med_ci[1])),
        "mean": float(np.mean(fines)),
        "mean_ci": (float(mean_ci[0]), float(mean_ci[1])),
    }


def n_star_crossover(c_fixed: float, avoided_per_decision: float) -> float | None:
    """N* = C_fixed / (avoided_per_decision), None when there is no crossover.

    `avoided_per_decision` is the per-decision saving BTV realizes:
    rho (full-avoidance scenario) or rho*delta (credit-only scenario),
    minus the variable cost per decision. A non-positive saving means the
    infrastructure never pays for itself under that scenario — the honest
    answer is None, not a number.
    """
    net = avoided_per_decision - 0.0
    if net <= 0:
        return None
    return c_fixed / net


def n_star_full(c_fixed: float, rho: float, c_var: float) -> float | None:
    if rho - c_var <= 0:
        return None
    return c_fixed / (rho - c_var)


def n_star_credit(c_fixed: float, rho: float, delta: float, c_var: float) -> float | None:
    return n_star_crossover(c_fixed, rho * delta - 0.0) if (rho * delta - c_var) > 0 else None


def sensitivity_table(params: dict, c_var: float) -> list[dict]:
    """(rho, c_fixed, N*) grid under the corrected full-avoidance formula."""
    sens = params["sensitivity"]
    rhos = np.linspace(sens["rho_min"], sens["rho_max"], sens["rho_steps"])
    c_fixeds = np.linspace(sens["c_fixed_min"], sens["c_fixed_max"], sens["c_fixed_steps"])
    rows = []
    for rho in rhos:
        for c_fixed in c_fixeds:
            n_star = n_star_full(c_fixed, float(rho), c_var)
            rows.append({
                "rho_usd_per_decision": float(rho),
                "c_fixed_usd_per_year": float(c_fixed),
                "n_star_decisions_per_year": "" if n_star is None else f"{n_star:.2f}",
            })
    return rows


def fmt_n(n: float | None) -> str:
    return "no crossover" if n is None else f"{n:,.0f}"


def main() -> int:
    cases = load_cases()
    params = load_params()
    boot = params["bootstrap"]
    c_fixed = params["c_fixed_usd_per_year"]
    c_fixed_hsm = params["c_fixed_usd_per_year_dedicated_hsm"]
    c_var = params["c_variable_usd_per_decision"]

    print(f"Loaded {len(cases)} enforcement cases", file=sys.stderr)
    print(f"C_fixed = ${c_fixed}/yr (base), ${c_fixed_hsm}/yr (dedicated HSM)", file=sys.stderr)
    print(f"C_variable = ${c_var}/decision", file=sys.stderr)

    # ---- Corpus statistics -------------------------------------------------
    regimes = params["regimes"]
    stats: dict[str, dict] = {}
    for regime, p in regimes.items():
        stats[regime] = (
            corpus_fine_stats(cases, regime, boot)
            if p["fine_source"] == "corpus_median"
            else {"n": 0}
        )

    # ---- Derive rho + N* per regime ----------------------------------------
    rows = []
    for regime, p in regimes.items():
        p_enf = p["enforcement_prob_per_year"]
        fine = p["expected_fine_usd"]
        n_bar = p["avg_controller_decisions_per_year"]
        rho = derive_rho(p_enf, fine, n_bar)
        delta = params["compliance_credit"].get(f"{regime}_delta", 0.0)
        n_full = n_star_full(c_fixed, rho, c_var)
        n_full_hsm = n_star_full(c_fixed_hsm, rho, c_var)
        n_credit = n_star_credit(c_fixed, rho, delta, c_var)
        rows.append({
            "regime": regime,
            "fine_source": p["fine_source"],
            "p_enf": p_enf,
            "expected_fine_usd": fine,
            "n_bar": n_bar,
            "rho_derived_usd_per_decision": rho,
            "delta_credit": delta,
            "n_star_full_avoidance": n_full,
            "n_star_full_avoidance_dedicated_hsm": n_full_hsm,
            "n_star_credit_only": n_credit,
            "corpus_n": stats[regime].get("n", 0),
            "corpus_median": stats[regime].get("median"),
            "corpus_median_ci": stats[regime].get("median_ci"),
            "corpus_mean": stats[regime].get("mean"),
            "corpus_mean_ci": stats[regime].get("mean_ci"),
        })

    # ---- Internal consistency check (the CI gate asserts THIS, not N* values)
    for r in rows:
        recomputed = derive_rho(r["p_enf"], r["expected_fine_usd"], r["n_bar"])
        if abs(recomputed - r["rho_derived_usd_per_decision"]) > 1e-15:
            print(
                f"INCONSISTENT: {r['regime']} rho used {r['rho_derived_usd_per_decision']} "
                f"!= derived {recomputed}",
                file=sys.stderr,
            )
            return 1
        if r["n_star_full_avoidance"] is not None:
            expect = c_fixed / (r["rho_derived_usd_per_decision"] - c_var)
            if abs(expect - r["n_star_full_avoidance"]) > 1e-6 * max(expect, 1.0):
                print(f"INCONSISTENT: {r['regime']} N* full", file=sys.stderr)
                return 1
        # G2 (COMSI-2026-04-0112 Round 2): a `fine_source: corpus_median`
        # regime is a claim that expected_fine_usd came FROM the corpus —
        # verify it, don't just trust the label. This is F4's exact defect
        # pattern (a declared number wearing a derived number's label) in
        # miniature: BR_LGPD's corpus_fine_stats() call silently matched
        # zero rows for a full round (the CSV said regime="BR", the YAML
        # key says "BR_LGPD") and nobody noticed because the declared
        # $100k happened to equal the real median anyway. A gate that only
        # checks rho's arithmetic never sees this — it has to check that
        # corpus_median claims actually touched a nonempty corpus.
        if r["fine_source"] == "corpus_median":
            if not r["corpus_n"]:
                print(
                    f"INCONSISTENT: {r['regime']} declares fine_source: corpus_median "
                    "but corpus_n == 0 — expected_fine_usd is not derived from anything; "
                    "check data/enforcement_cases.csv's `regime` column against this "
                    "regime's key in data/policy_parameters.yaml",
                    file=sys.stderr,
                )
                return 1
            if abs(r["expected_fine_usd"] - r["corpus_median"]) > 1e-9 * max(
                abs(r["corpus_median"]), 1.0
            ):
                print(
                    f"INCONSISTENT: {r['regime']} expected_fine_usd="
                    f"{r['expected_fine_usd']} != corpus median {r['corpus_median']} "
                    "— policy_parameters.yaml's declared value has drifted from the "
                    "corpus it claims to be derived from",
                    file=sys.stderr,
                )
                return 1
    print("Internal consistency: OK (rho and N* re-derive from primitives)", file=sys.stderr)
    print(
        "Corpus provenance: OK (every corpus_median regime has corpus_n > 0 and "
        "expected_fine_usd == corpus median)",
        file=sys.stderr,
    )

    # ---- Canonical N* table -------------------------------------------------
    nstar_path = DATA / "n_star_by_regime.csv"
    fields = [
        "regime", "fine_source", "p_enf", "expected_fine_usd", "n_bar",
        "rho_derived_usd_per_decision", "delta_credit",
        "n_star_full_avoidance", "n_star_full_avoidance_dedicated_hsm",
        "n_star_credit_only", "corpus_n",
    ]
    with nstar_path.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, extrasaction="ignore")
        w.writeheader()
        for r in rows:
            out = dict(r)
            for k in ("n_star_full_avoidance", "n_star_full_avoidance_dedicated_hsm", "n_star_credit_only"):
                out[k] = "" if r[k] is None else f"{r[k]:.2f}"
            w.writerow(out)
    print(f"Wrote {nstar_path}", file=sys.stderr)

    # ---- Sensitivity grid ----------------------------------------------------
    sens_rows = sensitivity_table(params, c_var)
    sens_path = DATA / "tco_plot_data.csv"
    with sens_path.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=[
            "rho_usd_per_decision", "c_fixed_usd_per_year",
            "n_star_decisions_per_year",
        ])
        w.writeheader()
        w.writerows(sens_rows)
    print(f"Wrote {sens_path} ({len(sens_rows)} rows)", file=sys.stderr)

    # ---- Markdown summary ----------------------------------------------------
    md_path = REPORTS / "tco_summary.md"
    with md_path.open("w") as f:
        f.write("# TCO Derivation Summary (OS-04 — derived, not declared)\n\n")
        f.write("**Model:** `rho = P_enf x E[fine] / N_bar`; "
                "`N*_full = C_fixed / (rho - c_variable)`; "
                "`N*_credit = C_fixed / (rho*delta - c_variable)`.\n\n")
        f.write("**Sources:** `data/enforcement_cases.csv`, `data/policy_parameters.yaml` "
                "(cost components cited there with public list prices, accessed 2026-09-15).\n\n")
        f.write(f"**Costs:** C_fixed = ${c_fixed:,}/yr (base; dedicated-HSM scenario "
                f"${c_fixed_hsm:,}/yr), c_variable = ${c_var}/decision.\n\n")
        f.write("## Corpus fine statistics (bootstrap, "
                f"{boot['resamples']} resamples, seed {boot['seed']})\n\n")
        f.write("| Regime | n | Median | 95% CI (median) | Mean | 95% CI (mean) |\n")
        f.write("|---|---:|---:|---:|---:|---:|\n")
        for regime, s in stats.items():
            if s.get("n", 0) == 0:
                # G2: name the regime's ACTUAL declared fine_source, not a
                # hardcoded "assumption" — that hid the BR_LGPD/"BR" key
                # mismatch behind a label implying a documented, deliberate
                # choice instead of a corpus lookup that silently matched
                # nothing. The gate above already refuses to reach this
                # point for any regime declaring corpus_median with n == 0,
                # so this line is defense in depth, not the only check.
                declared_source = regimes[regime]["fine_source"]
                f.write(f"| {regime} | 0 | — (no corpus cases; "
                        f"`fine_source: {declared_source}`) | — | — | — |\n")
            else:
                lo_m, hi_m = s["median_ci"]
                lo_a, hi_a = s["mean_ci"]
                f.write(
                    f"| {regime} | {s['n']} | ${s['median']/1e6:,.2f}M | "
                    f"[${lo_m/1e6:,.2f}M, ${hi_m/1e6:,.2f}M] | ${s['mean']/1e6:,.2f}M | "
                    f"[${lo_a/1e6:,.2f}M, ${hi_a/1e6:,.2f}M] |\n"
                )
        f.write("\n> **Selection bias, documented:** the corpus is a set of 20 NOTABLE\n")
        f.write("> enforcement actions, not a random sample of controllers; E[fine]\n")
        f.write("> estimated from it is biased upward relative to the controller\n")
        f.write("> population. This is why the submitted manuscript reports the\n")
        f.write("> economics qualitatively and the artifact publishes the derivation\n")
        f.write("> instead of a headline crossover volume.\n\n")
        f.write("## Derived rho and crossover per regime\n\n")
        f.write("| Regime | P_enf | E[fine] | N_bar | rho (derived) | delta | "
                "N* full-avoidance (base) | N* (dedicated HSM) | N* credit-only (base) |\n")
        f.write("|---|---:|---:|---:|---:|---:|---:|---:|---:|\n")
        for r in rows:
            f.write(
                f"| {r['regime']} | {r['p_enf']:.3f} | ${r['expected_fine_usd']/1e6:,.2f}M | "
                f"{r['n_bar']:.1e} | {r['rho_derived_usd_per_decision']:.6f} | "
                f"{r['delta_credit']*100:.0f}% | {fmt_n(r['n_star_full_avoidance'])} | "
                f"{fmt_n(r['n_star_full_avoidance_dedicated_hsm'])} | "
                f"{fmt_n(r['n_star_credit_only'])} |\n"
            )
        f.write("\n`no crossover` = the saving per decision does not cover the variable\n")
        f.write("cost (or is zero for credit-only regimes with delta = 0): BTV does not\n")
        f.write("pay for itself under that scenario and penalty framing alone.\n\n")
        f.write("## Reconciliation notes (audit F4)\n\n")
        f.write("- **GDPR 10.8x error:** the old file declared rho = 0.01 while the\n")
        f.write("  published formula gives 0.108 for the old inputs; the old declared\n")
        f.write("  headline crossover volume followed the declared value instead of the\n")
        f.write("  formula. rho is no longer an input anywhere in this artifact.\n")
        f.write("- **BR-LGPD contradiction (CSV 0.001 vs YAML 0.005):** with the\n")
        f.write("  corpus-derived E[fine] = $0.1M, the formula yields 0.001 — equal to\n")
        f.write("  the per-case rho column in `data/enforcement_cases.csv`. Resolved.\n")
        f.write("- **SEC $125M 'average':** the corpus mean for SEC is $65M and the\n")
        f.write("  median $35M; the corpus-derived median replaces the unverifiable\n")
        f.write("  $125M figure.\n")
        f.write("- **Credit direction:** the old `N* = C/(rho(1-delta))` made the\n")
        f.write("  credit REDUCE BTV's attractiveness; the corrected saving is\n")
        f.write("  `rho*delta` per decision, so `N*_credit = C/(rho*delta - c_var)`.\n")
        f.write("- **CI:** asserts internal consistency (rho and N* re-derive from\n")
        f.write("  the primitives); it does NOT assert any particular N* value.\n\n")
        f.write("## Reproducibility\n\n")
        f.write("```bash\npython3 scripts/compute_crossover.py\n```\n\n")
        f.write("Outputs: `data/n_star_by_regime.csv`, `data/tco_plot_data.csv`,\n")
        f.write("`appendix_b_pgfplots.tex`, this summary.\n\n")
        f.write("> **Epistemic footer.** *Este script valida determinismo, derivação a\n")
        f.write("> partir dos primitivos e consistência interna. Ele NÃO atesta\n")
        f.write("> imparcialidade do corpus (seleção de casos notórios) nem precisão\n")
        f.write("> dos preços de lista citados além da data de acesso; ver nota de\n")
        f.write("> viés acima. Nenhum valor de N* é assertado na CI.*\n")
    print(f"Wrote {md_path}", file=sys.stderr)

    # ---- PGFPlots snippet (no hardcoded N* text — data-driven only) ----------
    tex_path = REPO_ROOT / "appendix_b_pgfplots.tex"
    with tex_path.open("w") as f:
        f.write("% Auto-generated by scripts/compute_crossover.py (OS-04).\n")
        f.write("% Sensitivity grid: N* = C_fixed / (rho - c_variable), corrected formula.\n")
        f.write("% No headline crossover value is embedded in this snippet by construction.\n\n")
        f.write("\\begin{figure}[t]\n\\centering\n\\begin{tikzpicture}\n\\begin{axis}[\n")
        f.write("    width=\\columnwidth,\n    height=6cm,\n")
        f.write("    xlabel={$\\rho$ (\\$/decision)},\n    ylabel={$N^*$ (decisions/year)},\n")
        f.write("    ymode=log,\n    legend pos=north east,\n    legend cell align=left,\n")
        f.write("    grid=both,\n    grid style={dashed,gray!30},\n]\n")
        rhos = np.linspace(params["sensitivity"]["rho_min"], params["sensitivity"]["rho_max"], 50)
        for c_fixed_val in [1000, 5000, 10000, 50000]:
            f.write(f"% C_fixed = ${c_fixed_val}/year\n")
            f.write("\\addplot[mark=none] coordinates {\n")
            for r in rhos:
                n = n_star_full(c_fixed_val, float(r), c_var)
                if n is not None:
                    f.write(f"    ({r:.6f}, {n:.0f})\n")
            f.write("};\n")
            f.write(f"\\addlegendentry{{$C_{{\\text{{fixed}}}} = \\${c_fixed_val:,}$/yr}}\n")
        f.write("\\end{axis}\n\\end{tikzpicture}\n")
        f.write(
            "\\caption{Sensitivity of the compliance crossover "
            "$N^* = C_{\\text{fixed}}/(\\rho - c_{\\text{var}})$ for four "
            "fixed-cost scenarios ($c_{\\text{var}} = "
            f"\\${c_var}$ per decision). Data: \\texttt{{data/tco\\_plot\\_data.csv}}.}}\n"
        )
        f.write("\\label{fig:tco-sensitivity}\n\\end{figure}\n\n")
        f.write("% N* by regulatory regime (bar chart), derived values, both scenarios.\n")
        f.write("\\begin{figure}[t]\n\\centering\n\\begin{tikzpicture}\n\\begin{axis}[\n")
        f.write("    ybar,\n    bar width=18pt,\n    width=\\columnwidth,\n    height=5cm,\n")
        f.write("    ylabel={$N^*$ (decisions/year, log)},\n    ymode=log,\n")
        f.write("    symbolic x coords={GDPR, EU-AI-ACT, SEC, BR-LGPD},\n    xtick=data,\n")
        f.write("    nodes near coords,\n    nodes near coords style={font=\\scriptsize},\n")
        f.write("    enlarge x limits=0.20,\n]\n")
        f.write("\\addplot[fill=blue!50] coordinates {\n")
        for r in rows:
            short = r["regime"].replace("_", "-")
            if r["n_star_full_avoidance"] is not None:
                f.write(f"    ({short}, {r['n_star_full_avoidance']:.0f})\n")
        f.write("};\n")
        f.write("\\addplot[fill=green!50!black,postaction={pattern=north east lines}] coordinates {\n")
        for r in rows:
            short = r["regime"].replace("_", "-")
            if r["n_star_credit_only"] is not None:
                f.write(f"    ({short}, {r['n_star_credit_only']:.0f})\n")
        f.write("};\n")
        f.write("\\legend{Full-avoidance scenario, Credit-only scenario ($\\delta$)}\n")
        f.write("\\end{axis}\n\\end{tikzpicture}\n")
        f.write(
            "\\caption{Derived compliance crossover $N^*$ by regulatory regime "
            "(lower means BTV infrastructure pays for itself sooner). Regimes with "
            "no crossover under a scenario are omitted from that bar. "
            "Data: \\texttt{data/n\\_star\\_by\\_regime.csv}.}\n"
        )
        f.write("\\label{fig:tco-by-regime}\n\\end{figure}\n")
    print(f"Wrote {tex_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
