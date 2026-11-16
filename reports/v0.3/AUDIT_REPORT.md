# NTokenizer v0.3 Audit Report

Date: 2026-11-15  
Auditor: automated v0.3 work session  
Baseline artifacts: `output/v0.2/ntokenizer-0.2-256k.ntok` (immutable), `data/v0.2/manifest/manifest_benchmark.json`

## 1. Scope

This audit re-measures the v0.2 tokenizer instead of trusting the prior report,
checks the training/benchmark separation, quantifies unused vocabulary and
unused corpus headroom, and identifies the changes most likely to improve
held-out compression in v0.3. No v0.2 file is modified by this work.

Environment:

| Item | Value |
|---|---|
| CPU | Intel Core i7-12700H (14 cores / 20 logical threads) |
| RAM | 14 GiB |
| OS / kernel | Linux (Ubuntu), x86_64 |
| Rust | rustc 1.99.0, cargo 1.99.0 |
| Python env | `.venv` with `tiktoken==0.14.0`; `/tmp/opencode/hfvenv` with `tokenizers==0.23.3` + `transformers==5.23.0` (HF-export validation only) |
| Benchmark | `data/v0.2/clean/benchmark`, 544 files, 207,712,944 bytes |
| Benchmark manifest | `data/v0.2/manifest/manifest_benchmark.json`, SHA-256 `002a8224…e255103c8` |
| v0.2 model | `output/v0.2/ntokenizer-0.2-256k.ntok`, SHA-256 `1fc3ee85…d29ab54ac` |

The benchmark is held out. Every benchmark file lives under
`data/v0.2/clean/benchmark`; every training file lives under
`data/v0.2/clean/train`. The v0.2 corpus construction and leakage audit are
documented in `data/v0.2/` and `reports/v0.2/`; v0.3 reuses that separation and
never draws training bytes from the benchmark tree.

## 2. How v0.2 was re-measured

The audit added a native Rust harness
(`examples/benchmark_ntok.rs`) that:

- loads the `.ntok` model through the public library API;
- reads the benchmark manifest and uses its explicit `bucket` field;
- preloads every file so filesystem I/O is outside the measured window;
- encodes raw bytes with `Tokenizer::encode_bytes`;
- aggregates bytes and tokens overall and per bucket;
- reports both **byte-weighted** bytes/token (total bytes / total tokens) and
  **equal-category mean** bytes/token (arithmetic mean of the 11 bucket
  values);
- measures used vocabulary IDs with a bitset, per-file latency, warm-run
  medians, encode/decode throughput and process peak RSS;
- verifies deterministic token counts across repeated runs and checks
  `decode(encode(bytes)) == bytes` for every file.

Command used for the independent v0.2 check (single-thread, one measured run,
decode disabled because it was validated separately):

```bash
RAYON_NUM_THREADS=1 target/release/examples/benchmark_ntok \
  --model output/v0.2/ntokenizer-0.2-256k.ntok \
  --manifest data/v0.2/manifest/manifest_benchmark.json \
  --repeats 1 --warmup 0 --no-decode \
  --out /tmp/opencode/ntok/verify_v02.json
```

Result: **42,140,653 tokens / 207,712,944 bytes = 4.9290 B/T**, equal-category
**4.6857 B/T**, 232,659 of 262,144 vocabulary IDs observed (88.75%). These
match the prior v0.2 report exactly. The measurement is therefore reproducible
and the prior report's headline number was not an artifact of its runner.

## 3. Verified baseline

### 3.1 NTokenizer v0.2 vs o200k_base

o200k_base was measured independently with `tiktoken` using the same manifest
and the same byte counts; see `experiments/v0.3/results/o200k_baseline.json`.

