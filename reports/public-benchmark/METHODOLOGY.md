# Methodology — NTokenizer public benchmark comparison

Date: 2026-11-16
Scope: apples-to-apples comparison on the repository's 544-file **development/validation benchmark**.

**Development/validation benchmark — not a pristine blind evaluation set.** It has been used
throughout NTokenizer development. Numbers on it measure compression efficiency on this fixed,
mostly-wiki/STEM corpus, not generalization or language-model quality.

## 1. Input

- Manifest: `data/v0.2/manifest/manifest_benchmark.json` — 544 files, **207,712,944 bytes**,
  SHA-256 `002a8224671cd281312e059af85dad24c269e7a3945e55724d476e3e255103c8`.
- Category assignments (bucket) from the manifest, unchanged: turkish 20.7%, arabic 19.5%,
  english 15.4%, hindi 14.1%, korean 12.2%, spanish 7.6%, japanese 4.9%, stem 2.1%,
  programming 2.0%, chinese 1.4%, other_languages 0.1% of bytes.
- The same files, byte boundaries, and bucket assignments were used for every tokenizer.

## 2. What was sent

- **Raw bytes only.** No Unicode normalization, no whitespace changes, no punctuation removal,
  no chat templates, no system prompts/roles/tools wrappers, no few-shot text.
  Special-token handling: NTokenizer is measured through its byte-level `encode_bytes` path
  (native `.ntok`, as used by `examples/benchmark_ntok`); tiktoken uses
  `allowed_special=set()` (special strings tokenized as ordinary text); HF `tokenizers` uses
  `add_special_tokens=False`.
- All 544 files are valid UTF-8; they were decoded as UTF-8 for the Python tokenizers. NTokenizer's
  native harness consumed the raw bytes directly (its decoder is byte-exact by construction).

## 3. Metric formulas

- **Byte-per-token (B/T)** = total UTF-8 bytes ÷ total tokens. Higher = better compression
  efficiency, not necessarily better LM quality.
- **Equal-category B/T** = arithmetic mean of the 11 bucket-level B/T values (each category
  weighted equally, regardless of bucket bytes).
- **Token reduction vs X (%)** = (X_tokens − candidate_tokens) / X_tokens × 100.
  Positive = fewer tokens than X; negative = more tokens than X. This is a **different** metric
  from the percentage change in B/T; the tables use it for cross-tokenizer reduction reporting
  and keep B/T for compression.
- Per bucket: `bucket_B/T = bucket_bytes / bucket_tokens`.
- Category-level token reduction vs comparator uses the same formula on bucket token counts.

## 4. Candidates (exact versions pinned in `MODEL_AND_TOKENIZER_VERSIONS.md`)

| Candidate | How evaluated |
|---|---|
| NTokenizer v0.1 / v0.2 / v0.3-polished / v0.3-original / v0.4-fail (cjk8 & han4) | Native harness `examples/benchmark_ntok` on the `.ntok` artifact (v0.3-original re-evaluated on a build of the pre-polish source, reproducing 37,397,108 exactly) |
| o200k_base | tiktoken 0.14.0, `o200k_base` encoding, `allowed_special=set()` |
| Qwen2.5-7B-Instruct tokenizer | HF snapshot `Qwen/Qwen2.5-7B-Instruct` @ `a09a35458c702b33eeacc393d103063234e8bc28`, `tokenizer.json` sha256 `c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539` (identical to the file used in the 2026-11-14 historical Qwen2.5 measurement), HF `tokenizers` 0.23.3, `add_special_tokens=False` |
| Gemini 3.8 Flash | **NOT MEASURED** — no Gemini API credential exists in this environment (`LLM_API_KEY` is the literal placeholder `"local"`); and sending the corpus to an external API was not authorized by a verified license/data-handling review. Gemini would require `countTokens` on raw text through Google's official API with a model ID recorded at measurement time; none was available. Do not substitute estimates. |

## 5. Failure policy

No file was silently excluded. External tokenizers reported zero failures (0/544) on valid UTF-8
input; decode exactness of external tokenizers was **not** measured (they parse raw text here,
and WordPiece-style normalization could differ), whereas NTokenizer models are byte-exact by
design (verified 544/544 every release).

## 6. Reproducibility

- `scripts/measure_external.py` — measures o200k_base and the pinned Qwen tokenizer.
- `scripts/aggregate_comparison.py` — aggregates to `STANDARDIZED_RESULTS.csv` and `CATEGORY_RESULTS.csv`.
- Native: `RAYON_NUM_THREADS=1|8 ./target/release/examples/benchmark_ntok --model <file> --manifest <manifest> --repeats 1 --per-file-json <out>`.
- Benchmark files and their manifest are **not** distributed; this repository contains metadata,
  counts, hashes, and code only.

## 7. Limitations

- Development/validation set, not a blind test set; every number inherits that caveat.
- `other_languages` bucket is tiny (0.16 MB, 9 files); differences there are unreliable/noisy.
- `programming` is 4.1 MB (300 files, many small); STEM is 4.3 MB (113 files).
- o200k and Qwen are English/code-heavy references on their home turf; NTokenizer's advantage on
  this multilingual benchmark does not imply universal superiority or downstream LM gains.
- Gemini remains unmeasured; the comparison is incomplete without it.
- B/T ≠ LM quality; no 50M-language-model downstream evaluation exists yet.
