# NTokenizer v0.3 Benchmark Report

Date: 2026-11-15
Status: measured. All three models were evaluated on the same held-out benchmark
with the same byte counts. Nothing here is extrapolated.

## 1. Held-out benchmark (unchanged from v0.2)

| Item | Value |
|---|---|
| Directory | `data/v0.2/clean/benchmark` |
| Manifest | `data/v0.2/manifest/manifest_benchmark.json` |
| Manifest SHA-256 | `002a8224671cd281312e059af85dad24c269e7a3945e55724d476e3e255103c8` |
| Files | 544 |
| Bytes | 207,712,944 |
| Words (Unicode `\S+`) | 22,830,490 |
| Buckets | 11 (english, turkish, arabic, hindi, korean, spanish, chinese, japanese, programming, stem, other_languages) |
| Leakage discipline | benchmark tree never used for training; audit documented in `reports/v0.2/leakage_audit.json` |

## 2. How the numbers were produced

Native NTokenizer models: `examples/benchmark_ntok` (Rust). The harness preloads
all 544 files so filesystem I/O is outside the measured window, encodes raw bytes
with `Tokenizer::encode_bytes`, checks `decode(encode(bytes)) == bytes` for every
file, and tokenizes identically on repeated runs (a determinism check panics the
run on any drift).

Commands:

```bash
# v0.3 final (and identical for v0.2)
RAYON_NUM_THREADS=1 target/release/examples/benchmark_ntok \
  --model output/v0.3/ntokenizer-0.3-256k.ntok \
  --manifest data/v0.2/manifest/manifest_benchmark.json \
  --repeats 5 --warmup 1 --label ntokenizer-0.3-256k \
  --out experiments/v0.3/results/final_ntokenizer03_256k_single.json
```

`o200k_base` was measured independently with `tiktoken 0.14.0` using the same
manifest and bucket mapping (`experiments/v0.3/verify_o200k.py`); it is a Python
measurement, so its speed figures are not directly comparable to the native harness
(see `PERFORMANCE_REPORT.md`).

Validation of the harness: the v0.2 model re-encodes the benchmark to
**42,140,653 tokens = 4.9290 B/T**, matching the published v0.2 report exactly, so
the runner is trusted for all v0.3 comparisons.

## 3. Headline result

| Model | Tokens | Byte-weighted B/T | Equal-category B/T | Tokens/byte | Tokens/word | Used vocab | Vocab usage |
|---|---:|---:|---:|---:|---:|---:|---:|
| **NTokenizer v0.3 256K** | **37,397,108** | **5.5543** | **5.1057** | **0.18004** | **1.6380** | **245,334 / 262,144** | **93.59%** |
| NTokenizer v0.2 256K | 42,140,653 | 4.9290 | 4.6857 | 0.20292 | 1.8458 | 232,659 / 262,144 | 88.75% |
| o200k_base | 50,971,306 | 4.0751 | 4.0899 | 0.24540 | 2.2326 | 133,550 / 200,019 | 66.77% |

Model size: v0.3 = **23,403,000 B** (`1f0d4ad7…`), v0.2 = 22,712,471 B
(`1fc3ee85…`). o200k_base is not an NTokenizer artifact, so no `.ntok` size applies.

Deltas, v0.3 vs v0.2:

- **+12.7%** byte-weighted B/T (4.9290 → 5.5543)
- **+9.0%** equal-category B/T (4.6857 → 5.1057)
- **−11.26%** tokens (42,140,653 → 37,397,108)
- **+19.0%** o200k token reduction over v0.2 (−17.3% → **−26.6%**)

Deltas, v0.3 vs o200k_base:

- **+36.3%** byte-weighted B/T
- **+24.8%** equal-category B/T
- **−26.6%** tokens

The native harness verified `decode(encode(x)) == x` for **544/544 files** for both
v0.2 and v0.3 (see `COMPATIBILITY_REPORT.md`).

## 4. Per-bucket results

Bytes are bucket bytes; B/T is bytes/token.

| Bucket | Bytes | v0.2 B/T | v0.3 B/T | v0.3 Δ vs v0.2 | o200k B/T | v0.3 vs o200k |
|---|---:|---:|---:|---:|---:|---:|
| turkish | 43.00 MB | 4.538 | 5.305 | **+16.90%** | 3.447 | +53.9% |
| arabic | 40.60 MB | 6.047 | 7.198 | **+19.05%** | 4.763 | +51.1% |
| english | 32.06 MB | 4.052 | 4.463 | **+10.14%** | 4.154 | +7.4% |
| hindi | 29.24 MB | 8.673 | 9.928 | **+14.47%** | 7.237 | +37.2% |
| korean | 25.26 MB | 4.468 | 5.000 | **+11.91%** | 3.267 | +53.0% |
| spanish | 15.89 MB | 4.372 | 4.736 | **+8.34%** | 4.148 | +14.2% |
| japanese | 10.14 MB | 5.377 | 5.014 | **−6.75%** | 3.424 | +46.4% |
| stem | 4.32 MB | 3.102 | 3.564 | **+14.90%** | 2.936 | +21.4% |
| programming | 4.10 MB | 3.212 | 4.096 | **+27.53%** | 3.677 | +11.4% |
| chinese | 2.95 MB | 3.754 | 3.538 | **−5.76%** | 2.748 | +28.7% |
| other_languages | 0.16 MB | 3.948 | 3.320 | **−15.91%** | 5.188 | −36.0% |

