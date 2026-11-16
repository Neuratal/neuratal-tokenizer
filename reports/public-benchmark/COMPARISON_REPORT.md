# NTokenizer — Public benchmark comparison report

Date: 2026-11-16
Benchmark: **development/validation benchmark — NOT a pristine blind evaluation set**
(544 files, 207,712,944 bytes, `data/v0.2/manifest/manifest_benchmark.json`,
SHA-256 `002a8224671cd281312e059af85dad24c269e7a3945e55724d476e3e255103c8`).

**Bold** = higher B/T (fewer tokens per input byte) than the comparator. The **Best B/T** column shows the highest value in the row (bolded). NTokenizer v0.3-polished is bolded only where it strictly beats the comparator — not by default. B/T measures compression efficiency, not language-model quality.

## Table A — Standardized comparison (all re-evaluated on identical files today)

| Tokenizer (exact reference) | Vocabulary | Total tokens | Overall B/T | Equal-category B/T | Token reduction vs o200k | Token reduction vs Qwen2.5-7B |
|---|---:|---:|---:|---:|---:|---:|
| NTokenizer v0.1 | 262,144 | 65,573,261 | 3.1676 | 3.1203 | −28.65% | −5.31% |
| NTokenizer v0.2 | 262,144 | 42,140,674 | 4.9290 | 4.6855 | +17.32% | +32.32% |
| NTokenizer v0.3-original (pre-polish source) | 262,144 | 37,397,108 | 5.5543 | 5.1057 | +26.63% | +39.94% |
| **NTokenizer v0.3-polished (official release)** | 262,144 | **37,397,319** | **5.5542** | **5.1056** | **+26.63%** | **+39.94%** |
| NTokenizer v0.4-fail (CJK8) | 262,144 | 37,418,652 | 5.5511 | 5.1031 | +26.59% | +39.91% |
| NTokenizer v0.4-fail (Han4 arm) | 262,144 | 37,406,269 | 5.5529 | 5.1081 | +26.61% | +39.93% |
| o200k_base | 200,019 | 50,971,306 | 4.0751 | 4.0899 | 0.00% | +18.14% |
| Qwen2.5-7B-Instruct tokenizer | 151,665 | 62,266,982 | 3.3358 | 3.3236 | −22.16% | 0.00% |
| Gemini 3.8 Flash | n/a | NOT MEASURED | n/a | n/a | — | — |

Table notes:

- v0.3-polished = the shipping release (GitHub v0.3.0; HF `Neuratal/ntokenizer`). The model is byte-identical to v0.3-original; the two rows differ only in the implementation used to evaluate it.
- v0.4-fail rows are archived *unpromoted* experiments, shown for completeness; v0.4-v2 (64K pilots) produced no 256K model and is not in this table.
- o200k_base (50,971,306) reproduces the 2026-11 historical figure exactly. Qwen2.5-7B-Instruct reproduces the historical Qwen2.5-0.5B figure (3.3358 B/T); it is the **same** tokenizer.json file (sha256 `c0382117…` — see `MODEL_AND_TOKENIZER_VERSIONS.md`).
- Gemini: no API credential exists in this environment (`LLM_API_KEY` is the literal `"local"`); no measurement was made and none is estimated.
- v0.2 measured this run shows +21 tokens vs its recorded figure (source drift, see `MODEL_AND_TOKENIZER_VERSIONS.md`); B/T is unchanged at the printed precision.

## Table B — Per-category B/T (same benchmark)

