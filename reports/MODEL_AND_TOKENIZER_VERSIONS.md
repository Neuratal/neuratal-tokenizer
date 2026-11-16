# Model and tokenizer versions — NTokenizer public benchmark

Date: 2026-11-16

## NTokenizer models (all re-evaluated on the same benchmark unless noted)

| Display name | Artifact | SHA-256 | How evaluated |
|---|---|---|---|
| NTokenizer v0.1 | `output/ntokenizer-256k.ntok` | `8abe252f159ed58a1a0072eaac0f9861dab74b4e6bed5da9b69a218493662f00` | Native harness |
| NTokenizer v0.2 | `output/v0.2/ntokenizer-0.2-256k.ntok` | `1fc3ee8568b7a08a124a5e27ee7a8078d5bdfae495a253b3f97ba0ad29ab54ac` | Native harness |
| NTokenizer v0.3-polished (**official release**) | `output/v0.3/ntokenizer-0.3-256k.ntok` | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` | Native harness (current polished source) |
| NTokenizer v0.3-original (historical record) | same artifact | same SHA-256 | Native harness on a build of the **pre-polish source snapshot** (`experiments/v0.3-v2/pre_session_source_snapshot.tar.gz`, sha256 `6a45fc5b936a4037c934beaf384521b9eb2530cba0782f4e3e3c03d5a9a3ef50`) — reproduced 37,397,108 tokens exactly |
| NTokenizer v0.4-fail (CJK8) | `output/v0.4-fail/ntokenizer-0.4-256k.ntok` | `fbb5d2d386071f94993130ee2a196e02990c240307d56996487ee6878590874e` | Native harness |
| NTokenizer v0.4-fail (Han4 arm) | `experiments/v0.4-fail/models/final-RELEASE_math_cjkhan4_256k.ntok` | `213b20d4d3af8327846c5b43d6e47e1a73dcfd85ba3df8bb4c7c2fdcaf7276a9` | Native harness |

Note: v0.4-fail-CJK8 and v0.4-fail-Han4 are different artifacts and are reported as two separate rows. v0.4-v2 (64K pilots) produced no promotable 256K model and is excluded.

Common NTokenizer configuration (all 256K vocabulary models): byte-level BPE, 256 base byte tokens + 261,881 learned merges + 7 special tokens, NTOK v3 format, `MathSymbolAware` pretokenizer, `Normalization::None`.
v0.4-fail models additionally use the `MathSymbolAwareCjk8` / `MathSymbolAwareCjkHan4` pretokenizer mode (the only training difference; same manifest/seed/recipe otherwise: `--min-freq 2 --min-word-freq 2 --pretokenizer cjk* --seed 42`).

## External tokenizers

| Name | Library / source | Revision / version | Tokenizer file hash | Vocab | Notes |
|---|---|---|---|---|---|
| o200k_base | tiktoken 0.14.0 (`tiktoken.get_encoding('o200k_base')`) | tiktoken built-in; rank file downloaded by tiktoken at first use | not pinned as a git revision (library manages it) | 200,019 | `allowed_special=set()`; decode not measured |
| Qwen2.5-7B-Instruct tokenizer | HF `tokenizers` 0.23.3-family | `a09a35458c702b33eeacc393d103063234e8bc28` (lastModified 2025-01-12) | `tokenizer.json` sha256 `c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539` — **identical to the Qwen2.5 tokenizer used in the 2026-11-14 historical measurements** | 151,665 | `add_special_tokens=False`; decode not measured |
| Gemini 3.8 Flash | — | — | — | — | **NOT MEASURED** (no API credential in this environment; see METHODOLOGY.md §4) |

Cross-reference checks recorded in this cycle:

- tiktoken `o200k_base` on the benchmark: **50,971,306 tokens — identical to the 2026-11 historical figure**.
- Qwen tokenizer.json sha256 `c0382117…` — identical to the file hash recorded in `reports/v0.2/BENCHMARK_REPORT.md`; measured 62,266,982 tokens → 3.3358 B/T — identical to the historical 3.3358.
- Historical NTokenizer v0.3 (original, 37,397,108 / 5.5543) reproduced exactly by the pre-polish source build.
- v0.2 (42,140,674 tokens this run vs 42,140,653 recorded): +21 tokens, the same pre-existing source drift documented in `reports/v0.3-v2/` and `reports/public-release/PUBLICATION_REPORT.md`.

## Special tokens (NTokenizer models)

`<|bos|> 262137`, `<|eos|> 262138`, `<|pad|> 262139`, `<|unk|> 262140`, `<|system|> 262141`, `<|user|> 262142`, `<|assistant|> 262143`
