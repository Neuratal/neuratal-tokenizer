# NTokenizer v0.3 Performance Report

Date: 2026-11-15
Status: measured on the reference machine (Intel Core i7-12700H, 14 cores /
20 logical threads, 14 GiB RAM). Warm-run medians over **5 measured repetitions**
after 1 warmup for all headline figures. Peak RSS is process `ru_maxrss`.

## 1. Headline (544-file, 207,712,944-byte benchmark)

| Model / runner | Threads | Encode MB/s | Tokens/s | Median pass | Load time | Decode MB/s | Decode exact |
|---|---:|---:|---:|---:|---:|---:|---:|
| **NTokenizer v0.3** | 1 | **10.18** | **1.83 M** | 20.40 s | 0.205 s | **246.8** | **544/544** |
| **NTokenizer v0.3** | 8 | **56.30** | **10.14 M** | 3.69 s | 0.207 s | **1386.8** | **544/544** |
| NTokenizer v0.2 | 1 | 11.61 | 2.36 M | 17.89 s | 0.369 s | 233.7 | 544/544 |
| NTokenizer v0.2 | 8 | 65.10 | 13.21 M | 3.19 s | 0.379 s | 1213.2 | 544/544 |
| o200k_base (tiktoken, Python, incl. file I/O) | 1 | 16.66 | — | 12.47 s | — | n/a (native decode not available) | n/a |

Per-run MB/s spread (5 repeats):

- v0.3 single-thread: 10.15, 9.99, 10.31, 10.18, 10.32
- v0.3 8-thread: 49.86, 56.30, 56.44, 47.16, 56.58 (median 56.30; repeats 1 and 4
  hit a noisy machine — median used throughout)
- v0.2 single-thread: 11.70, 11.61, 11.54, 11.59, 11.81
- v0.2 8-thread: 66.43, 65.67, 64.85, 64.92, 65.10

## 2. Latency (per file, parallel wall time)

| Model | Scope | Median | p95 | Max |
|---|---|---:|---:|---:|
| v0.3 | 1 thread | 1.51 ms | 241.5 ms | 267.2 ms |
| v0.3 | 8 threads | 2.00 ms | 315.2 ms | 498.8 ms |
| v0.2 | 1 thread | 1.12 ms | 195.5 ms | 261.9 ms |
| v0.2 | 8 threads | 1.46 ms | 260.3 ms | 455.2 ms |

Per-file latency is dominated by the long tail: most files encode in ~1–2 ms;
p95/max are the multi-MB benchmark files. Under `--threads 8`, latency includes
contention from 8 parallel encoders, exactly as the harness reports.

## 3. Thread scaling (3 measured repeats, no decode pass)

| Threads | v0.2 MB/s | v0.2 tok/s | v0.3 MB/s | v0.3 tok/s |
|---:|---:|---:|---:|---:|
| 1 | 11.61* | 2.36 M | 10.18* | 1.83 M |
| 2 | 20.51 | 4.16 M | 19.23 | 3.46 M |
| 4 | 38.15 | 7.74 M | 35.13 | 6.33 M |
| 8 | 65.10* | 13.21 M | 56.30* | 10.14 M |
| 20 | 102.73 | 20.84 M | 86.64 | 15.60 M |

(*5-rep medians; others are 3-rep medians.) Scaling is near-linear to 4 threads
(3.3× on 4× cores) and sub-linear after 8, consistent with the preloaded-corpus
benchmark being memory-bandwidth and shared-cache bound, not I/O bound.

CPU utilization at 8 threads (`/usr/bin/time -v`): v0.3 = 711%, v0.2 = 724% —
i.e. the work is genuinely parallel; per-run jitter is the only source of the
occasional 47 MB/s dip in v0.3 repeats.

## 4. Cost of the MathSymbolAware pretokenizer

v0.3 encodes ~12% fewer bytes/s single-threaded (10.18 vs 11.61 MB/s) and ~14%
fewer at 8 threads (56.3 vs 65.1 MB/s) than v0.2, at equal vocabulary size and
model size. That is the measured overhead of the heavier symbol/context rules —
~2.5 s extra across the 544-file benchmark (≈ 4.6 ms per file, ≈ 12 µs per KiB),
a worthwhile price for +12.7% bytes/token, but it is reported, not hidden.
Tokens/s drops further (1.83 M vs 2.36 M) only because the v0.3 token stream is
denser per byte.

Decode is byte-exact in 544/544 files for v0.3 and v0.2 and is **faster** for v0.3
(246.8 vs 233.7 MB/s single pass, 1386.8 vs 1213.2 MB/s at 8 threads) because
fewer tokens are emitted per byte.

## 5. Comparison with o200k_base (measured, not estimated)

o200k_base was measured with `tiktoken 0.14.0` in `.venv` on the same benchmark:

- 5 measured passes at 12.59, 11.46, 11.25, 12.47, 12.97 s → median **12.47 s**,
  **16.66 MB/s** (includes per-file open/read/UTF-8 decode in Python),
  peak RSS **172 MiB**.
- The native NTokenizer harness preloads files and excludes I/O, while the tiktoken
  pass includes it; the two numbers must not be compared as “Rust vs Python
  throughput”. They are both “wall-clock to encode the benchmark” as measured.
- Byte-exact round-trip is unavailable for o200k_base in this setup
  (`decode` is only as lossless as the tiktoken configuration used), so no decode
  MB/s is claimed. v0.2 and v0.3 are both 544/544 byte-exact.
- v0.3 produces **26.6% fewer tokens** than o200k_base (37.40 M vs 50.97 M).

## 6. Methodology notes and honest caveats

- Encode throughput includes the full pipeline: model lookup, merge application,
  special-token detection. Files are preloaded; filesystem and manifest parsing are
  outside the window.
- Bytes/token is deterministic and does not drift between runs; the harness panics
  on any nondeterminism.
- Peak RSS differences (v0.3 1119–1479 MiB single-thread/8-thread, v0.2
  1029–1154 MiB) are dominated by per-thread arenas in the parallel encode path and
  by the model file’s own size (23.40 MB vs 22.71 MB). See `MEMORY_REPORT.md`.
- Session caveat from v0.2 still applies: absolute MB/s on this machine varies with
  system load; relative v0.2/v0.3 gaps (−12 to −14%) reproduce across runs.
- No GPU, no batching, no AVX-512 dispatch claims; everything is safe Rust on the
  default target features.
