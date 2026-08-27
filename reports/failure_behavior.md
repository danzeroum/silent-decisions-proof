# Failure Behavior — Fail-Secure Under Partition

**Branch:** `artifact-v2` (local)
**Date:** 2026-08-27
**Test source:** `btv-core/tests/test_partition.rs` (5 tests), `tests/pyo3/test_binding.py::test_fail_secure_when_log_unavailable`

> **Epistemic footer.** *Este teste valida que `issue_verdict` retorna `Err(BtvError::LogUnavailable)` quando `sink.is_available() == false`, e que nenhum registro é appendado ao log. Ele NÃO garante que o sistema preserva Legalidade sob todas as formas de falha distribuída — apenas sob a falha explicitamente simulada (toggle de `AtomicBool`). Falhas de rede reais, partições bizantinas, ou comprometimento do backend não são cobertas.*

---

## CAL Trilemma — sacrificed dimension

O CAL Trilemma afirma que, em um sistema distribuído, é possível garantir ao mesmo tempo apenas dois dos três:

- **C**onsistency (Legalidade: toda decisão emitida tem evidência persistente)
- **A**vailability (Disponibilidade: toda requisição recebe resposta)
- **L**atency (baixa latência: resposta em tempo finito e pequeno)

O BTV escolhe **C + L**, sacrificando **A**: quando o log está indisponível, o sistema **não emite o Verdict** (nega serviço) em vez de emitir um Verdict sem persistência.

Isto é o oposto do teorema CAP (que escolhe C ou A sob partição de rede); a analogia é exata apenas sob a hipótese de que o `LogSink::append` é a única fonte de verdade persistente. Se houver outras fontes (ex.: persistência no caller), a analogia quebra.

---

## Mecanismo de fail-secure

`btv-core/src/lib.rs`, função `issue_verdict`:

```rust
pub fn issue_verdict(
    token: EvidenceToken,
    compliance: ComplianceToken,
    decision: Decision,
    explanation: String,
    sink: &dyn LogSink,
) -> Result<Verdict, BtvError> {
    if !sink.is_available() {
        // Fail-secure: do not construct the Verdict. Token is consumed
        // (moved into this function) but not used — the caller loses it.
        // This is intentional: if we returned the token to the caller,
        // they could retry with the same token, violating linearity.
        std::mem::forget(token);
        std::mem::forget(compliance);
        return Err(BtvError::LogUnavailable);
    }

    let verdict = Verdict::new(token, compliance, decision, explanation);
    let record = verdict.to_record();

    match sink.append(&record) {
        Ok(()) => Ok(verdict),
        Err(e) => {
            // Verdict existed in memory but is not durable. Forget it
            // so the caller cannot accidentally use an un-logged verdict.
            std::mem::forget(verdict);
            Err(e)
        }
    }
}
```

**Pontos-chave:**

1. **Checagem pré-construção**: `sink.is_available()` é consultado antes de `Verdict::new()`. Se `false`, o Verdict nunca existe em memória.
2. **Linearidade preservada**: `EvidenceToken` e `ComplianceToken` são movidos para dentro da função. Se falharmos antes do consumo, fazemos `std::mem::forget` para impedir que o caller reutilize o token em um retry.
3. **Falha pós-construção**: se `sink.append()` retornar `Err` (ex.: I/O error, constraint violation), o Verdict existed em memória mas não é durável. `mem::forget` impede que o caller use essa versão "fantasma".
4. **Erro tipado**: `BtvError::LogUnavailable` vs `BtvError::Backend(msg)` permitem ao caller distinguir "log está down" de "log rejeitou o registro".

---

## Testes executados

### `partition_in_memory_no_verdict_emitted`

Simula `InMemoryLogSink::fail()` antes da chamada. Verifica:
- `issue_verdict` retorna `Err(BtvError::LogUnavailable)`
- `sink.len() == 0` (nenhum registro appendado)

### `partition_sqlite_no_verdict_emitted`

Mesmo padrão, com `SqliteLogSink::fail()` (toggle do `AtomicBool`, sem fechar a conexão SQLite). Verifica que o caminho fail-secure é independente do backend.

### `recovery_allows_subsequent_verdicts`

Após `fail()` + tentativa rejeitada, chama `recover()`. Verifica que o sistema volta a emitir Verdicts e que o contador de registros incrementa corretamente.

### `concurrent_failures_all_rejected`

10 threads concorrentes tentam `issue_verdict` enquanto o sink está em `fail()`. Todas as 10 recebem `Err(BtvError::LogUnavailable)`, e `sink.len() == 0` ao final. Verifica atomicidade do `AtomicBool` sob contenção.

### `append_failure_also_rejected`

Usa um `LogSink` custom que retorna `is_available() == true` mas cujo `append()` sempre falha. Verifica que o caminho pós-construção também fail-secures (verifica `Err(BtvError::Backend(_))`).

### `test_fail_secure_when_log_unavailable` (Python/PyO3)

Mesmo cenário via binding PyO3. Verifica que o erro tipado é propagado para Python como `btv_python.BTVError`.

---

## Limites

1. **A falha simulada é binária** (toggle de `AtomicBool`). Falhas reais de rede têm semântica mais rica (timeout, partial write, partition asymétrica). O `LogSink` trait não modela essas distinções.
2. **Não há retries.** Se o sink falhar, o caller recebe `Err` imediatamente. Em produção, pode-se querer retry com backoff exponencial — mas isto **deve** ser implementado no caller ou em um wrapper, nunca dentro de `issue_verdict` (pois isto adiaria a falha e poderia violar a hipótese de latência baixa do CAL).
3. **Não há propagação de timeout.** O `LogSink::append` é síncrono; se o backend bloquear indefinidamente, `issue_verdict` também bloqueia. Recomenda-se que implementações de `LogSink` usem timeouts internos (ex.: `tokio::time::timeout` em volta do I/O).
4. **Não há Byzantine fault tolerance.** Se o backend `LogSink` for comprometido e responder `Ok(())` sem efetivamente persistir, o `Verdict` é retornado ao caller sob a crença falsa de que está persistido. Mitigar isto requer Merkle chains ou quorum writes — fora do escopo.

---

## Ação recomendada para o manuscrito

### Seção 6 (Threat Model) — adicionar subseção "Failure semantics"

> *Under log partition, `issue_verdict` returns `Err(BtvError::LogUnavailable)` without constructing a `Verdict`. The `EvidenceToken` and `ComplianceToken` are consumed (moved into the function) and forgotten via `std::mem::forget`, preventing retry with the same token. This realizes the CAL Trilemma's "sacrifice Availability" choice: the system denies service rather than emit an unevidenced decision. The analogy to CAP's "sacrifice Availability under partition" holds only under the assumption that `LogSink::append` is the sole source of durable truth; if the caller has independent persistence, the analogy breaks.*

### Reescrita de claim "fails-closed by construction"

**Antes:**

> *The system fails closed — it is impossible to issue an unevidenced decision.*

**Depois (sugerido):**

> *Under the hypothesis that `LogSink::is_available()` and `LogSink::append()` correctly report the backend's state, `issue_verdict` fails closed: no `Verdict` is returned to the caller unless `append` returned `Ok(())`. If the backend lies (Byzantine fault), this guarantee does not hold.*
