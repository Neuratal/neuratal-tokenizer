# NTokenizer v0.4 — Benchmark v1 Specification

Date: 2026-11-15
Status: implemented; all artifacts rebuilt and leakage-audited on 2026-11-15.

## 1. Motivation

The v0.3-era benchmark (`data/v0.2/clean/benchmark`, 544 files, 207,712,944 bytes)
was produced to evaluate v0.2 against published baselines, and it has since been
used for every intermediate decision in v0.2 → v0.3 → v0.4. It remains the
*reference* set for continuity, but it is no longer a pristine holdout for this
project: it guided which arms were trained, what caps to choose, and where to
look. Benchmark v1 is designed so that:

1. every candidate for v0.4 is chosen on **validation** data that is disjoint
   from both the training corpus and the final holdout;
2. the **final holdout** is used only for the reported final numbers;
3. both sets are independent of all prior NTokenizer releases (v0.1/v0.2/v0.3
   never trained on any of these files, by construction);
4. comparisons against external tokenizers (o200k, cl100k, GPT-2, Qwen) are fair
   (external models never saw any of our data either).

## 2. Layout

```
data/v0.4/
  benchmark/holdout/<bucket>/*.txt      FINAL holdout (untouched during development)
  benchmark/validation/<bucket>/*.txt   validation split (used for experiment decisions)
  manifest/manifest_holdout.json        holdout manifest (bucket/domain/source/license/bytes/sha256)
  manifest/manifest_validation.json     validation manifest
  manifest/manifest_holdout_full.json   holdout + synthetic Unicode stress fixtures
  manifest/manifest_validation_full.json validation + stress fixtures
  manifest/holdout_exclusion.txt        files excluded from the v0.4 training corpus
```

Synthetic Unicode stress fixtures (43 categories: flag/ZWJ sequences, keycap,
skin tones, variation selectors, combining marks in Latin/Devanagari/Arabic/Thai,
Turkish İ/ı, CJK ext-B..F, Hangul, NUL/control bytes, bidi marks, long lines,
lone CR/LF/CRLF/NEL/LS/PS, single char, empty line, pure CJK line) are generated
by `experiments/v0.4/build_unicode_stress.py` and copied into
`benchmark/holdout/unicode_stress/`. They are license-free (synthetic) and their
manifest lives at `data/v0.4/manifest/manifest_unicode_stress.json`.

## 3. Selection rules

- Source: `data/v0.2/clean/train` (5.14 GB, 12,590 files) — the same
  leakage-audited tree used for training. **v0.1/v0.2/v0.3 never trained on any
  of these files except where marked `carved_from_training: true` (see §4).**
- Per bucket, files are ordered by SHA-256 of the repository-relative path and
  taken until the byte target is met (holdout ≈ 3.5 MB, validation ≈ 1.2 MB per
  bucket; `other_languages` smaller targets).
- ArXiv papers are split into many per-section files; those are **grouped by
  paper ID** and selected/rejected as a unit, then all sections of a selected
  paper (used by any prior train manifest or not) are excluded from v0.4
  training. This prevents same-paper sibling leakage.
- Deterministic: re-running the builder reproduces the same files and manifests.

## 4. Fairness labels (important!)

The holdout deliberately contains both kinds of entries:

| Pool | Meaning | Fair for comparing |
|---|---|---|
| unused pool | files in neither the v0.2 nor the v0.3 manifest (arabic, english, hindi, japanese, korean, spanish, stem, turkish) | v0.2 vs v0.3 vs v0.4 vs externals — fully fair |
| carved pool | files consumed by v0.2/v0.3 (chinese, programming, other_languages, some stem papers); excluded from the v0.4 training corpus but **seen by v0.2/v0.3** | only externals and v0.4 (carved away) vs anything; v0.2/v0.3 are advantaged |

`carved_from_training` is recorded in every manifest entry. Bucket-level tables in
the reports mark the carved buckets; headline claims that rely on those buckets
(e.g., "same performance as v0.3 on code") must consider that v0.3/v0.2 are
contaminated there (they saw these files in training), so we report both old-
benchmark (fair for all) and new-holdout (fair for all *except* those buckets for
v0.2/v0.3).

## 5. Leakage audit

`data/v0.2/tools/leakage_audit.py` was run on the v0.4 training corpus (the v0.3
manifest minus `holdout_exclusion.txt`, symlink farm `data/v0.4/_audit_train_farm`)
against the holdout directory:

```
A (file-exact)     0
B (normalized)     0
C (≥100-char line overlap, boilerplate in ≥50 train files)  0
D (bottom-64 shingle containment, threshold 0.6)  max 0.594, 0 violations
verdict: passed
```

Artifact: `reports/v0.4/leakage_audit.json`. The 0.594 maximum containment is the
same bound the v0.2 audit produced and is dominated by
machine-generated/template source files (codec tables, generated SSA tests), not
natural-language content.

## 6. Metrics and statistics

For every model evaluated on a benchmark:

- total bytes, total tokens, total words (Unicode whitespace split)
- byte-weighted B/T = total_bytes / total_tokens (primary)
- equal-category B/T = arithmetic mean of per-bucket B/T (secondary)
- tokens/byte, tokens/word, used vocabulary and usage %, per-bucket tables
- paired file-level bootstrap (≥1000 resamples) for 95% confidence intervals on
  byte-weighted B/T and on the equal-category mean
- per-file latency, encode/decode MB/s and tokens/s (single- and multi-thread,
  ≥5 warm runs, median reported)

Reports always show per-bucket results; aggregate means are never presented
alone.

## 7. External reference tokenizers

- `o200k_base` and `cl100k_base` — tiktoken (MIT), encodings from the official package
- GPT-2 `r50k_base` — tiktoken
- Qwen2 tokenizer (if it loads in `tokenizers` with a pinned HF revision) — external
- all measured on identical benchmark bytes with identical manifests; revision
  recorded in the experiment log

## 8. Use during development

- `validation` split: may be consulted freely for experiment decisions; it also
  appears in `manifest_validation_full.json` with the stress fixtures.
- `holdout` split: never used for any decision. Final reported numbers were
  produced by exactly one run per model after the results were fixed.
- Bucket composition per split is reported in `reports/v0.4/BENCHMARK_SPEC.md`
  artifacts (`data/v0.4/manifest/*.json`) together with sizes and licenses.
