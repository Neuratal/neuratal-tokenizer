# NTokenizer v0.3 Experiment Plan

Date: 2026-11-15  
Status: executed; see `EXPERIMENT_RESULTS.csv` for outcomes and
`EXPERIMENT_RESULTS.md` for the narrative.

## 1. Research question

Can a retrained NTokenizer with (a) a better-corpus manifest, (b) a
context-aware pretokenizer, and (c) the existing 256K byte-level BPE vocabulary
beat the verified v0.2 baseline and o200k_base on the fixed held-out benchmark,
without changing the benchmark, leaking training data, or manipulating the
metric?

Primary metric: **equal-category bytes/token** (arithmetic mean of the 11
benchmark buckets). Secondary: byte-weighted B/T. Both are reported for every
candidate, along with per-bucket B/T, token counts, used vocabulary, encode and
decode throughput, latency and peak RSS.

Stretch question from the brief: 2× o200k overall B/T = **8.1502 B/T**. The
audit's vocabulary-scaling data (8K→256K on the same merge order) does not
support that target; it is treated as a hypothesis to test, not an expected
result.

## 2. Controlled variables

All comparisons use:

- the same benchmark: `data/v0.2/clean/benchmark`, 544 files, 207,712,944 bytes;
- the same benchmark manifest (explicit bucket mapping);
- the same training seed (42), pair-frequency floor (`--min-freq 2`), and
  vocabulary budget for a given experiment;
- the same deterministic merge tie-break (highest frequency, then smaller token
  IDs);
- the same native harness (`examples/benchmark_ntok.rs`) and byte-level
  encoding (`encode_bytes`);
- the same held-out discipline: no benchmark text in any training manifest.

Only the declared variable (pretokenizer, corpus size, vocabulary size, or
minimum word frequency) changes between arms.

## 3. Data

### 3.1 Source

`data/v0.2/clean/train`: 5.137 GB, 12,590 files, already cleaned and
leakage-audited against the benchmark. v0.3 does not download new training text
for the primary runs, so the license provenance is inherited from v0.2 and
documented in `data/v0.2/LICENSES.md` and `LICENSE_AUDIT.md`.

### 3.2 Pilot slice

`experiments/v0.3/pilot_manifest_20mb.json`: 210.38 MB, equal ~20 MB per
bucket, selected deterministically by SHA-256 of the repository-relative path.
Used for the pretokenizer tournament at 32K.

`experiments/v0.3/pilot_manifest_60mb.json`: 631.63 MB, equal ~60 MB per
bucket, same selection rule. Used to measure corpus scaling at 64K.

### 3.3 Production manifest

`data/v0.3/manifest/manifest_corpus.json`: 1,963.48 MB, 10,555 files, with
per-domain caps chosen from the audit's headroom table:

| Bucket | MB | Main sources |
|---|---:|---|
| turkish | 287.19 | trwiki, trwikisource, trwiktionary, Tatoeba |
| arabic | 293.65 | arwikisource, Tatoeba |
| hindi | 230.57 | hiwiki, Tatoeba |
| english | 218.95 | simplewiki, enwikinews, enwikibooks, Gutenberg |
| stem | 189.08 | arXiv, Gutenberg science |
| korean | 187.27 | kowikisource, kowiktionary, Tatoeba |
| spanish | 147.08 | eswikisource, Tatoeba |
| programming | 140.65 | CPython, Rust, Go, TypeScript |
| japanese | 135.09 | jawikisource, jawiktionary, Gutenberg |
| chinese | 130.30 | Gutenberg CJK, zhwiktionary |
| other_languages | 3.64 | all Tatoeba tails |
| **total** | **1,963.48** | |

Selection is deterministic and reproducible:

```bash
python3 experiments/v0.3/build_v03_manifest.py --profile final \
  --out data/v0.3/manifest/manifest_corpus.json \
  --stats reports/v0.3/corpus_distribution.json
```

The v0.2 benchmark manifest is reused unchanged for v0.3 evaluation.

## 4. Pretokenizer candidates

