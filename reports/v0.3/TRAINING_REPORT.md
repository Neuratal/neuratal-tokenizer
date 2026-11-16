# NTokenizer v0.3 Training Report

Date: 2026-11-15
Status: complete — final model trained and validated; all numbers measured on the
reference machine (Intel Core i7-12700H, 14 cores / 20 logical threads, 14 GiB RAM,
Ubuntu Linux, rustc/cargo 1.99.0).

## 1. Final run

```bash
MALLOC_ARENA_MAX=2 /usr/bin/time -v target/release/ntokenizer train \
  --manifest data/v0.3/manifest/manifest_corpus.json \
  --vocab-size 262144 --min-freq 2 --min-word-freq 2 \
  --pretokenizer math --seed 42 \
  --output output/v0.3/ntokenizer-0.3-256k.ntok
```

| Field | Value |
|---|---|
| Model | `output/v0.3/ntokenizer-0.3-256k.ntok` |
| SHA-256 | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` |
| Size | 23,403,000 bytes |
| Format | NTOK v3, embedded metadata (`model_name=ntokenizer-0.3-256k`, crate/model version 0.3.0) |
| Algorithm | byte_level_bpe |
| Pretokenizer | `MathSymbolAware` (`pretokenizer_version=1`) |
| Normalization | None |
| Vocabulary | 262,144 (256 base bytes + 261,881 merges + 7 special tokens) |
| Special tokens | `<|bos|>`, `<|eos|>`, `<|pad|>`, `<|unk|>`, `<|system|>`, `<|user|>`, `<|assistant|>` (IDs 262137–262143) |
| Manifest SHA-256 | `2b5a5c523d28b76aa11c371c312814be400cce82d95a0ec37c989b0a7a2cc186` |
| Manifest files | 10,555 entries; 20,041,192 records; 1,963,480,352 bytes |
| Unique pre-tokens | 20,252,150 |
| Pruned at `--min-word-freq 2` | 14,555,417 (71.9%); 5,696,733 used for merge learning |
| Merges learned | 261,881 (all requested merges learned) |
| Wall time | 5 min 47.30 s (347.3 s) |
| User / system CPU | 576.2 s / 668.5 s (358% CPU) |
| Peak training RSS | 3,882,900 KiB (≈ 3.70 GiB) |
| Training started/finished | 2026-11-15T12:58:31Z → 2026-11-15T13:04:18Z |
| Seed | 42 · `--min-freq 2` · `--min-word-freq 2` |
| Licenses reference | `data/v0.2/LICENSES.md` (see `LICENSE_AUDIT.md`) |

Exact commands and full logs are in `experiments/v0.3/logs/train_v03_final_256k.log`;
independent extraction of the timing output is in `EXPERIMENT_RESULTS.csv`.

## 2. Why `--min-word-freq 2` for the final run

The pilot exposed a hard memory wall in the merge phase before frequency pruning
was available:

- 64K on the 210 MB pilot with `mwf=1`: 3.48 M unique pre-tokens, 2.58 GiB peak RSS,
  3 min 07 s wall.
- 64K on the same pilot with `mwf=2`: the same 3.48 M pre-tokens ingested,
  2.47 M pruned, 1.02 M used; **0.68 GiB (715,548 KiB) peak RSS, 44 s wall**.
- 64K on the 631 MB pilot with `mwf=2`: 8.91 M unique pre-tokens, 6.56 M pruned;
  1.55 GiB (1,628,236 KiB) peak RSS, 1 min 56 s wall.
- Full 1.96 GB corpus with `mwf=2`: 20.25 M unique pre-tokens, 14.56 M pruned;
  **3.70 GiB peak RSS**, 5 min 47 s wall.

A control run on the small pilot (`pilot20-hybrid-64k-mwf1`) showed the quality cost
of `mwf=2` is negligible: 4.5259 B/T / 4.3069 equal-category B/T vs 4.5243 / 4.2205
for `mwf=2` (byte-weighted tie, equal-category +2.0% for `mwf=1`, driven almost
entirely by the tiny `chinese` and `other_languages` buckets). At production scale
`mwf=1` would have required storing ~20 M unique pre-tokens and their pair
frequencies; the pilot extrapolates to well beyond available RAM. `mwf=2` was
therefore mandatory, not optional, and it is applied consistently to all 64K pilots
and the final run.

## 3. Pretokenizer tournament used to pick the final pretokenizer

All arms: same corpus slice, same `--min-freq 2`, same seed, same harness.

| Arm | 32K / 20 MB pilot | 64K / 60 MB pilot | Decision |
|---|---|---|---|
| LeadingWhitespaceUnicode (P3, v0.2 default) | 3.8945 / 3.7062 | 4.3549 / 4.0288 | control |
| ConservativeUnicode (P1, legacy v0.1 default) | 2.9221 / 2.9530 | — | dominated by LWS by −25.0% overall |
| ScriptAndDigitAware (P2, legacy v0.1) | 2.8114 / 2.8447 | — | dominated by LWS by −27.8% overall |
| UnicodeHybrid (P5) | 4.0604 / 3.8592 | 4.5992 / 4.2585 | selected finalist |
| MathSymbolAware (P7) | 4.0626 / 3.8626 | **4.6033 / 4.2640** | **selected** |
| CodeAware (P6) | **4.0691 / 3.8742** | 4.6004 / 4.2366 | finalist |
| ScriptTransitionAware (P4) | 2.8994 / 2.9147 | — | rejected |
| RawByte (P0) | **feasibility failure** | — | rejected |

(“B/T / equal-category B/T”; see `EXPERIMENT_RESULTS.csv` for full columns.)

MathSymbolAware wins the controlled 64K comparison by 0.004 B/T over Hybrid and
beats Hybrid on stem (+1.55%) and programming (+0.49%) at the cost of ≤0.22% on
five language buckets. The 32K tournament had CodeAware first (4.0691/3.8742);
at 64K the ordering flips by <0.1%, i.e. they are statistically equivalent on this
machine, and MathSymbolAware was chosen because it additionally binds
decimal/time/operator and prefix/suffix symbol classes that matter for the
STEM-heavy production manifest.

## 4. Data selection

The production manifest is `data/v0.3/manifest/manifest_corpus.json`
(SHA-256 `2b5a5c52…`), built deterministically from the leakage-audited v0.2
`clean/train` tree by `experiments/v0.3/build_v03_manifest.py` with
`--profile final`. Per-bucket totals:

| Bucket | MB | Files |
|---|---:|---:|
| arabic | 293.65 | 131 |
| turkish | 287.19 | 138 |
| hindi | 230.57 | 110 |
| english | 218.95 | 144 |
| stem | 189.08 | 2,361 |
| korean | 187.27 | 90 |
| spanish | 147.08 | 70 |
| programming | 140.65 | 7,098 |
| japanese | 135.09 | 81 |
| chinese | 130.30 | 323 |
| other_languages | 3.64 | 9 |
| **total** | **1,963.48** | **10,555** |

No benchmark file is present; the benchmark manifest is
`data/v0.2/manifest/manifest_benchmark.json` (SHA-256 `002a8224…`),
207,712,944 bytes, 544 files.

## 5. Failures and aborted runs

| Run | Failure mode | Consequence |
|---|---|---|
| `pilot-raw-32k` | RawByte produced only 1.66 M unique pre-tokens but merge learning exploded the pair index; **7.52 GiB RSS at 325/32,505 merges** and still growing | RawByte mode is not viable with the current trainer; it was stopped and is reported as a failed feasibility experiment. No RawByte model is shipped. |
| 631 MB hybrid 64K, `mwf=1` (first attempt) | 8.91 M unique pre-tokens, 6.2 GB RSS before merge completion | Required `--min-word-freq 2`; all subsequent 64K and final runs use it. |
| `pilot20` hybrid mwf=1 vs mwf=2 | mwf=1 kept every unique pre-token and gained +2.05% equal-category B/T (+0.04% overall) on the small slice, but cost 3.8× the peak RSS (2.58 GiB vs 0.68 GiB) | `mwf=2` kept for the final run — at 1.96 GB the mwf=1 pair index does not fit; the mwf=1 control remains in `EXPERIMENT_RESULTS.csv` |
| Full-scale `mwf=1` final | Extrapolated memory exceeds available RAM; not attempted | `mwf=2` final is the supported configuration. |

## 6. What the final model is and is not

It is:

- a byte-level BPE tokenizer trained on 1.96 GB of leakage-audited text with
  the same seed, the same `--min-freq 2`, the same 256K vocabulary budget, and
  the same NTOK v3 format as v0.2;
- validated to round-trip 544/544 benchmark files byte-exactly (`decode(encode(b)) == b`).

It is not:

- trained on any benchmark bytes;
- guaranteed to be token-for-token compatible with Hugging Face `tokenizer.json`
  output (see `COMPATIBILITY_REPORT.md`);
- a proof of the 8.15 B/T stretch target. Measured v0.3 result: **5.5543 B/T**
  overall, **5.1057 B/T** equal-category — a **+12.7%** / **+9.0%** improvement
  over the verified v0.2 baseline, far short of 8.15 B/T. The gap is quantified
  in `BENCHMARK_REPORT.md` and `FINAL_REPORT.md`.