| Metric | v0.2 256K | o200k_base | v0.2 advantage |
|---|---:|---:|---:|
| Tokens | 42,140,653 | 50,971,306 | −8,830,653 tokens (−17.3%) |
| Byte-weighted B/T | **4.9290** | 4.0751 | **+20.95%** |
| Equal-category B/T | **4.6857** | 4.0899 | **+14.57%** |
| Tokens/byte | 0.20292 | 0.24540 | −17.3% |
| Used vocabulary | 232,659 / 262,144 | 133,550 / 200,019 | — |
| Vocabulary usage | 88.75% | 66.77% | — |

Note: the o200k used-vocabulary figure is the number of IDs actually observed
on this benchmark, not its full 200,019-token table. o200k is much more
vocabulary-sparse on this multilingual benchmark than v0.2, which is one reason
v0.2's larger table still pays off.

### 3.2 Per-bucket B/T

| Bucket | Bytes | v0.2 B/T | o200k B/T | v0.2 vs o200k |
|---|---:|---:|---:|---:|
| english | 32.06 MB | 4.0521 | 4.1537 | −2.4% |
| turkish | 43.00 MB | 4.5385 | 3.4472 | +31.7% |
| arabic | 40.60 MB | 6.0465 | 4.7628 | +27.0% |
| hindi | 29.24 MB | 8.6734 | 7.2370 | +19.8% |
| korean | 25.26 MB | 4.4679 | 3.2668 | +36.8% |
| spanish | 15.89 MB | 4.3716 | 4.1477 | +5.4% |
| chinese | 2.95 MB | 3.7545 | 2.7483 | +36.6% |
| japanese | 10.14 MB | 5.3766 | 3.4240 | +57.0% |
| programming | 4.10 MB | 3.2120 | 3.6773 | −12.7% |
| stem | 4.32 MB | 3.1015 | 2.9363 | +5.6% |
| other_languages | 0.16 MB | 3.9475 | 5.1876 | −23.9% |

v0.2's byte-level architecture is strongest on Turkish, Arabic, Korean,
Chinese and Japanese, but it trails o200k on English, programming, and the tiny
`other_languages` bucket. The aggregate lead is carried by the high-byte
Turkish/Arabic/Hindi/Korean buckets and by Japanese.

### 3.3 Target arithmetic

The original aggressive target was 2× o200k overall B/T, i.e. **8.1502 B/T**.
On the same 207.7 MB benchmark that requires about **25.49 M tokens**, versus
42.14 M for v0.2. That is a further 39.5% token reduction relative to v0.2.
Given the measured vocabulary scaling in §4, this target is not supported by
the available evidence and should be treated as a stretch research question,
not a promised deliverable. v0.3 will report the achieved value honestly.

## 4. Vocabulary scaling on the same merge order

The v0.2 release contains exported models at several vocabulary sizes. They
share one merge order, so they isolate the value of vocabulary size on the same
corpus and benchmark.

| Vocab | Tokens | Byte-weighted B/T | Equal-category B/T | Used vocab |
|---:|---:|---:|---:|---:|
| 8,192 | 69,457,972 | 2.9905 | 2.8091 | 98.84% |
| 32,768 | 54,347,180 | 3.8220 | 3.5823 | 98.61% |
| 65,536 | 49,116,692 | 4.2290 | 3.9733 | 97.22% |
| 131,072 | 45,091,962 | 4.6064 | 4.3511 | 94.74% |
| 196,608 | 43,274,466 | 4.7999 | 4.5487 | 91.73% |
| 262,144 | 42,140,653 | 4.9290 | 4.6857 | 88.75% |

Each doubling of vocabulary buys progressively less: +27.8% B/T from 8K→32K,
+10.7% 32K→64K, +8.9% 64K→128K, +4.2% 128K→196K, +2.7% 196K→256K. Extrapolating
the same trend, 512K might add roughly 1–3%, not the ~65% needed for 8.15 B/T.
Vocabulary growth alone cannot reach 2×.

## 5. The unused-vocabulary finding