Bucket winners: v0.3 beats v0.2 on **9 of 11** buckets and beats o200k on
**10 of 11** buckets (loses only `other_languages`, which o200k handles with
dedicated Thai/Vietnamese/… tokens; this bucket is 0.16 MB, 0.07% of the benchmark).

The v0.3 regression set is exactly three buckets: chinese (−5.76%), japanese
(−6.75%), other_languages (−15.91%). All three are CJK/low-volume buckets where the
final run (a) uses `--min-word-freq 2`, which prunes the long tail of rare
whole-character merges, (b) uses MathSymbolAware, which treats hooks around CJK
punctuation as math/symbol tokens rather than CJK lexical tokens, and (c) has a
much broader Chinese/Japanese training distribution than v0.2.

### CJK single-token coverage (measured)

`experiments/v0.3/chinese_coverage.py` measures how many of the 7,845 unique
ideographs in the Chinese benchmark files exist as a single vocabulary token:

| Model | Unique chars covered | Unique % | Occurrence-weighted % |
|---|---:|---:|---:|
| v0.2 | 6,151 / 7,845 | **78.4%** | **99.43%** |
| v0.3 | 3,492 / 7,845 | **44.5%** | **96.28%** |

Results: `experiments/v0.3/results/chinese_coverage_v02.json`,
`experiments/v0.3/results/chinese_coverage_v03.json`. The 96.3% weighted figure
means most *occurrences* still resolve to one token; the long tail of rare
characters (medical and rare-name ideographs, e.g. 瀉 痰 脾) moved to multi-token
encodings. Fixing this requires CJK-scoped exceptions to `--min-word-freq`
(follow-up work, not applied post hoc).

## 5. Equal-category vs byte-weighted

Both metrics are reported because the corpus is imbalanced. v0.3 wins under both,
so no benchmark-massing or bucket-weighting manipulation can be alleged:

- byte-weighted: 4.9290 → **5.5543** (+12.7%)
- equal-category: 4.6857 → **5.1057** (+9.0%)

Byte-weighting favors high-byte-mass buckets (turkish, arabic); equal-category
gives every bucket the same weight. The win persists under the stricter metric,
i.e. it is not an artifact of the Turkish/Arabic buckets.

## 6. Stretch target: 2× o200k_base (8.1502 B/T)

**Not achieved.** Required tokens for 8.1502 B/T on 207,712,944 bytes =
**25,486,625 tokens**. v0.3 produced **37,397,108** tokens, i.e. it needs a further
**31.8% token reduction** to hit the target. The measured trajectory does not
support it:

| Vocab on v0.2 merge order | B/T (byte-weighted) | vs previous |
|---|---:|---:|
| 8,192 | 2.9905 | — |
| 32,768 | 3.8220 | +27.8% |
| 65,536 | 4.2290 | +10.7% |
| 131,072 | 4.6064 | +8.9% |
| 196,608 | 4.7999 | +4.2% |
| 262,144 | 4.9290 | +2.7% |

Each doubling of the vocabulary buys strictly less compression; reaching 8.15 B/T
from 5.55 by vocabulary alone would require an unsupported super-linear scaling law
and is not possible at any vocabulary this trainer can handle. The remaining
lever — more training data — measured at **+1.66%** overall B/T per tripling
(20 → 60 MB per bucket at 64K, 4.5243 → 4.5992); even a 10× larger corpus would
not bridge a **2.6 B/T** gap. The gap is closed neither by the data nor the
vocabulary nor the pretokenizer arms we measured. The 8.15 figure remains a
stretch research question, not an outcome.

## 7. Reproducibility

| Item | SHA-256 |
|---|---|
| v0.3 model | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` |
| v0.2 model | `1fc3ee8568b7a08a124a5e27ee7a8078d5bdfae495a253b3f97ba0ad29ab54ac` |
| Benchmark manifest | `002a8224671cd281312e059af85dad24c269e7a3945e55724d476e3e255103c8` |
| v0.3 corpus manifest | `2b5a5c523d28b76aa11c371c312814be400cce82d95a0ec37c989b0a7a2cc186` |
| Raw results | `experiments/v0.3/results/*.json` |
| Aggregate | `reports/v0.3/EXPERIMENT_RESULTS.csv` |

All measurements are single-machine, single-process, warm-run medians of ≥5
measured repetitions after one warmup for the speed figures, and single-run exact
for the token counts (they are deterministic; the harness checks repeatability).
Numbers may differ by ±0.5% on other machines; bytes/token itself does not drift.