| ID | Mode | Hypothesis |
|---|---|---|
| P0 | `RawByte` | No boundaries maximizes possible merges; expected memory/time blow-up on long records. |
| P1 | `ConservativeUnicode` | Legacy v0.1 category split; control. |
| P2 | `ScriptAndDigitAware` | Script isolation + individual digits; control. |
| P3 | `LeadingWhitespaceUnicode` (LWS) | v0.2 default; leading-space attachment control. |
| P4 | `ScriptTransitionAware` | LWS without space attachment; isolates the value of space attachment. |
| P5 | `UnicodeHybrid` | Brackets, trailing punctuation and internal connectors become part of the lexical pre-token. |
| P6 | `CodeAware` | Identifier/operator/LaTeX rules; expected to help programming and mixed text. |
| P7 | `MathSymbolAware` | Decimal/time/operator binding and prefix/suffix symbols; expected to help STEM and code. |

All new modes use Unicode general categories and Unicode scripts from
`unicode-general-category` and `unicode-script`; legacy modes remain frozen.

### 4.1 Tournament design

- **Round 1 (32K):** P3, P5, P6, P7 on the 210 MB pilot. P4 and P0 as
  ablations/feasibility. Fixed `min-word-freq=1`. The legacy v0.1 controls
  (P1 ConservativeUnicode, P2 ScriptAndDigitAware) were also re-measured on the
  same slice for completeness (2.9221/2.9530 and 2.8114/2.8447).
- **Round 2 (64K):** winner of Round 1 plus P3 control, on the 60 MB pilot.
  This checks that the 32K advantage survives a larger vocabulary and more data.
- **Corpus-scaling check:** the same 64K candidate on 20 MB vs 60 MB per bucket
  to estimate whether the 1.96 GB production manifest is worth the compute.
- **Frequency-floor check:** the winning candidate at 64K with
  `--min-word-freq 2` vs 1 to measure quality and memory impact.
- **Final:** winning pretokenizer at 256K on the production manifest, compared
  with v0.2 and o200k on the full held-out set.

Acceptance for a candidate: improves equal-category B/T over LWS on the same
slice; no bucket regression larger than 2% without an explicit trade-off
explanation; trainer peak RSS stays below ~12 GiB on the 14 GiB machine. A
candidate that only wins on the byte-weighted metric by shifting mass to one
language is rejected.

## 5. Measurements

For each trained model:

- `.ntok` SHA-256 and size; training wall time, user/system CPU time, peak RSS;
- total training bytes, records, unique pre-tokens, pruned pre-tokens (if any),
  merges learned, final vocabulary size;
- held-out token count, byte-weighted B/T, equal-category B/T, tokens/byte,
  tokens/word;
- per-bucket bytes, tokens, B/T;
- used/unused vocabulary IDs and usage percentage;
- encode MB/s and tokens/s (single-thread and multi-thread), decode MB/s and
  exactness;
- per-file latency median/p95/max;
- ≥5 warm-up-adjusted measured runs with median reported for the final model;
- peak inference RSS and model load time.

The native harness is validated against the independently reproduced v0.2
number before any candidate is trusted (42,140,653 tokens; 4.9290 B/T).

## 6. Leakage and honesty controls

- Training and benchmark trees are physically separate; no manifest entry points
  into `clean/benchmark`.
- The v0.2 leakage audit remains the authority for near-duplicate isolation.
  v0.3 changes only which clean-training files are selected.
- Benchmark buckets come from the benchmark manifest, not from filename
  heuristics.
- No per-file weights, no benchmark-specific vocabulary injection, no
  post-hoc token-count selection.
- Failed or aborted experiments (e.g. RawByte if it exceeds memory) are
  reported as failures, not silently dropped.
- If the final model misses 8.15 B/T, the report states the gap and the measured
  scaling evidence; it does not relabel the benchmark or the metric.

## 7. Compute and artifact policy

- Existing v0.1/v0.2 model files are read-only; no experiment writes into
  `output/v0.2/` or `output/ntokenizer-256k.ntok`.
- New models go to `experiments/v0.3/models/` (pilots) and `output/v0.3/`
  (release candidates).
- Logs go to `experiments/v0.3/logs/`, machine-readable results to
  `experiments/v0.3/results/`, reports to `reports/v0.3/`.
- No GitHub push or Hugging Face upload is performed without user approval.
