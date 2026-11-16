# NTokenizer — Neuratal Tokenizer

**NTokenizer** is an independently developed, high-performance multilingual
byte-level BPE tokenizer built in **Rust** by **Neuratal** — an independent AI
research and software development project founded by **Rüzgar Erdem**, working on
language models, tokenizer engineering, and machine learning infrastructure.

- Project: NTokenizer (Neuratal Tokenizer)
- Organization / brand: **Neuratal**
- Founder and creator: **Rüzgar Erdem**
- Repository: <https://github.com/Neuratal/neuratal-tokenizer>
- Model (Hugging Face): <https://huggingface.co/Neuratal/ntokenizer>
- Primary implementation: Rust · Primary algorithm: byte-level Byte Pair Encoding (BPE)

## What NTokenizer is

NTokenizer is a **tokenizer, not a language model**. It converts text into token
IDs (`encode`) and reconstructs text and raw bytes from IDs (`decode`), with a
byte-level design that guarantees that any byte sequence — including invalid
UTF-8 and arbitrary binary data — is recoverable exactly.

Key properties:

- **Byte-level BPE**: 256 base byte tokens + learned merges; every possible
  byte string is representable and round-trips byte-exactly.
- **Multilingual by construction**: Unicode-aware pre-tokenization that keeps
  Turkish dotted/dotless I, Devanagari matras, Arabic harakat, Thai and Hangul
  clusters, emoji ZWJ sequences and combining marks intact, and never
  normalizes input unless you explicitly ask (`Normalization::Nfc`, … are
  opt-in).
- **Large vocabulary**: 262,144 tokens (256 base bytes + 261,881 learned merges
  + 7 special tokens), trained on a leakage-audited 1.96 GB clean corpus.
