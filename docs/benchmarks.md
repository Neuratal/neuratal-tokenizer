# Benchmarking

This document records how NTokenizer benchmarks are measured, how to reproduce
them, and what was actually measured on the reference machine. Reported numbers
are specific to that machine and workload; they are not performance claims.

## v0.2 held-out benchmark (2026-11)

For the v0.2 release, the evaluation is a genuinely held-out corpus:
`data/v0.2/clean/benchmark/` — 544 files / 207.7 MB / page-, book-, sentence-, and
file-hash split with an audited leakage report (file-exact 0, normalized-exact 0,
containment 0; 60 distinct boilerplate long-lines documented). See
`reports/v0.2/BENCHMARK_REPORT.md` for the full measured comparison of v0.1,
v0.2 and external tokenizers on this machine, and `data/v0.2/CORPUS.md` for the
split methodology. This document's synthetic suite below is retained for
strategy A/B only and is not used for release claims.

## Reference environment

- Machine: the development workstation for this project (Linux x86-64, Rust 1.99,
  gcc 15.2), release profile (`cargo build --release`).
- The same benchmark corpus/seed is used on every run; frequencies are stable.

## Reproduce

```bash
cargo build --release
./target/release/ntokenizer benchmark --vocab-size 450     # strategy comparison
./target/release/examples/throughput                          # encode/decode/batch MB/s
cargo run --release --example memory_profile -- <records> <vocab> [--zipf]
```

## Pre-tokenization strategy comparison (built-in synthetic suite)

`ntokenizer benchmark --vocab-size 450` trains three models (one per strategy)
on a synthetic multilingual suite (19 representative samples × 12 repetitions),
then measures compression and throughput on that same suite.

Latest measurement on the reference machine (500 encode iterations, small-corpus
regime; bytes-per-token is deterministic for this corpus/seed, throughput has
±10% run-to-run variance):

| Strategy | Bytes/token | Throughput (MB/s, observed spread) | Tokens/sec (spread) |
|---|---:|---:|---:|
| RawByte | 1.509 | ~12–24 | ~8.2–16.9M |
| ConservativeUnicode | 1.444 | ~11–23 | ~8.1–16.8M |
| ScriptAndDigitAware | 1.437 | ~11–23 | ~7.8–16.7M |

Bytes-per-token is deterministic for this corpus/seed; throughput on this
shared workstation varies ±50% depending on desktop load (observed across
identical runs). Treat the throughput figures as order-of-magnitude only.

Per-category winners vary by script (see command output). There is no universal
winner; re-measure on your own corpus before choosing a strategy. Historical
Phase-4 numbers (different methodology) are superseded and recorded in the
project's change history only.

**Do not benchmark against production corpora using this suite** — it exists to
compare strategies in a fixed, small, reproducible setting.

## Encoding / decoding / batch throughput (examples/throughput)

Run: `cargo run --release --example throughput`

Measured on the reference machine (synthetic repeated multilingual payload,
release build):

| Path | Throughput |
|------|------------|
| base-tokenizer encode (no merges) | 108.3 MB/s (108M tokens/s) |
| trained-model encode | 28.1 MB/s, 10.3M tokens/s, 2.7 B/token compression |
| trained-model decode (byte reconstruction) | 385.1 MB/s |
| batch encode (64 parallel slices) | 115.4 MB/s |
| Python/ctypes (short call) | ~3.6 µs per call |
| Python/ctypes decode of ~105 KB payload | ~93 MB/s |
| Python/PyO3 probe (short call) | ~0.58 µs per call |

Notes:
- Batch encoding uses Rayon over inputs; single-thread per input.
- The Python numbers are in docs/python_integration.md (A/B ctypes vs PyO3).

## Training throughput

Measured with `examples/memory_profile.rs` on synthetic corpora (release build):

| Corpus | Unique pre-tokens | Peak RSS | Train time |
|---|---:|---:|---:|
| 10k zipf-like records | 7,526 | 8.0 MB | 0.08 s |
| 50k zipf-like records | 36,850 | 21.6 MB | 0.43 s |
| 200k zipf-like records | 147,302 | 66.0 MB | ~1.6 s |
| 2k diversified records | 5,027 | 5.2 MB | 0.03 s |
| 10k diversified records | 26,181 | 15.8 MB | 0.17 s |
| 50k diversified records | 130,329 | 64.2 MB | 1.07 s |

RSS growth ≈ 0.28–0.35 KB per unique pre-token. The corpus is streamed; unique
pre-token counts and the pair-occurrence index are held in memory. Synthetic
corpora have unrealistically uniform distributions versus natural text; treat
these as order-of-magnitude guidance only.

## Memory profiling methodology

`examples/memory_profile.rs` generates a synthetic corpus (zipf-like head
repetition and rare `rare{n}` lines, or uniform-diversified), reads
`/proc/self/status` VmRSS before/after training, prints unique-word, pair,
occurrence counts, and exits. Peak RSS is additionally measured via
`/usr/bin/time -v`. Corpus is deleted after the run; files are written to the
system temp dir.

## Determinism of benchmarks

- Trained-model bytes-per-token: deterministic for a fixed corpus and seed.
- Wall-clock throughput: noisy; always report variance and machine specifics.
- Repeated model training and repeated serialization are byte-identical
  (enforced by tests).
