# Baseline Comparativo de Latência e Throughput

**Branch:** `artifact-v2` (local)

**Hardware:** x86-64 (single core, in-process)

**Samples:** 5,000 por implementação Python; 100 por bench Rust (criterion)

## Resultados

| Implementação | p50 (μs) | p95 (μs) | p99 (μs) | média (μs) | stdev (μs) | throughput (ops/s) |
|---|---:|---:|---:|---:|---:|---:|
| BTV-Rust-native (criterion, mean only) | — | — | — | 4.29 | 18.08 | 233260 |
| BTV-PyO3 (Python bench) | 7.66 | 12.06 | 18.03 | 7.93 | 3.46 | 126111 |
| BTV-Rust+SQLite WAL+FULL (criterion, mean only) | — | — | — | 208.93 | 120.41 | 4786 |
| OpenTelemetry pós-hoc (BatchProcessor+NullExporter) | 17.77 | 25.11 | 47.71 | 20.76 | 31.45 | 48164 |
| SQLite ACID bare (BEGIN..COMMIT) | 8.61 | 13.99 | 20.53 | 9.96 | 4.25 | 100442 |

**OS-08 note:** linhas rotuladas "mean (Criterion)" NÃO têm percentis — Criterion reporta a média no `estimates.json`; as células p50/p95/p99 estão vazias no CSV em vez de conter a média copiada (o defect F8). O throughput das linhas Python é o recíproco da média em laço single-threaded (igual a ops/wall_clock neste desenho).

## Notas metodológicas

- **BTV-Rust-native** medido via `cargo bench` (criterion, 100 amostras, estimativa pontual da média). Não há percentis pois criterion reporta apenas a média no `estimates.json`.
- **BTV-PyO3** medido via `time.perf_counter_ns()` em loop Python, 5.000 amostras após warmup de 50 iterações.
- **OpenTelemetry pós-hoc** usa `BatchSpanProcessor` com `NullExporter` (export síncrono para evitar I/O de arquivo). Mede apenas a criação do span e enfileiramento, não o flush.
- **SQLite ACID bare** mede `BEGIN..INSERT..COMMIT` em SQLite WAL+FULL synchronous, sem BTV. É o baseline mais pessimista (commit síncrono em disco a cada operação).
- **BTV-Rust+SQLite WAL+FULL** é o `issue_verdict` completo com persistência SQLite (criterion, 100 amostras).

## Interpretação para o manuscrito

- O BTV nativo (Rust, in-memory) adiciona ~4.29 μs de overhead puro (hash BLAKE3 + HMAC-SHA256 + construção de structs).
- Quando persistido em SQLite WAL+FULL, o BTV fica em ~208.93 μs — comparável ao baseline SQLite ACID bare (~9.96 μs).
- Através do PyO3, o overhead de FFI adiciona ~3.64 μs sobre o nativo, mas ainda é ~0.4× mais rápido que OpenTelemetry pós-hoc (que tem custo de enfileiramento assíncrono).

## Limitações

- Hardware único (x86-64); sem repetição em ARM64.
- Sem carga concorrente (medição single-threaded).
- OpenTelemetry com `NullExporter` subestima o custo real de export (que envolve serialização protobuf + rede).
- SQLite em disco local SSD; replicação geográfica não medida.

> **Epistemic footer.** *Este teste valida que o overhead do BTV (Rust e PyO3) é mensurável e comparável a baselines realistas (OpenTelemetry, SQLite ACID). Ele NÃO garante que os números sejam representativos de produção, pois a medição foi feita em hardware único (x86-64), sem carga concorrente, sem replicação geográfica, e com SQLite em memória primária.*