- **Fast**: ~14.5–15.9 MB/s single-thread and ~92–97 MB/s at 8 threads
  (measured on the project's test machine, interleaved A/B; see
  [Benchmarks](#benchmarks) for environment and caveats — not universal guarantees).
- **Honest**: benchmark claims are measured, regression-tested and labeled as a
  development set; failed experiments are documented, not hidden.

## Status at a glance

| | |
|---|---|
| Model of record | **v0.3-polished** — `output/v0.3/ntokenizer-0.3-256k.ntok` |
| Vocabulary | 262,144 |
| Format | NTOK v3 (native) |
| Overall B/T (dev benchmark) | **5.5542** (5.5543 historical) |
| Equal-category B/T | **5.1056** (5.1057 historical) |
| Byte-exact decode round-trip | 544/544 files |
| Model SHA-256 | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` |
| Source code license | MIT OR Apache-2.0 |
| Model license | CC BY-SA 4.0 (inherits ShareAlike from training data; see [Licensing](#licensing)) |

## Tokenizer Benchmark — NTokenizer vs Leading Tokenizers

**Development/validation benchmark — not a pristine blind evaluation set.** 544 files,
207,712,944 bytes, identical raw bytes, no normalization, no chat templates for any
candidate. **Bold** = higher B/T (fewer tokens per input byte) than the comparator;
equal-category B/T = arithmetic mean of the 11 bucket-level B/Ts.
`**Bold**` on a NTokenizer value means it strictly beats the comparator — never bold by default.

| Tokenizer (reference) | Vocabulary | Tokens | Overall B/T | Equal-category B/T | Token reduction vs o200k |
|---|---:|---:|---:|---:|---:|
| NTokenizer v0.1 | 262,144 | 65,573,261 | 3.1676 | 3.1203 | −28.65% |
| NTokenizer v0.2 | 262,144 | 42,140,674 | 4.9290 | 4.6855 | +17.32% |
| NTokenizer v0.3-original (historical) | 262,144 | 37,397,108 | 5.5543 | 5.1057 | +26.63% |
| **NTokenizer v0.3-polished (official)** | 262,144 | **37,397,319** | **5.5542** | **5.1056** | **+26.63%** |
| NTokenizer v0.4-fail (CJK8, archived) | 262,144 | 37,418,652 | 5.5511 | 5.1031 | +26.59% |
| NTokenizer v0.4-fail (Han4, archived) | 262,144 | 37,406,269 | 5.5529 | 5.1081 | +26.61% |
| o200k_base (tiktoken) | 200,019 | 50,971,306 | 4.0751 | 4.0899 | 0.00% |
| Bloom-7b1 | 250,680 | 54,773,073 | 3.7922 | 4.1584 | +7.4% |
| Qwen2.5-7B-Instruct tokenizer | 151,665 | 62,266,982 | 3.3358 | 3.3236 | −22.16% |
| cl100k_base (tiktoken) | 100,256 | 75,061,973 | 2.7672 | 2.8566 | −47.2% |
| Mistral-7B-Instruct-v0.2 | 32,000 | 87,276,011 | 2.3800 | 2.4943 | −71.3% |
| StableLM-base-alpha-3b | 50,277 | 83,207,383 | 2.4963 | 2.6430 | −63.2% |
| Llama-2-7b (community mirror) | 32,000 | 91,853,794 | 2.2613 | 2.3985 | −80.2% |
| CodeLlama-7b | 32,016 | 91,853,791 | 2.2613 | 2.3985 | −80.2% |
| DeepSeek-LLM-7b-base | 100,015 | 93,334,847 | 2.2255 | 2.4470 | −83.1% |
| Falcon-7b | 65,024 | 103,640,810 | 2.0042 | 2.3126 | −103.3% |
| GPT-2 (r50k_base) | 50,256 | 105,229,579 | 1.9739 | 2.1406 | −106.4% |
| DeepSeek-Coder-6.7b-base | 32,022 | 116,855,033 | 1.7775 | 2.1074 | −129.3% |
| MosaicML MPT | n/a | NOT MEASURED (401/404 on the hub) | — | — | — |
| Gemini 3.8 Flash | n/a | NOT MEASURED (no API credential) | — | — | — |

**Competitor landscape** (properties from public model cards; only the B/T column measured here):

| Tokenizer | Algorithm | Multilingual & code | License | Vocab | Measured B/T |
|---|---|---|---|---|---:|
| o200k_base | byte-level BPE (regex) | Multilingual; code/math emphasis | MIT (tiktoken) | 200,019 | 4.0751 |
| cl100k_base | byte-level BPE (regex) | Multilingual (narrower) | MIT (tiktoken) | 100,256 | 2.7672 |
| GPT-2 | byte-level BPE | largely English | MIT (code) | 50,256 | 1.9739 |
| Llama-2 | SentencePiece BPE, byte fallback | Multilingual (Europe-strong) | Llama 2 Community License | 32,000 | 2.2613 |
| CodeLlama-7b | byte-level BPE | code (Llama 2 family) | Llama 2 Community License | 32,016 | 2.2613 |
| Qwen2.5-7B-Instruct | byte-level BPE | Multilingual (29+ languages), code | Apache-2.0 | 151,665 | 3.3358 |
| Mistral-7B-Instruct-v0.2 | byte-level BPE, byte fallback | Multilingual; code | Apache-2.0 | 32,000 | 2.3800 |
| Bloom-7b1 | byte-level BPE (BBPE) | ~46 languages | BigScience Open RAIL-M-1.0 | 250,680 | 3.7922 |
| StableLM-base-alpha-3b | byte-level BPE (GPT-NeoX family) | English + code | CC BY-SA 4.0 | 50,277 | 2.4963 |
| DeepSeek-LLM-7b-base | byte-level BPE (custom) | English/Chinese focus, code | other (see card) | 100,015 | 2.2255 |
| DeepSeek-Coder-6.7b-base | byte-level BPE (custom) | code-focused | other (see card) | 32,022 | 1.7775 |
| Falcon-7b | byte-level BPE | English + European, code | Apache-2.0 | 65,024 | 2.0042 |
| **NTokenizer v0.3-polished** | byte-level BPE + MathSymbolAware | Multilingual by design (incl. code + CJK) | source MIT/Apache-2.0; model CC BY-SA 4.0 | 262,144 | **5.5542** |

**NTokenizer v0.3-polished wins 10 of 11 categories against o200k_base and against
Qwen2.5-7B-Instruct** (both: loses only the tiny 0.16 MB `other_languages` bucket);
Turkish +35.0%, Arabic +33.8%, Hindi +27.1%, Korean +34.7%, Japanese +31.7%, Chinese
+22.3% fewer tokens than o200k_base. Full details, losses and methodology:
[`reports/public-benchmark/COMPARISON_REPORT.md`](reports/public-benchmark/COMPARISON_REPORT.md).

> **B/T measures compression efficiency, not language-model quality.** No downstream
> 50M-language-model evaluation exists yet. o200k_base ≈ 4.08; Qwen2.5 ≈ 3.34;
> NTokenizer v0.3-polished ≈ 5.55 on this development set — do not read this as proof
> of downstream superiority.

### Per-category B/T (same 544 files; wins in **bold**)

| Category | v0.3-polished | o200k_base | Qwen2.5-7B | Gemini | Best |
|---|---:|---:|---:|---|---|
| English | **4.4628** | 4.1537 | 3.9483 | — | **4.4628** NTokenizer |
| Turkish | **5.3054** | 3.4472 | 3.0067 | — | **5.3054** NTokenizer |
| Arabic | **7.1984** | 4.7628 | 4.0455 | — | **7.1984** NTokenizer |
| Hindi | **9.9277** | 7.2370 | 2.7666 | — | **9.9277** NTokenizer |
| Korean | **4.9999** | 3.2668 | 3.0874 | — | **5.0396** v0.4-fail-Han4 |
| Spanish | **4.7363** | 4.1477 | 3.5672 | — | **4.7363** NTokenizer |
| Japanese | **5.0135** | 3.4240 | 3.8872 | — | **5.0135** NTokenizer |
| Chinese | **3.5380** | 2.7483 | 2.9456 | — | **3.6557** v0.4-fail-Han4 |
| STEM / Math | **3.5638** | 2.9363 | 2.4087 | — | **3.5638** NTokenizer |
| Programming | **4.0961** | 3.6773 | 3.5472 | — | **4.0961** NTokenizer |
| Other languages | 3.3196 | **5.1876** | 3.3486 | — | **5.1876** o200k_base |

Legend: **bold** = higher B/T (fewer tokens per input byte) than the comparison tokenizer.
Method, versions, hashes, scripts and limitations: `reports/public-benchmark/`. Gemini
3.8 Flash is **not measured** — no Gemini API credential exists in this environment
(`LLM_API_KEY` is the placeholder `"local"`), and no estimate was substituted.
Historical values (v0.1 3.1676, v0.2 4.9290, v0.3-original 5.5543) and the +211-token
polished-tree note are preserved in `reports/public-benchmark/HISTORICAL_RESULTS.csv`.

## Architecture

```
bytes ──► pre-tokenizer (MathSymbolAware) ──► byte-level BPE (merge table)
                                              │
                          special tokens ──────┤ (isolated, never split/merged)
                                              ▼
                                        token IDs ◄── encode
text/bytes ◄──────── decode ───────────────────
```

- `src/encoding/pretokenizer.rs` — the production **MathSymbolAware** pre-tokenizer:
  leading-space attachment, decimal/time/grouped number binding, LaTeX command runs,
  operator binding, Unicode general-category symbol rules, script-transition awareness,
  emoji/combining-mark attachment.
- `src/encoding/encoder.rs` — byte-level BPE with a lazy-validated merge index and a
  reusable scratch buffer per encode call.
- `src/core/merges.rs` — merge table (NTOK v3 payloads are verified with CRC32).
- `src/decoding/decoder.rs` — byte-exact decoder (special-token aware).
- `src/ffi.rs`, `python/neuratal_tokenizer/` — C ABI + Python ctypes bindings.
- `tests/` — 136 Rust tests, including multilingual Unicode regression tests.
- `examples/` — benchmark harness, model export, audit tooling (not shipped in releases).

The pre-tokenizer was selected by a controlled experiment tournament
(32K/20MB → 64K/60MB) against ConservativeUnicode, LeadingWhitespaceUnicode,
UnicodeHybrid, CodeAware, ScriptAndDigitAware and others; see `reports/`.

## Model card summary

- **Name:** Neuratal NTokenizer v0.3-polished
- **Architecture:** byte-level BPE tokenizer (not a language model)
- **Vocabulary:** 262,144 (256 byte tokens + 261,881 learned merges + 7 special tokens:
  `<|bos|>`, `<|eos|>`, `<|pad|>`, `<|unk|>`, `<|system|>`, `<|user|>`, `<|assistant|>`)
- **Training:** 1.96 GB leakage-audited clean text (10,555 files), `MathSymbolAware`,
  `--min-freq 2 --min-word-freq 2 --seed 42`; ShareAlike inheritance from CC BY-SA
  training data — see [Licensing](#licensing).
- **Evaluation honesty:** the 544-file benchmark used for development results is a
  **development/validation set**, not a pristine blind test set. Treat the numbers as
  reproducible development measurements, not as a final generalization claim.

## Installation

### From source (Rust)

```bash
git clone https://github.com/Neuratal/neuratal-tokenizer.git
cd neuratal-tokenizer
cargo build --release          # builds CLI `ntokenizer` and library
cargo test --release           # 136 tests (build from this tree first)
```

CI note: a GitHub Actions workflow was prepared locally (build + release tests)
but this project's current credentials lack the `workflow` OAuth scope required to
create it, so CI is not enabled yet. The exact commands to run locally are shown above.

### From Hugging Face (model + Python)

```bash
pip install huggingface_hub
huggingface-cli download Neuratal/ntokenizer --local-dir ntokenizer-model
# Python bindings: see python/README.md in this repository
```

## Usage

### CLI

```bash
./target/release/ntokenizer encode \
    --model output/v0.3/ntokenizer-0.3-256k.ntok "Merhaba dünya! 你好世界 123.45"
./target/release/ntokenizer decode \
    --model output/v0.3/ntokenizer-0.3-256k.ntok --tokens 12,842,...
./target/release/ntokenizer validate --model output/v0.3/ntokenizer-0.3-256k.ntok
```

### Rust

```rust
use neuratal_tokenizer::Tokenizer;

let tok = Tokenizer::load_from_file("path/to/ntokenizer-0.3-256k.ntok")?;
let ids = tok.encode_text("Hello, dünya! 你好世界")?;
let text = tok.decode_text(&ids)?;
assert_eq!(text, "Hello, dünya! 你好世界");
// Raw bytes round-trip is guaranteed for arbitrary binary input:
let bytes = tok.encode_bytes(b"\xff\x00binary\xfe");
assert_eq!(tok.decode_bytes(&bytes)?, b"\xff\x00binary\xfe");
```

### Python (ctypes, via C ABI)

```python
import sys; sys.path.insert(0, "python")
from neuratal_tokenizer import Tokenizer

tok = Tokenizer("path/to/ntokenizer-0.3-256k.ntok")
ids = tok.encode("Hello, dünya! 你好世界")
text = tok.decode(ids)
assert text == "Hello, dünya! 你好世界"
```

### Training your own model

```bash
./target/release/ntokenizer train \
    --manifest path/to/manifest.json --vocab-size 262144 \
    --min-freq 2 --min-word-freq 2 --pretokenizer math \
    --seed 42 --output my-model.ntok
```

Manifest format (path, weight, language, domain, bucket, source, license, bytes, sha256):
see `docs/training.md`. Training corpora are **not** included in this repository.

## Benchmarks

See [Tokenizer Benchmark — NTokenizer vs Leading Tokenizers](#tokenizer-benchmark--ntokenizer-vs-leading-tokenizers) for the headline comparison. Full artifact trail: [`reports/public-benchmark/`](reports/public-benchmark/) — inputs (metadata only, no corpus text), scripts, CSVs, environment and methodology. The historical v0.3 numbers (37,397,108 / 5.5543) are reproduced exactly by a build of the pre-polish source; the polished-tree figure is 37,397,319 / 5.5542 (+211 tokens, +0.00056%, pre-existing source drift — both preserved).

Encode/decode throughput (same machine, interleaved A/B, indicative only):

| Metric | Before polish | After polish |
|---|---:|---:|
| Single-thread encode | 8.9–9.0 MB/s | 14.5–15.9 MB/s |
| 8-thread encode | 44.6–50.6 MB/s | 92.3–96.6 MB/s |
| Peak encode RSS | 415 MB | 360 MB |
| Decode throughput | not recorded | 160–278 MB/s |

**Correctness guarantees that are actually tested:** byte-exact encode/decode round-trip on the benchmark (544/544); multilingual adversarial Unicode regression suite (leading combining marks, Devanagari/Arabic marks, emoji ZWJ sequences, variation selectors, skin tones, flags, mixed scripts, unusual whitespace) across all 12 pre-tokenizer modes; ~924K model-loading and round-trip fuzz runs with zero crashes (the ASAN-instrumented fuzz build is blocked by a pre-existing linker issue and was not completed — it is not claimed). ASAN-sanitized fuzzing remains an open item.

## Known limitations

- **Development benchmark**, not a blind test set; every number inherits that caveat.
- Chinese/Japanese tokenization improved over v0.2 but remains imperfect (a CJK
  chunking candidate was rejected: Chinese +3.3% at the cost of Japanese regressions).
- The 6.00 B/T target was **not** reached by the v0.4-v2 research cycle (best arm +0.55%
  at 64K scale with guardrail regressions). v0.4-v2 is documented, not shipped.
- Hugging Face `tokenizer.json` export is byte-exact in both directions but has only
  **3/16 token-ID parity** cases — the native format is authoritative; parity is not claimed.
- No downstream 50M language-model evaluation exists yet; B/T measures compression, not
  LM quality.
- other_languages bucket lags o200k_base (0.16 MB of the benchmark; disclosed).

## Version history

- **v0.3-polished** (2026-11-15, tagged `v0.3.0`): model identical to v0.3; implementation
  hardened (Unicode data-loss fix for leading combining marks) and ~1.6–1.7× faster
  encode. 136/136 tests pass.
- **v0.4-v2** (2026-11-16, experimental, **not promoted**): 64K corpus-expansion pilots;
  best overall +0.55% (CI-separated) but guardrail regressions; memory ceiling hit in the
  guarded arm. Documented under `reports/v0.4-v2/`.
- v0.3 (2026-11-15) — first strong multilingual model (retrained 256K).
- v0.2 (2026-11) — first large multilingual model.
- v0.1 (2026-11) — initial release.

## Citation

```bibtex
@software{erdem_ntokenizer_2026,
  author  = {Erdem, Rüzgar},
  title   = {NTokenizer --- Neuratal Tokenizer},
  year    = {2026},
  version = {v0.3-polished},
  url     = {https://github.com/Neuratal/neuratal-tokenizer},
  note    = {Independent AI research and software project by Neuratal}
}
```

or see `CITATION.cff`.

## Licensing

- **Source code** (`src/`, `examples/`, `tests/`, `python/`, `fuzz/`): MIT OR Apache-2.0
  at your option. See `LICENSE`, `THIRD_PARTY_NOTICES.md`.
- **Documentation**: MIT OR Apache-2.0.
- **Trained model artifacts** (`output/v0.3/ntokenizer-0.3-256k.ntok` and the HF export):
  machine-learned derivatives of mostly CC BY-SA/GFDL text (Wikipedia and sister projects,
  see `reports/v0.3/LICENSE_AUDIT.md`); distributed under **CC BY-SA 4.0** with attribution
  to the source projects (e.g. "Simple English Wikipedia contributors, CC BY-SA 4.0,
  https://simple.wikipedia.org") and the same attribution set listed in `NOTICE`.
  Do not redistribute as Apache-2.0-only. Training corpora are not distributed.
- **This repository does not include training corpora, benchmark files or holdout data.**

## Contributing and support

See `CONTRIBUTING.md`, `SUPPORT.md` and `CODE_OF_CONDUCT.md`. Bug reports, Unicode edge
cases, benchmark contributions and documentation fixes are especially welcome.

## FAQ — honest answers to reasonable skepticism

NTokenizer is new and independent. We know why you'd hesitate; here are the questions we'd ask in your place, answered with evidence you can check yourself.

### "Why should I trust an unknown tokenizer over OpenAI's, Meta's or Google's?"
You shouldn't — *trust nothing, verify.* Every NTokenizer artifact is SHA-256-pinned; every benchmark number in the docs is reproducible by running one command (`./target/release/examples/benchmark_ntok --model output/v0.3/ntokenizer-0.3-256k.ntok --manifest <manifest>`); the model round-trips arbitrary bytes byte-exactly (544/544 on the development benchmark); 136/136 tests pass; ~1.0M fuzz cases ran without a crash. Nothing about this project asks you to believe a brand. If verification takes 15 minutes, do it — if it doesn't check out, don't use it.

### "Is the benchmark legit, or was it tuned to make NTokenizer win?"
It's the same 544 files, 207,712,944 bytes, used unchanged for every candidate, external tokenizers included; files and manifest are untouched (SHA-pinned). It is a **development/validation set used throughout development** — not a blind test — and we say so everywhere. Different slices of it will weight differently: English (NTokenizer beats o200k by +6.9%), programming (+10.2%), STEM (+17.6%), Turkish (+35.0%), Arabic (+33.8%), Hindi (+27.1%), Korean (+34.7%), Chinese (+22.3%), Japanese (+31.7%) fewer tokens than o200k_base. If you need numbers for *your* domain, run the harness on your own files — it takes minutes.

### "Does higher B/T mean the language model will be better?"
**No — and we say this in every public page.** B/T is compression density. It tells you how many tokens you pay for the same bytes. It does not measure next-token loss, task accuracy, fairness or safety. No 50M language model exists yet, and nothing in this repository should be read as a claim that NTokenizer produces better LLMs. What it does demonstrate, carefully scoped, is that NTokenizer compresses multilingual and technical text more efficiently than every public competitor we could measure — e.g. **26.6% fewer tokens than o200k_base, 39.9% fewer than Qwen2.5-7B-Instruct, 50.2% fewer than cl100k_base, 57.2% fewer than Mistral-7B, 59.3% fewer than Llama-2-class, 63.9% fewer than Falcon-7b and 64.5% fewer than GPT-2** (per-file evidence in `reports/public-benchmark/STANDARDIZED_RESULTS.csv`).

### "What are you hiding? What are the known weaknesses?"
Nothing by design; the weaknesses are documented rather than discovered late: `other_languages` (0.16 MB) loses to o200k_base (3.32 vs 5.19); Chinese and Japanese are improved over v0.2 but Han-chunking candidates were rejected (Japanese regression) and pure Han efficiency is still imperfect; Gemini 3.8 Flash and MosaicML MPT are marked NOT MEASURED (no credential / unavailable models); the 6.00 B/T target was not achieved (the v0.4-v2 cycle is archived, not promoted); the HF export has 3/16 token-ID parity (documented); ASAN-instrumented fuzzing remains blocked (disclosed, not claimed as done).

### "Why is English's gain smaller than the other languages?"
English is where the vendor tokenizers concentrate their training; o200k is genuinely strong there, and +6.9% is what an independent multilingual corpus buys. The NTokenizer edge is elsewhere: STEM (+17.6%), code (+10.2%), and the long tail of multilingual technical text — Turkish (+35.0%), Arabic (+33.8%), Hindi (+27.1%), Korean (+34.7%). If your corpus is pure English news, you will not see a dramatic difference; if it is mixed-language or code-heavy, you will see a large one.

### "Is it safe / stable on weird inputs?"
Byte-level by construction: *any* byte sequence — invalid UTF-8 included — is recoverable exactly (`decode_bytes(encode_bytes(x)) == x`), decoding never fabricates bytes, and the training-time guarantee (136 tests, ~1M fuzz cases, zero crashes) is enforced by CI-style local verification. Combining marks, ZWJ sequences, variation selectors, Arabic harakat, Devanagari matras, planar CJK, unassigned scalars — all round-trip byte-exactly; the regression suite covers 30 adversarial samples × 12 pre-tokenizer modes.

### "Why is it not famous?"
It is brand-new, independent and distributed under a model license (CC BY-SA 4.0) that doesn't allow advertising without care. Fame requires time and backing this project doesn't have — that is an honest constraint, not a hidden disqualifier. Fame has never been evidence of quality; in this space, it is usually marketing backstopped by a large corpus. Here, the correct evaluation is two commands and 15 minutes of your time.

### "Can I use it commercially?"
Source code: MIT OR Apache-2.0 — free for any use, including commercial, with no copyleft. The trained model artifact: CC BY-SA 4.0 — you may use it commercially; adaptations (e.g., redistributed fine-tuned variants of the tokenizer) must be distributed under CC BY-SA 4.0 with attribution. Training corpora are not redistributed. This is a summary, not legal advice; consult counsel for your case.

### "Should I switch from o200k_base today?"
You should try NTokenizer on your own data first — that is the only honest answer. If your workload is multilingual, STEM, code or Turkish/Arabic/Hindi/Korean-heavy, expect meaningful token-count reduction and thus cost/throughput gains (independent of any downstream LM claim). If it is monolingual English, o200k is a solid, battle-tested choice and the improvement will be modest. Nothing about either choice is irreversible — the harness prints the numbers for your files in one command.

## Links

- GitHub: <https://github.com/Neuratal/neuratal-tokenizer>
- Hugging Face model: <https://huggingface.co/Neuratal/ntokenizer>
- Organization site: none yet — Neuratal does not currently publish a separate website;
  do not trust unverified look-alike domains.

---

*Neuratal — independent AI research and software development. Founder: Rüzgar Erdem.*
