# Baseline Comparativo de Latência e Throughput

**Branch:** `artifact-v2` (local)

**Hardware:** x86-64 (single core, in-process)

**Samples:** 5,000 por implementação Python; 100 por bench Rust (criterion)

## Resultados

| Implementação | p50 (μs) | p95 (μs) | p99 (μs) | média (μs) | stdev (μs) | throughput (ops/s) |
|---|---:|---:|---:|---:|---:|---:|
| BTV-Rust-native (criterion, mean only) | 1.10 | 1.10 | 1.10 | 1.10 | 0.01 | 910576 |
| BTV-PyO3 (Python bench) | 2.23 | 3.40 | 5.74 | 2.39 | 1.07 | 419257 |
| BTV-Rust+SQLite WAL+FULL (criterion, mean only) | 10.34 | 10.34 | 10.34 | 10.34 | 0.26 | 96755 |
| OpenTelemetry pós-hoc (BatchProcessor+NullExporter) | 17.70 | 24.55 | 47.06 | 20.87 | 33.05 | 47912 |
| SQLite ACID bare (BEGIN..COMMIT) | 8.61 | 13.08 | 23.11 | 9.80 | 4.50 | 102039 |

## Notas metodológicas

- **BTV-Rust-native** medido via `cargo bench` (criterion, 100 amostras, estimativa pontual da média). Não há percentis pois criterion reporta apenas a média no `estimates.json`.
- **BTV-PyO3** medido via `time.perf_counter_ns()` em loop Python, 5.000 amostras após warmup de 50 iterações.
- **OpenTelemetry pós-hoc** usa `BatchSpanProcessor` com `NullExporter` (export síncrono para evitar I/O de arquivo). Mede apenas a criação do span e enfileiramento, não o flush.
- **SQLite ACID bare** mede `BEGIN..INSERT..COMMIT` em SQLite WAL+FULL synchronous, sem BTV. É o baseline mais pessimista (commit síncrono em disco a cada operação).
- **BTV-Rust+SQLite WAL+FULL** é o `issue_verdict` completo com persistência SQLite (criterion, 100 amostras).

## Interpretação para o manuscrito

- O BTV nativo (Rust, in-memory) adiciona ~1.10 μs de overhead puro (hash BLAKE3 + HMAC-SHA256 + construção de structs).
- Quando persistido em SQLite WAL+FULL, o BTV fica em ~10.34 μs — comparável ao baseline SQLite ACID bare (~9.80 μs).
- Através do PyO3, o overhead de FFI adiciona ~1.29 μs sobre o nativo, mas ainda é ~0.1× mais rápido que OpenTelemetry pós-hoc (que tem custo de enfileiramento assíncrono).

## Limitações

- Hardware único (x86-64); sem repetição em ARM64.
- Sem carga concorrente (medição single-threaded).
- OpenTelemetry com `NullExporter` subestima o custo real de export (que envolve serialização protobuf + rede).
- SQLite em disco local SSD; replicação geográfica não medida.

> **Epistemic footer.** *Este teste valida que o overhead do BTV (Rust e PyO3) é mensurável e comparável a baselines realistas (OpenTelemetry, SQLite ACID). Ele NÃO garante que os números sejam representativos de produção, pois a medição foi feita em hardware único (x86-64), sem carga concorrente, sem replicação geográfica, e com SQLite em memória primária.*
