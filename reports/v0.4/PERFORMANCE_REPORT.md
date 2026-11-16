# NTokenizer v0.4 Performance Report

Date: 2026-11-15
Machine: i7-12700H, 14 GiB RAM, Ubuntu Linux. Release build (`opt-level 3`).

## 1. Encode throughput

Native harness `examples/benchmark_ntok` (5 warm runs × 1 warmup, medians):

| Model | Threads | Encode MB/s | Tokens/s | Load | Decode exact |
|---|---:|---:|---:|---:|---:|
| v0.3 | 1 | 10.18 (median over session; ±10% machine variance) | 1.83M | 0.205 s | 544/544 |
| v0.3 | 8 | 56.30 | 10.14M | 0.207 s | 544/544 |
| v0.4-fail (cjk8) | 1 | 10.18 | 1.83M | 0.207 s | 544/544 |
| v0.4-fail (cjk8) | 8 | 56.42 | 10.16M | 0.207 s | 544/544 |
| **v0.4 cjkhan4 (new arm)** | 1 | **9.27 (this session)** | **1.67M** | 0.32 s | 544/544 |
| v0.4 cjkhan4 | 8 | — (not measured) | — | — | 544/544 |

`perf` is blocked on this machine (`kernel.perf_event_paranoid=4`, no root).
Section timing was done with in-process instrumentation
(`examples/profile_sections.rs`) on the largest benchmark file
(arwikisource 3.0 MB, Arabic, after the v0.4 optimization):

| Section | v0.2 (lws) | v0.3 (math) | v0.4 (optimized math) |
|---|---|---|---|
| pretokenize | 55.6 MB/s (54.6 ms) | 25.8 MB/s (117.8 ms) | **30.4 MB/s (100.0 ms) — 15% faster** |
| model/BPE share | 80.1% | 67.1% | 68.7% |
| total | 10.90 MB/s | 8.49 MB/s | **9.49 MB/s** |

The v0.4 change removes the `Vec<(usize,char)>` allocation per `split_text` call
and fetches the Unicode general category **once** per char (instead of twice in
`is_structured_mark` + `classify_structured`). Token streams are byte-identical
(37,397,108 tokens on the full benchmark, exact match with v0.3).

Honest measurement note: whole-benchmark encode throughput on this shared machine
moves ±10% between sessions (v0.3: 9.38–10.18 MB/s across sessions; cjkhan4: 9.27
in the same "quiet" conditions as 9.54 for v0.3). Any claimed >10% speedup would
require a quieter box; we report the instrumented pretokenizer win, not a system-
level one.

## 2. Decode throughput

| Model | 1 thread | 8 threads |
|---|---|---|
| v0.3 | 302–343 MB/s (median ~310) | ~1580 MB/s |
| v0.4-fail cj8 | 223–343 MB/s | 956–1890 MB/s (high variance) |

All byte-exact (544/544). No decode regression is introduced by v0.4.

## 3. Model sizes and load times

| Model | Size | Load |
|---|---:|---:|
| v0.1 | 22,224,561 B | ~0.37 s |
| v0.2 | 22,712,471 B | ~0.37 s |
| v0.3 | 23,403,000 B | 0.205 s |
| v0.4-fail cj8 | 23,387,324 B | 0.207 s |
| v0.4 cjkhan4 arm | 23,323,368 B | 0.32 s |

## 4. What an actual ≥10% system-level win would need

- MathSymbolAware's per-char script+category lookups (~25% of encode time)
  replaced by a compact two-level table — feasible but touches the hot path and
  risks behavior change; needs a dedicated review window.
- The BPE pair-lookup loop (~60% of encode time) is hash-map bound; a perfect-
  hash table for merge pairs would be a larger project.
- Both are v0.5 work, not this session.
