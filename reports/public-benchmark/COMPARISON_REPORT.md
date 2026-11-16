# NTokenizer — Public benchmark comparison report

Date: 2026-11-16
Benchmark: **development/validation benchmark — NOT a pristine blind evaluation set**
(544 files, 207,712,944 bytes, `data/v0.2/manifest/manifest_benchmark.json`,
SHA-256 `002a8224671cd281312e059af85dad24c269e7a3945e55724d476e3e255103c8`).

**Bold** = higher B/T (fewer tokens per input byte) than the comparator. The **Best B/T** column shows the highest value in the row (bolded). NTokenizer v0.3-polished is bolded only where it strictly beats the comparator — not by default. B/T measures compression efficiency, not language-model quality.

## Table A — Standardized comparison (all re-evaluated on identical files today, 2026-11-16)

| Tokenizer (exact reference) | Vocabulary | Total tokens | Overall B/T | Equal-category B/T | Token reduction vs o200k |
|---|---:|---:|---:|---:|---:|
| NTokenizer v0.1 | 262,144 | 65,573,261 | 3.1676 | 3.1203 | −28.65% |
| NTokenizer v0.2 | 262,144 | 42,140,674 | 4.9290 | 4.6855 | +17.32% |
| NTokenizer v0.3-original (pre-polish source) | 262,144 | 37,397,108 | 5.5543 | 5.1057 | +26.63% |
| **NTokenizer v0.3-polished (official release)** | 262,144 | **37,397,319** | **5.5542** | **5.1056** | **+26.63%** |
| NTokenizer v0.4-fail (CJK8) | 262,144 | 37,418,652 | 5.5511 | 5.1031 | +26.59% |
| NTokenizer v0.4-fail (Han4 arm) | 262,144 | 37,406,269 | 5.5529 | 5.1081 | +26.61% |
| o200k_base (tiktoken 0.14.0) | 200,019 | 50,971,306 | 4.0751 | 4.0899 | 0.00% |
| Bloom-7b1 (bigscience) | 250,680 | 54,773,073 | 3.7922 | 4.1584 | +7.4% |
| Qwen2.5-7B-Instruct tokenizer | 151,665 | 62,266,982 | 3.3358 | 3.3236 | −22.16% |
| cl100k_base (tiktoken 0.14.0) | 100,256 | 75,061,973 | 2.7672 | 2.8566 | −47.2% |
| Mistral-7B-Instruct-v0.2 | 32,000 | 87,276,011 | 2.3800 | 2.4943 | −71.3% |
| StableLM-base-alpha-3b | 50,277 | 83,207,383 | 2.4963 | 2.6430 | −63.2% |
| Llama-2-7b (NousResearch mirror) | 32,000 | 91,853,794 | 2.2613 | 2.3985 | −80.2% |
| CodeLlama-7b | 32,016 | 91,853,791 | 2.2613 | 2.3985 | −80.2% |
| DeepSeek-LLM-7b-base | 100,015 | 93,334,847 | 2.2255 | 2.4470 | −83.1% |
| Falcon-7b (tiiuae) | 65,024 | 103,640,810 | 2.0042 | 2.3126 | −103.3% |
| GPT-2 (r50k_base) | 50,256 | 105,229,579 | 1.9739 | 2.1406 | −106.4% |
| DeepSeek-Coder-6.7b-base | 32,022 | 116,855,033 | 1.7775 | 2.1074 | −129.3% |
| MosaicML MPT | n/a | NOT MEASURED (repositories 401/404 on the HF hub) | n/a | n/a | — |
| Gemini 3.8 Flash | n/a | NOT MEASURED (no credential) | n/a | n/a | — |

## Table A2 — Competitor landscape (properties + measured compression)

Properties are quoted from the public model cards; only the B/T column was measured here. Measurement policy identical to Table A (raw text, no chat templates, `add_special_tokens=False` / `allowed_special=set()`).

| Tokenizer | Algorithm | Multilingual & code | License | Vocab | Measured B/T |
|---|---|---|---|---|---:|
| o200k_base | byte-level BPE (regex pre-tokenizer) | Multilingual (incl. code/math emphasis) | MIT (tiktoken) | 200,019 | 4.0751 |
| cl100k_base | byte-level BPE | Multilingual (narrower than o200k) | MIT (tiktoken) | 100,256 | 2.7672 |
| GPT-2 | byte-level BPE | largely English | MIT (code) | 50,256 | 1.9739 |
| Llama-2 | SentencePiece BPE, byte fallback | Multilingual (strong on European languages) | Llama 2 Community License | 32,000 | 2.2613 |
| CodeLlama-7b | SentencePiece/byte-level BPE, byte fallback | Code-optimized (Llama 2 family) | Llama 2 Community License | 32,016 | 2.2613 |
| Qwen2.5-7B-Instruct | byte-level BPE (Qwen2 tokenizer family) | Multilingual (29+ languages), code-capable | Apache-2.0 | 151,665 | 3.3358 |
| Mistral-7B-Instruct-v0.2 | byte-level BPE with byte fallback | Multilingual; code | Apache-2.0 | 32,000 | 2.3800 |
| Bloom-7b1 | byte-level BPE (BBPE) | ~46 languages, multilingual focus | BigScience Open RAIL-M-1.0 | 250,680 | 3.7922 |
| StableLM-base-alpha-3b | byte-level BPE (GPT-NeoX family) | English + code | CC BY-SA 4.0 | 50,277 | 2.4963 |
| DeepSeek-LLM-7b-base | byte-level BPE (deepseek custom) | Multilingual (English/Chinese focus), code | other (see model card) | 100,015 | 2.2255 |
| DeepSeek-Coder-6.7b-base | byte-level BPE (deepseek custom) | code-focused | other (see model card) | 32,022 | 1.7775 |
| Falcon-7b | byte-level BPE (falcon family) | English + European languages; code | Apache-2.0 | 65,024 | 2.0042 |
| MosaicML MPT | — | — | — | — | NOT MEASURED (401/404 on the hub) |
| Gemini 3.8 Flash | — (vendor API tokenizer) | — | — | — | NOT MEASURED (no credential) |
| **NTokenizer v0.3-polished** | byte-level BPE + MathSymbolAware pre-tokenizer | Multilingual by design (technical + code + CJK included) | source MIT/Apache-2.0; model CC BY-SA 4.0 | 262,144 | **5.5542** |

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
