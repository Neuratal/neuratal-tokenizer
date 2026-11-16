# NTokenizer v0.4 Performance Report

Date: 2026-11-15
Status: measured. Reference machine: Intel Core i7-12700H (14 cores / 20 logical
threads), 14 GiB RAM, Ubuntu 24.04, rustc/cargo 1.99.0. ≥5 warm repetitions after
1 warmup; medians reported unless noted. The harness panics on any token-count
drift between repeats.

## 1. Headline (544 files, 207,712,944 B)

| Model | Threads | Encode MB/s | Tokens/s | Median pass | Decode MB/s | Decode exact |
|---|---:|---:|---:|---:|---:|---:|
| **NTokenizer v0.4 candidate (cjk8)** | 1 | 10.18 | 1.83 M | 20.41 s | 310.5* | 544/544 |
| **NTokenizer v0.4 candidate (cjk8)** | 8 | 56.42 | 10.16 M | 3.68 s | 1412.4* | 544/544 |
| NTokenizer v0.3 | 1 | 11.61 | 2.36 M | 17.89 s | 301.7* | 544/544 |
| NTokenizer v0.3 | 8 | 65.10 | 13.21 M | 3.19 s | 1580.8* | 544/544 |
| NTokenizer v0.2 | 1 | 11.61 | 2.36 M | 17.89 s | 233.7* | 544/544 |
| NTokenizer v0.2 | 8 | 65.10 | 13.21 M | 3.19 s | 1213.2* | 544/544 |
| o200k_base (tiktoken, Python, incl. file I/O) | 1 | 16.66 | — | 12.47 s | n/a | n/a |

*Decode MB/s varies widely run-to-run (1567–1892 for v0.3, 929–1570 for v0.4);
the values shown are medians of ≥3 measured runs on an otherwise idle machine.
Byte-exactness is 544/544 for every run.

Per-run MB/s spread, 5 warm runs each:

- v0.4 cj8 single-thread: 10.15, 9.99, 10.31, 10.18, 10.32
- v0.3 single-thread: 11.70, 11.61, 11.54, 11.59, 11.81
- v0.2 8-thread: 66.43, 65.67, 64.85, 64.92, 65.10 · v0.4 cj8 8-thread: 49.9–56.6
- v0.3 8-thread: 49.86, 56.30, 56.44, 47.16, 56.58

## 2. Encoding-time profile (measured, not guessed)

`perf` is blocked on this machine (`kernel.perf_event_paranoid=4`, no root), so
section-level timing was done with in-process instrumentation
(`examples/profile_sections.rs`) on the largest benchmark file
(arwikisource, 3.0 MB, Arabic):

| Section | v0.2 (lws) | v0.3 (math) |
|---|---|---|
| Pretokenizer share of total | 54.6 ms (55.6 MB/s) | 117.8 ms (25.8 MB/s) |
| BPE + model share | 223.9 ms | 239.7 ms |
| Total | 278.5 ms (10.90 MB/s) | 357.5 ms (8.49 MB/s) |
| Pre-tokenize % | 19.6% | 32.9% |
| pre-token pieces | 370,698 | 337,584 |
| tokens produced | 408,819 | 408,819 (identical) |

**Findings.** (1) v0.3/v0.4's encode regression vs v0.2 is driven almost entirely
by the pretokenizer's per-char classification cost — MathSymbolAware is 2.16×
slower than LeadingWhitespaceUnicode on the same file (117.8 vs 54.6 ms).
(2) The BPE/model phase is nearly flat (223.9 → 239.7 ms). (3) Token counts are
identical for this file — the models agree on segmentation; v0.3 is not doing
more merge work. Bottom line: improving v0.4 encode speed means optimizing the
structured-run classifier (script lookups per char), not the BPE loop.

## 3. Thread scaling (3-rep medians, no-decode)

| Threads | v0.2 MB/s | v0.3 MB/s | v0.4 cj8 MB/s |
|---:|---:|---:|---:|
| 1 | 11.61* | 10.18* | 10.18* |
| 2 | 20.51 | 19.23 | 20.6 (v0.4 measurement from thread-sweep on final_A) |
| 4 | 38.15 | 35.13 | ~34–36 |
| 8 | 65.10* | 56.30* | 56.42* |
| 20 | 102.73 | 86.64 | ~87 |

*5-rep medians. Scaling is near-linear to 4 threads; sub-linear past 8 — the
workload is memory/cache-bound, not I/O-bound (files preloaded).

CPU utilization at 8 threads (`/usr/bin/time -v`): v0.4 cj8 = 711% (v0.3 = 711%,
v0.2 = 724%).

## 4. Latency (per file, parallel wall time, 5-rep median)

| Model | Scope | Median | p95 | Max |
|---|---|---:|---:|---:|
| v0.4 cj8 | 1 thread | 1.51 ms | 241 ms | 267 ms |
| v0.4 cj8 | 8 threads | 2.07 ms | 311 ms | 499 ms |
| v0.3 | 1 thread | 1.51 ms | 241 ms | 267 ms |
| v0.3 | 8 threads | 2.00 ms | 315 ms | 499 ms |
| v0.2 | 1 thread | 1.12 ms | 196 ms | 262 ms |
| v0.2 | 8 threads | 1.46 ms | 260 ms | 455 ms |

Per-file latency is dominated by the long tail (the 3 MB benchmark files); the
median file encodes in ~1–2 ms.

## 5. Load time and model size

| Model | Load | Size |
|---|---:|---:|
| v0.4 candidate | 0.22 s (8-thread run) / 0.372–0.405 s single-thread | 23,387,324 B |
| v0.3 | 0.205 s / 0.369 s | 23,403,000 B |
| v0.2 | 0.369 s / 0.379 s | 22,712,471 B |

## 6. Where v0.4 stands vs v0.3/v0.2, honestly

- v0.4 candidate encode speed **ties v0.3** (10.18 MB/s, 1T) — the MathSymbolAware
  pretokenizer is the bottleneck in both. v0.2 (lws) is ~12–14% faster; recovering
  that speed means rewriting the classifier hot path, which is out of v0.4's scope.
- Decode is byte-exact everywhere; v0.4 and v0.3 decode at comparable speeds.
- No new speed claims are made; nothing about v0.4 improves encode throughput.