v0.2 trained 261,881 merges and exported a 262,144-token vocabulary, but only
232,659 IDs are used anywhere on the benchmark; **29,485 IDs (11.25%) are
unused**. Two effects contribute:

1. merges learned on one language/domain are never selected on this benchmark;
2. the training corpus is not balanced toward the benchmark's language mix, so
   vocabulary capacity is spent on Chinese Gutenberg prose and English books
   that are under-represented in the held-out bytes.

Reallocating even half of the unused capacity to Turkish, Arabic, Hindi,
Korean, code and math is a plausible source of several percent B/T.

## 6. The corpus-headroom finding

`data/v0.2/clean/train` contains **5.137 GB in 12,590 files**, while the v0.2
manifest selected **1.0188 GB in 8,675 files**. Available bytes by bucket:

| Bucket | v0.2 training bytes | Available clean bytes | Headroom |
|---|---:|---:|---:|
| english | 260.1 MB | 831.5 MB | 3.2× |
| turkish | 94.7 MB | 1,009.0 MB | 10.7× |
| arabic | 38.4 MB | 935.1 MB | 24.3× |
| hindi | 53.1 MB | 654.6 MB | 12.3× |
| korean | 58.9 MB | 529.1 MB | 9.0× |
| spanish | 49.6 MB | 348.5 MB | 7.0× |
| japanese | 63.5 MB | 269.3 MB | 4.2× |
| chinese | 166.0 MB | 165.7 MB | 1.0× |
| programming | 140.4 MB | 140.7 MB | 1.0× |
| stem | 90.4 MB | 238.6 MB | 2.6× |
| other_languages | 3.6 MB | 3.6 MB | 1.0× |

The largest misallocations are Arabic (24× headroom unused), Turkish (10.7×),
Hindi (12.3×), Korean (9×) and Spanish (7×). Chinese and programming were
already saturated in v0.2, and Chinese Gutenberg alone accounts for 151 MB of
the 166 MB Chinese training budget while the benchmark has only 2.95 MB of
Chinese text. This is the single largest data lever for v0.3.

## 7. Trainer memory and throughput audit

v0.2's published run: 13,885,542 records, 1,018,783,149 bytes, 10,820,251 unique
pre-tokens, 261,881 merges, ~807.6 s wall, peak RSS ~9.7 GB. Re-measured pilot
runs show the same shape:

| Run | Corpus | Unique pre-tokens | Merges | Wall | Peak RSS |
|---|---:|---:|---:|---:|---:|
| v0.2 (published) | 1.019 GB | 10,820,251 | 261,881 | 807.6 s | ~9.7 GB |
| pilot lws 32K | 210.4 MB | 2,584,248 | 32,505 | 145.4 s | 2.49 GB |
| pilot hybrid 32K | 210.4 MB | 3,484,787 | 32,505 | 146.8 s | 2.67 GB |
| pilot code 32K | 210.4 MB | 3,552,531 | 32,505 | 190.2 s | 3.15 GB |

Peak RSS scales with unique pre-tokens and pair occurrences, not raw corpus
bytes. On a 14 GiB machine, a 2 GB corpus at the v0.2 ingestion strategy is
OOM-risky. v0.3 therefore adds an explicit **minimum word frequency** floor
(default 1 = legacy behavior) so singleton typos, hashes and one-off
identifiers can be pruned before the pair index is built. This must be measured
for quality impact, not assumed free.

The current trainer also pays for:

- `HashMap<Vec<u8>, usize>` word counts during ingestion (one heap allocation
  per unique pre-token);
- `HashMap<(u32,u32), Vec<Occurrence>>` pair occurrences and a `BinaryHeap`
  with one entry per pair-frequency update;
- full `(u32,u32)` keys in every occurrence list.

The v0.3 plan treats trainer compaction as an optimization, not a prerequisite
for the first measured corpus-scaling runs. If the frequency floor keeps peak
RSS within budget, quality experiments take priority over a risky rewrite.

