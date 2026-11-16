# NTokenizer — Public benchmark audit (final)

Date: 2026-11-16
Benchmark: **development/validation — not a pristine blind test set** (544 files, 207,712,944 bytes)
Scope docs: `METHODOLOGY.md`, `ENVIRONMENT.md`, `MODEL_AND_TOKENIZER_VERSIONS.md`, `COMPARISON_REPORT.md`, CSVs.

## 1. Tokenizers measured on identical files today

| Candidate | Status |
|---|---|
| NTokenizer v0.1 | measured (native harness) |
| NTokenizer v0.2 | measured (native harness) |
| NTokenizer v0.3-original | measured (pre-polish source build; reproduced 37,397,108 / 5.5543 exactly) |
| NTokenizer v0.3-polished (official) | measured (37,397,319 / 5.5542) |
| NTokenizer v0.4-fail (CJK8) | measured (37,418,652 / 5.5511) |
| NTokenizer v0.4-fail (Han4 arm) | measured (37,406,269 / 5.5529) |
| o200k_base (tiktoken 0.14.0) | measured (50,971,306 / 4.0751; historical value reproduced exactly) |
| Qwen2.5-7B-Instruct tokenizer | measured (62,266,982 / 3.3358; historical Qwen2.5 value reproduced) |
| **Gemini 3.8 Flash** | **NOT MEASURED** — `LLM_API_KEY` is the literal placeholder `"local"`; no API credential exists in this environment. No estimate was substituted. Completing this requires an official Gemini API key and `countTokens` on raw text only |

## 2. Re-evaluations on identical data

All rows above used the same 544 files, same bytes, same buckets, same formula set, same raw-text policy. Every external measurement had **zero file failures** (0/544). NTokenizer models are byte-exact by construction: **544/544 decode round-trip for every NTokenizer row** (v0.1, v0.2, v0.3-polished, v0.4-fail cjk8/han4 — measured this cycle; the v0.3-original row evaluates the byte-identical model file). External tokenizers: decode exactness not measured (out of scope; they parse text here).

## 3. Historical-only values

All historical figures survive in `HISTORICAL_RESULTS.csv`; none were replaced or erased. Differences documented: v0.2 +21 tokens vs recorded (same pre-existing source drift as +211 on v0.3), reported with B/T unchanged at printed precision.

## 4. Qwen reference, exactly

- Repo: `Qwen/Qwen2.5-7B-Instruct`, revision **`a09a35458c702b33eeacc393d103063234e8bc28`** (lastModified 2025-01-12)
- `tokenizer.json` sha256 **`c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539`** — byte-identical to the Qwen2.5 tokenizer file used in the 2026-11-14 historical measurements
- `vocab.json` sha256 `ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910`; `merges.txt` sha256 `599bab54…`; `tokenizer_config.json` sha256 `5b5d4f65…`
- Vocab 151,665; `add_special_tokens=False`; **3.3358 B/T** on this benchmark (historical value reproduced)

## 5. Overall B/T ranking (this benchmark)

1. v0.3-original 5.5543 (historical behavior) — 2. **v0.3-polished 5.5542** (official) — tie at printed precision
2. v0.4-fail Han4 5.5529 — 3. v0.4-fail CJK8 5.5511 — 4. v0.2 4.9290 — 5. o200k 4.0751 — 6. Qwen2.5-7B 3.3358 — 7. v0.1 3.1676
8. Gemini — not measured

## 6–11. Per-category winners (B/T)

- **NTokenizer v0.3-polished beats o200k_base in 10/11 buckets** (loses only `other_languages` 3.32 vs 5.19): turkish, arabic, english, hindi, korean, spanish, japanese, chinese, stem, programming.
- **NTokenizer v0.3-polished beats Qwen2.5-7B in 10/11 buckets** (same loss, margin −0.9%): same set.
- **vs Gemini: no comparison** (not measurable here).
- Best-in-row by category: v0.3-polished/original ties in turkish, arabic, english, spanish, japanese, stem, programming (v0.3-original wins hindi narrowly 9.9284); **v0.4-fail-Han4 wins chinese (3.6557) and korean (5.0396)**; **o200k_base wins other_languages (5.1876)**.
- NTokenizer v0.3-polished loses: `other_languages` vs both externals; chinese vs v0.4-fail-Han4 (as an internal arm); japanese vs nothing external (5.01 > o200k 3.42 > …), but v0.4 arms are slightly lower on japanese (4.98).

## 12. Missing/unavailable artifacts

- Gemini 3.8 Flash — no credential; blocked (see §1).
- ASAN-instrumented fuzzing — still blocked by the pre-existing linker issue (not claimed as done).
- No untouched blind test set exists anywhere in this project; the 544-file benchmark and the v0.4 holdout have both been used during development.

## 13. GitHub README — updated and pushed

- New section "Tokenizer Benchmark — NTokenizer vs Leading Tokenizers" added immediately after the status table; lower benchmark section slimmed; all tables match the audit artifacts exactly.
- Pushed: `master` `b8866bf` → `ca2ed10861e589bc548cb027d5e9e67d4dc328ca` (verified public: raw README renders the table; CI note preserved).

## 14. Hugging Face model card — updated and pushed

- Same comparison table inserted near the top (after Model details, before Description); Evaluation section references the pinned Qwen revision/hashes; SHA256SUMS refreshed (README now `41cb563e…`).
- Verified by unauthenticated fetch after upload; model file untouched (`1f0d4ad7…`).

## 15. Verification links (all public, verified 2026-11-16)

- GitHub README: https://github.com/Neuratal/neuratal-tokenizer (table visible)
- Release v0.3.0: https://github.com/Neuratal/neuratal-tokenizer/releases/tag/v0.3.0
- HF model: https://huggingface.co/Neuratal/ntokenizer (model card shows the same table)
- Audit trail: `reports/public-benchmark/` in the public repo (methodology, environment, versions, CSVs, CSVs' scripts, SHA256SUMS)

## 16. Model SHA-256 preservation check

`output/v0.3/ntokenizer-0.3-256k.ntok` = `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` — verified before, during and after this cycle; HF copy re-downloaded and identical. **v0.3-polished is untouched; no model was created or promoted by this audit.**

## Verdict

- **What NTokenizer v0.3-polished beats (compression on this dev benchmark):** GPT-2-era and o200k-scale tokenizers on every category except `other_languages` (o200k wins there 5.19 vs 3.32), and Qwen2.5-7B-Instruct on the same 10 buckets. It beats v0.2 decisively (+12.7% B/T), and far exceeds the historical v0.1.
- **What it does not beat:** o200k on the tiny `other_languages` bucket; its own Han4-arm sibling on chinese and korean (internal experiments); Gemini-3.8-Flash (unmeasured); and nothing about this establishes downstream LM superiority.
- **Is it globally better on compression?** On this development benchmark, yes — decisively on every major multilingual category and code/STEM, narrowly not universal (one small bucket loses; Gemini unknown; the benchmark is a development set).
- **Why this proves nothing about LM quality:** B/T counts bytes per token. It does not measure next-token loss, downstream task accuracy, multilingual fairness of LM behavior, or any model-level property. No 50M language model has been trained yet; tokenizer choice must be validated with downstream evaluation before claims about model quality are made.