| Category | Bytes | NTokenizer v0.3-polished | o200k_base | Qwen2.5-7B-Instruct | Gemini 3.8 Flash | Best B/T (bolded) |
|---|---:|---:|---:|---:|---|---|
| English | 32.06 MB | **4.4628** | 4.1537 | 3.9483 | — | **4.4628** v0.3-polished |
| Turkish | 43.00 MB | **5.3054** | 3.4472 | 3.0067 | — | **5.3054** v0.3-polished |
| Arabic | 40.60 MB | **7.1984** | 4.7628 | 4.0455 | — | **7.1984** v0.3-polished |
| Hindi | 29.24 MB | **9.9277** | 7.2370 | 2.7666 | — | **9.9277** v0.3 (original 9.9284, tie at 4dp) |
| Korean | 25.26 MB | **4.9999** | 3.2668 | 3.0874 | — | **5.0396** v0.4-fail-Han4 |
| Spanish | 15.89 MB | **4.7363** | 4.1477 | 3.5672 | — | **4.7363** v0.3-polished |
| Japanese | 10.14 MB | **5.0135** | 3.4240 | 3.8872 | — | **5.0135** v0.3-polished |
| Chinese | 2.95 MB | **3.5380** | 2.7483 | 2.9456 | — | **3.6557** v0.4-fail-Han4 |
| STEM / Math | 4.32 MB | **3.5638** | 2.9363 | 2.4087 | — | **3.5638** v0.3-polished |
| Programming | 4.10 MB | **4.0961** | 3.6773 | 3.5472 | — | **4.0961** v0.3-polished |
| Other languages | 0.16 MB | 3.3196 | **5.1876** | 3.3486 | — | **5.1876** o200k_base |

Bold legend: **bold** = higher B/T than the comparator shown. Best-value column bolded.
NTokenizer v0.3-polished wins against the two external references in **10 of 11** categories (loses `other_languages` to both). Its single external opponent loss is the tiny `other_languages` bucket (0.16 MB / 9 files — too small to support a confident conclusion).

## Table C — Category token counts (v0.3-polished) and reduction vs reference models

| Category | Tokens (polished) | vs o200k (%) | vs Qwen (%) |
|---|---:|---:|---:|
| Turkish | 8,105,316 | +35.0% | +43.3% |
| Arabic | 5,640,670 | +33.8% | +43.8% |
| English | 7,184,246 | +6.9% | +11.5% |
| Hindi | 2,944,938 | +27.1% | +72.1% |
| Korean | 5,051,443 | +34.7% | +38.3% |
| Spanish | 3,354,176 | +12.4% | +24.7% |
| Japanese | 2,022,536 | +31.7% | +22.5% |
| Chinese | 834,803 | +22.3% | +16.7% |
| STEM / Math | 1,212,019 | +17.6% | +32.4% |
| Programming | 1,000,283 | +10.2% | +13.4% |
| Other languages | 46,889 | −56.3% | −0.9% |

## Table D — Historical published results (recorded at the time; not all on identical runs)

| Release | Reported B/T | Tokens | Benchmark | Comparable to standardized run? |
|---|---:|---:|---|
| v0.1 | 3.1676 | 65,573,253 | 544-file / 207.7 MB (same) | Yes (2026-11-15 runner) |
| v0.2 | 4.9290 | 42,140,653 | 544-file / 207.7 MB (same) | Yes; +21-token drift this cycle noted |
| v0.3 original | 5.5543 | 37,397,108 | 544-file / 207.7 MB (same) | Yes; reproduced exactly this cycle |
| v0.3-polished | 5.5542 | 37,397,319 | 544-file / 207.7 MB (same) | Yes |
| v0.4-fail (CJK8) | 5.5511 | 37,418,652 | 544-file / 207.7 MB (same) | Yes; reproduced this cycle |
| v0.4-fail (Han4) | 5.5529 | 37,406,269 | 544-file / 207.7 MB (same) | Yes; reproduced this cycle |
| o200k_base | 4.0751 | 50,971,306 | 544-file / 207.7 MB (same) | Yes; reproduced exactly this cycle |
| Qwen2.5 (0.5B) | 3.3358 | 62,266,982 | 544-file / 207.7 MB (same) | Yes; same tokenizer.json as pinned 7B revision |

All standardized numbers are in `STANDARDIZED_RESULTS.csv`; bucket-level counts are in `CATEGORY_RESULTS.csv`.

## Correctness signals (this cycle, 2026-11-16)

- NTokenizer models: encode/decode byte-exact round-trip **544/544 files** (harness requirement; all model rows below passed).
- External tokenizers: files processed successfully 544/544 each (tiktoken, HF tokenizers); decode exactness not measured (out of scope here).
- Public model verification (post-release, by re-download): SHA-256 `1f0d4ad7…` matches the official checkpoint.
