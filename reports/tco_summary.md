# TCO Reproducibility Summary

**Branch:** `artifact-v2` (local)

**Date:** 2026-08-27

**Sources:** `data/enforcement_cases.csv`, `data/policy_parameters.yaml`

## Formula

```latex
\rho = P_{\text{enf}} \times \frac{E[\text{fine}]}{\bar{N}_{\text{controller}}}
C_{\text{penalty}}(N) = \rho \times N
\text{TCO}(N) \approx C_{\text{fixed}} \quad \text{(for } N < 10^7\text{)}
N^* = \frac{C_{\text{fixed}}}{\rho}
```

## N* por regime (C_fixed = $5,000/year)

| Regime | ρ ($/decision) | δ (credit) | N* (sem crédito) | N* (com crédito) | P_enf | E[fine] | N̄_controller |
|---|---:|---:|---:|---:|---:|---:|---:|
| GDPR | $0.0100 | 10% | 500,000 | 555,556 | 1.0% | $10,800,000 | 1,000,000 |
| EU_AI_ACT | $0.0200 | 50% | 250,000 | 500,000 | 2.0% | $3,000,000 | 3,000,000 |
| SEC | $1.0400 | 0% | 4,808 | 4,808 | 5.0% | $125,000,000 | 6,000,000 |
| BR_LGPD | $0.0050 | 0% | 1,000,000 | 1,000,000 | 0.5% | $500,000 | 500,000 |

## Cenários de sensibilidade

| Cenário | ρ ($/decision) | C_fixed ($/year) | N* (decisions/year) |
|---|---:|---:|---:|
| optimist | $0.0500 | $1,000 | 20,000 |
| base | $0.0100 | $5,000 | 500,000 |
| pessimist | $0.0050 | $50,000 | 10,000,000 |

## Casos de enforcement (20 casos do corpus)

Ver `data/enforcement_cases.csv` para a tabela completa. Resumo: 20 casos, 13/20 com falha evidencial (T1), 9/20 com destruição de registros (T2), 14/20 com audit intractability (T3). Total de multas: $840.68M.

## Validação do N* = 500,000

- **N* ≈ 500,000 corresponde ao regime GDPR** com ρ = $0.0100/decision (sem compliance credit). N* exato = 500,000.
- **N* ≈ 500,000 corresponde ao regime GDPR** com ρ = $0.0100/decision e compliance credit δ = 10%. N* exato = 555,556.
- **N* ≈ 500,000 corresponde ao regime EU_AI_ACT** com ρ = $0.0200/decision e compliance credit δ = 50%. N* exato = 500,000.

## Reprodutibilidade

Para reproduzir:
```bash
python3 scripts/compute_crossover.py
```
Saída:
- `data/n_star_by_regime.csv` — N* por regime
- `data/tco_plot_data.csv` — grid de sensibilidade (2500 pontos)
- `reports/tco_summary.md` — este resumo
- `appendix_b_pgfplots.tex` — snippet LaTeX para Apêndice B

> **Epistemic footer.** *Este teste valida que o cálculo de N* é determinístico e reproduzível a partir de dados versionados. Ele NÃO atesta que os valores de ρ ou C_fixed sejam precisos além das fontes citadas em `paper4/section5_crossover.tex`; qualquer alteração nos parâmetros recalcula N* automaticamente.*
