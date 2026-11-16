# NTokenizer v0.4-v2 — Baseline Audit & Diagnostics

Date: 2026-11-16
Status: baseline recorded before any v0.4-v2 experiments; no production artifact was modified in this cycle.

## 1. Polished v0.3-v2 source state (immutable baseline)

- Directory: repo root; the v0.3-v2 polishing cycle (Unicode-loss fix + 4 performance changes) completed 2026-11-15.
- Source-tree hash/reference: `experiments/v0.3-v2/pre_session_source_snapshot.tar.gz` (pre-session), current tree = pre-session + v0.3-v2 changes.
- **No source change was made in the v0.4-v2 cycle** (`src/`, `tests/`, `examples/` untouched this cycle). All polished runtime improvements are retained.
- Cargo build: clean (`cargo build --release` with the v0.3-v2 code; tests 136/136 pass per `reports/v0.3-v2/`).
- Unicode regression suite: `tests/unicode_regression_tests.rs` (6/6 pass, re-verified by v0.3-v2 report; not re-run for code changes this cycle since none were made).

## 2. v0.3 model integrity

| Item | Value |
|---|---|
| File | `output/v0.3/ntokenizer-0.3-256k.ntok` |
| SHA-256 (before and after this cycle) | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` |
| Vocabulary | 262,144 (256 base byte tokens + 261,881 learned merges + 7 special tokens) |
| Pretokenizer | `MathSymbolAware` |
| Normalization | `None` |
| Format | NTOK v3 |
| `ntokenizer validate --model` | PASS (vocabulary structure, merge consistency, special tokens, roundtrip text, roundtrip bytes, metadata) |
| `output/v0.3-v2/` contents | README.md only — **no new model bytes** |

## 3. Baseline reproduction (polished implementation, same machine)

Command: `RAYON_NUM_THREADS=1 target/release/examples/benchmark_ntok --model output/v0.3/ntokenizer-0.3-256k.ntok --manifest data/v0.2/manifest/manifest_benchmark.json --repeats 3 --warmup 1 --per-file-json experiments/v0.4-v2/results/baseline_perfile.json --out experiments/v0.4-v2/results/baseline_single.json`

Results (JSON: `experiments/v0.4-v2/results/baseline_single.json`, per-file: `experiments/v0.4-v2/results/baseline_perfile.json`):

| Metric | Polished v0.3-v2 (this cycle) | Historical v0.3 |
|---|---:|---:|
| Total benchmark bytes | 207,712,944 | 207,712,944 |
| Total tokens | **37,397,319** | 37,397,108 |
| Overall byte-weighted B/T | **5.5542** | 5.5543 |
| Equal-category B/T | **5.1056** | 5.1057 |
| Used vocabulary | 245,334 / 262,144 (93.59%) | 245,334 / 262,144 (93.59%) |
| Decode round-trip | 544/544 byte-exact | 544/544 byte-exact |
| Single-thread encode | 17.15 / 16.64 / 16.52 MB/s (3 repeats) | 10.18 MB/s (recorded) |

### The +211-token discrepancy — investigation conclusion

Historical v0.3: 37,397,108 tokens. Polished tree: 37,397,319 (+211, +0.00056%, 24 files:
15 hiwiki Hindi files, 5 trwiki, 3 enwikibooks, 1 arwikisource, 1 zhwiktionary).

Findings established in the v0.3-v2 cycle and re-confirmed:

1. The v0.4-fail-era micro-optimizations (iterator-based run loop, single category lookup,
   and the full `Mn/Mc/Me` mark classification in `is_structured_mark`) changed which
   characters attach as combining marks in Indic-heavy text vs the true v0.3 release code.
2. Reverting **this cycle's** and the v0.3-v2 cycle's Unicode pinning fix reproduces
   37,397,319 — the Unicode fix for leading combining marks is benchmark-neutral (verified
   per-file, zero files affected).
3. The v0.4-era snapshot source (`experiments/v0.3-v2/pre_session_source_snapshot.tar.gz`,
   SHA-256 `6a45fc5b936a4037c934beaf384521b9eb2530cba0782f4e3e3c03d5a9a3ef50`) builds to
   37,397,108 = the historical figure exactly.
4. The +211 drift therefore originates in the source delivered for that cycle and predates
   the v0.3-v2 polishing work. It is small (0.00056%) and arguably *more* correct for
   Devanagari mark attachment, but it is **disclosed, not erased**: both figures stand in the
   history. All v0.4-v2 comparisons use the polished build (37,397,319 / 5.5542), matching
   the project brief's instruction to use the reproducible polished implementation as the
   operational baseline.

## 4. Per-bucket baseline (polished build, this cycle)

Bytes are bucket bytes; B/T = bytes/token; equal-category over 11 buckets.

| Bucket | Files | Bytes | Tokens | B/T |
|---|---:|---:|---:|---:|
| chinese | 18 | 3.0 MB | 0.835 M | 3.5380 |
| programming | 300 | 4.1 MB | 1.000 M | 4.0961 |
| stem | 113 | 4.3 MB | 1.212 M | 3.5638 |
| english | 17 | 32.1 MB | 7.184 M | 4.4628 |
| spanish | 9 | 15.9 MB | 3.354 M | 4.7363 |
| korean | 14 | 25.3 MB | 5.051 M | 4.9999 |
| japanese | 7 | 10.1 MB | 2.023 M | 5.0135 |
| turkish | 23 | 43.0 MB | 8.105 M | 5.3054 |
| arabic | 19 | 40.6 MB | 5.641 M | 7.1984 |
| hindi | 15 | 29.2 MB | 2.945 M | 9.9277 |
| other_languages | 9 | 0.2 MB | 0.047 M | 3.3196 |
| **overall** | **544** | **207.7 MB** | **37.397 M** | **5.5542** |
| **equal-category** | | | | **5.1056** |

Benchmark composition (share of bytes): turkish 20.7%, arabic 19.5%, english 15.4%, hindi 14.1%, korean 12.2%, spanish 7.6%, japanese 4.9%, stem 2.1%, programming 2.0%, chinese 1.4%. Any overall B/T gain must come mostly from turkish/arabic/hindi/korean/english.

Note: `programming` counts 300 of 544 files (many small code files). The bucket is only 2.0% of bytes, but guardrails protect it explicitly.

## 5. Corpus audit (v0.3 training manifest `data/v0.3/manifest/manifest_corpus.json`)

- 10,555 files, 1.963 GB, seed 42, `--pretokenizer math --min-freq 2 --min-word-freq 2`.
- Manifest SHA-256 (v0.3, recorded in metadata): `2b5a5c523d28b76aa11c371c312814be400cce82d95a0ec37c989b0a7a2cc186`.
- Bucket distribution (recomputed): arabic 15.0%, turkish 14.6%, hindi 11.7%, english 11.2%, stem 9.6%, korean 9.5%, spanish 7.5%, programming 7.2%, japanese 6.9%, chinese 6.6%, other 0.2%.

### Source-attribution availability (Research A check)

The manifest carries `source`/`domain` tags but they are **coarse**: programming is split into cpython_code (45.8 MB, 2275 files), typescript_src (31.7 MB, 686), go_src (30.4 MB, 2117), rust_src (15.6 MB, 1419), cpython_docs (17.2 MB, 601) — i.e. 5 platform buckets, not repositories. Per-repository identifiers do not exist inside these buckets (cpython_code is the whole CPython tree, go_src the Go tree, …). There are also no commit/path-level repo keys.

**Consequence:** Source-Attributed BPE's R (repo count) is ≈4–5 for programming pairs; `log(R+1)` ≈ 1.6 for every code pair — a constant multiplier, not a discriminator. The paper's mechanism cannot express itself at this granularity. A source-attributed merge pilot was therefore rejected on evidence before implementation (see EXPERIMENT_LOG.md E-02). This is not an ideological objection; the instrumentation required does not exist in the data.

### Unused clean data discovery

`data/v0.2/clean/train` contains **2035 unused files (3.17 GB)** relative to the v0.3 manifest.
Of these, **303 files (50.5 MB)** are holdout-excluded (`data/v0.4/manifest/holdout_exclusion.txt`) and are **never** used for training; the remaining **1732 files (3.12 GB)** are licensed, clean, and free of any benchmark/holdout overlap:

- trwiki +702 MB, arwikisource +635 MB, hiwiki +419 MB, enwikibooks +367 MB, kowikisource +345 MB, eswikisource +195 MB, simplewiki +179 MB, jawikisource +128 MB, gutenberg_cjk_zh +31 MB (unused CJK), arxiv +45 MB, enwikinews +24 MB, trwikisource +13 MB, gutenberg_en +72 MB.

This was the only material, legitimate, leakage-free lever found in this cycle; it was piloted (EXPERIMENT_LOG.md E-01).

## 6. Vocabulary utilization audit

- Used on the 544-file benchmark: **245,334 / 262,144 (93.59%)**; unused: **16,810**.
- No merge-provenance metadata is stored in `.ntok`, and the training-time pair index is discarded; a per-merge provenance/marginal-utility audit is therefore **not reconstructible post-hoc** from artifacts. It would require a trainer change and a full retrain, and is out of scope for the remaining budget. Recorded as an explicit limitation.
- A 64K-scale observation from pilot A (unused vocab 1,341 of 65,536 = 2.05%) is consistent with 256K utilization being healthy; the v0.3 model is not capacity-starved at 256K.
- Intermediate merge tokens: correctly excluded from any "unused token" pruning analysis — a token can be absent from the final stream while its merge is required for building other merges. No removal of any kind was performed or proposed.

## 7. Encoding memory observation (pilot stage)

Peak RSS during 64K pilot training (mwf=2):

| Pilot | Manifest | Nominal bytes | Peak RSS |
|---|---|---:|---:|
| A (v0.3 mixture) | `data/v0.3/manifest/manifest_corpus.json` | 1.96 GB | 4.12 GB |
| B (plain expansion) | `data/v0.4-v2/manifest/manifest_corpus.json` | 3.45 GB | **5.84 GB** |
| C (guarded, code/stem ×2) | `data/v0.4-v2/manifest/manifest_guard.json` | 3.02 GB (weighted) | **6.63 GB — over ceiling; stopped analysis, archived** |

A 256K-parameter run on the plain expanded manifest projects to ≈5.5–6.5 GiB (v0.3's 3.70 GiB scaled by unique-word count 5.70M→8.33M, ×1.46). **No 256K run was attempted**: the pilot gains do not justify the memory risk (see EXPERIMENT_LOG.md).
