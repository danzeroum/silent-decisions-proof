# TCO Derivation Summary (OS-04 — derived, not declared)

**Model:** `rho = P_enf x E[fine] / N_bar`; `N*_full = C_fixed / (rho - c_variable)`; `N*_credit = C_fixed / (rho*delta - c_variable)`.

**Sources:** `data/enforcement_cases.csv`, `data/policy_parameters.yaml` (cost components cited there with public list prices, accessed 2026-09-15).

**Costs:** C_fixed = $2,700/yr (base; dedicated-HSM scenario $15,100/yr), c_variable = $6e-06/decision.

## Corpus fine statistics (bootstrap, 10000 resamples, seed 20260914)

| Regime | n | Median | 95% CI (median) | Mean | 95% CI (mean) |
|---|---:|---:|---:|---:|---:|
| GDPR | 10 | $14.25M | [$8.80M, $28.75M] | $51.41M | [$12.00M, $123.45M] |
| EU_AI_ACT | 0 | — (no corpus cases; `fine_source: assumption`) | — | — | — |
| SEC | 5 | $35.00M | [$6.00M, $200.00M] | $65.00M | [$17.00M, $134.20M] |
| BR_LGPD | 0 | — (no corpus cases; `fine_source: assumption`) | — | — | — |

> **Selection bias, documented:** the corpus is a set of 20 NOTABLE
> enforcement actions, not a random sample of controllers; E[fine]
> estimated from it is biased upward relative to the controller
> population. This is why the submitted manuscript reports the
> economics qualitatively and the artifact publishes the derivation
> instead of a headline crossover volume.

## Derived rho and crossover per regime

| Regime | P_enf | E[fine] | N_bar | rho (derived) | delta | N* full-avoidance (base) | N* (dedicated HSM) | N* credit-only (base) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| GDPR | 0.010 | $14.25M | 1.0e+06 | 0.142500 | 10% | 18,948 | 105,969 | 189,474 |
| EU_AI_ACT | 0.020 | $3.00M | 3.0e+06 | 0.020000 | 50% | 135,041 | 755,227 | 270,000 |
| SEC | 0.050 | $35.00M | 6.0e+06 | 0.291667 | 0% | 9,257 | 51,772 | no crossover |
| BR_LGPD | 0.005 | $0.10M | 5.0e+05 | 0.001000 | 0% | 2,716,298 | 15,191,147 | no crossover |

`no crossover` = the saving per decision does not cover the variable
cost (or is zero for credit-only regimes with delta = 0): BTV does not
pay for itself under that scenario and penalty framing alone.

## Reconciliation notes (audit F4)

- **GDPR 10.8x error:** the old file declared rho = 0.01 while the
  published formula gives 0.108 for the old inputs; the old declared
  headline crossover volume followed the declared value instead of the
  formula. rho is no longer an input anywhere in this artifact.
- **BR-LGPD contradiction (CSV 0.001 vs YAML 0.005):** with the
  corpus-derived E[fine] = $0.1M, the formula yields 0.001 — equal to
  the per-case rho column in `data/enforcement_cases.csv`. Resolved.
- **SEC $125M 'average':** the corpus mean for SEC is $65M and the
  median $35M; the corpus-derived median replaces the unverifiable
  $125M figure.
- **Credit direction:** the old `N* = C/(rho(1-delta))` made the
  credit REDUCE BTV's attractiveness; the corrected saving is
  `rho*delta` per decision, so `N*_credit = C/(rho*delta - c_var)`.
- **CI:** asserts internal consistency (rho and N* re-derive from
  the primitives); it does NOT assert any particular N* value.

## Reproducibility

```bash
python3 scripts/compute_crossover.py
```

Outputs: `data/n_star_by_regime.csv`, `data/tco_plot_data.csv`,
`appendix_b_pgfplots.tex`, this summary.

> **Epistemic footer.** *Este script valida determinismo, derivação a
> partir dos primitivos e consistência interna. Ele NÃO atesta
> imparcialidade do corpus (seleção de casos notórios) nem precisão
> dos preços de lista citados além da data de acesso; ver nota de
> viés acima. Nenhum valor de N* é assertado na CI.*