## 8. Pretokenizer audit

v0.2's default `LeadingWhitespaceUnicode` (LWS) attaches leading spaces/tabs to
the following lexical run, keeps script runs separate, keeps digit runs whole,
and attaches combining marks/ZWJ. It has a specific blind spot: **trailing
punctuation is always a separate pre-token**. `"world."` becomes `"world"` +
`"."`, so the pair `l` + `.` can only be learned as a token that starts with a
period, not as `"world."`. It also cannot learn `"(hello"`, `"don't"` or
`"state-of-the-art"` as single pre-tokens.

The audit compared the prior candidates only at toy vocabulary sizes. v0.3
implements and measures at 32K/64K on the same balanced slice:

- `ScriptTransitionAware`: LWS minus space attachment (ablation);
- `UnicodeHybrid`: context-aware brackets, trailing punctuation, internal
  connectors (apostrophe/hyphen/slash/dot/colon/underscore/@/&), decimal/time
  numbers, prefix/suffix symbols;
- `CodeAware`: identifier characters (alnum, `_`, `$`, internal `'`/`.`),
  operator runs, LaTeX commands, indentation attachment;
- `MathSymbolAware`: decimal/time/grouped thousands, LaTeX commands, operator
  binding (`x^2`, `a+b`), prefix/suffix symbols.

The existing modes (`RawByte`, `ConservativeUnicode`,
`ScriptAndDigitAware`, `LeadingWhitespaceUnicode`) are untouched, preserving
v0.1/v0.2 encode compatibility. See `EXPERIMENT_RESULTS.csv` and
`EXPERIMENT_PLAN.md` for measured outcomes.

## 9. Benchmark coverage audit

The held-out benchmark has 11 buckets but is uneven:

- Chinese is represented by only 2.95 MB; the Chinese B/T estimate has a wide
  uncertainty and should not be over-interpreted.
- `other_languages` is 0.16 MB of Tatoeba sentences. It is too small for a
  stable per-language claim; it is reported as a bucket but never used alone to
  declare a language victory.
- Programming is 4.10 MB, STEM 4.32 MB. These are adequate for a directional
  comparison but not for fine-grained claims about every language.
- French/German/Russian/etc. appear only in the tiny Tatoeba tail. v0.3 adds no
  new benchmark data; adding data would change the benchmark and invalidate
  comparability. Any additional languages must be reported separately.

The v0.3 reports therefore use both byte-weighted and equal-category means and
show the full per-bucket table for every candidate. No category is silently
dropped.

## 10. Risks and audit conclusions

1. **Metric risk:** byte-weighted B/T rewards the high-byte buckets. Equal-
   category B/T is reported alongside it to prevent a single language from
   hiding regressions.
2. **Overfitting risk:** v0.3 corpus caps are informed by bucket headroom, not
   by benchmark text. Train/benchmark separation is unchanged. We do not train
   on benchmark files, benchmark-derived n-grams, or benchmark-tuned per-file
   weights.
3. **Memory risk:** the trainer is the bottleneck. A frequency floor is the
   first mitigation; compact pair storage is a fallback.
4. **Compatibility risk:** adding pretokenizer enum variants and metadata
   strings must not change the behavior of existing models. Existing modes are
   frozen and covered by regression tests.
5. **License risk:** all added corpus bytes come from the already-audited
   v0.2 clean tree. New dependencies are permissively licensed; details are in
   `LICENSE_AUDIT.md`.
6. **Target risk:** 2× o200k (8.15 B/T) is not supported by measured scaling.
   v0.3 will report the real number and explain the gap.

The audit supports proceeding with: (a) a balanced ~2 GB v0.3 manifest,
(b) the structured pretokenizer candidates, (c) a frequency floor for trainer
memory, and (d) a full 256K training run, with 64K pilots used to choose the
pretokenizer before committing the large run.
