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

Development/validation benchmark (544 files, 207,712,944 bytes; **not** a pristine
blind set):

| Model | Tokens | Overall B/T | Equal-category B/T | Decode |
|---|---:|---:|---:|---|
| NTokenizer v0.3-polished (current) | 37,397,319 | **5.5542** | **5.1056** | 544/544 exact |
| NTokenizer v0.3 (original report) | 37,397,108 | 5.5543 | 5.1057 | 544/544 exact |
| NTokenizer v0.2 | 42,140,653 | 4.9290 | 4.6857 | 544/544 exact |
| o200k_base (external reference) | 50,971,306 | 4.0751 | 4.0899 | n/a |

The historical 37,397,108-token figure and the polished-tree 37,397,319-token figure
differ by +211 tokens on 24 files (+0.00056%); the cause is a pre-existing source-tree
drift that predates the polishing release and was verified not to come from the
Unicode-loss fix. Both are recorded; nothing is hidden.

Encode/decode throughput (same machine, interleaved A/B, indicative only):

| Metric | Before polish | After polish |
|---|---:|---:|
| Single-thread encode | 8.9–9.0 MB/s | 14.5–15.9 MB/s |
| 8-thread encode | 44.6–50.6 MB/s | 92.3–96.6 MB/s |
| Peak encode RSS | 415 MB | 360 MB |
| Decode throughput | not recorded | 160–278 MB/s |

Per-bucket B/T (polished model): Hindi 9.93 · Arabic 7.20 · Turkish 5.31 · Korean 5.00 ·
Japanese 5.01 · Spanish 4.74 · English 4.46 · Programming 4.10 · STEM 3.56 ·
Chinese 3.54 · Other languages 3.32.

**Correctness guarantees that are actually tested:** byte-exact encode/decode round-trip
on the benchmark (544/544); multilingual adversarial Unicode regression suite
(leading combining marks, Devanagari/Arabic marks, emoji ZWJ sequences, variation
selectors, skin tones, flags, mixed scripts, unusual whitespace) across all 12
pre-tokenizer modes; ~924K model-loading and round-trip fuzz runs with zero crashes
(the ASAN-instrumented fuzz build is blocked by a pre-existing linker issue and was not
completed — it is not claimed). ASAN-sanitized fuzzing remains an open item.

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

## Links

- GitHub: <https://github.com/Neuratal/neuratal-tokenizer>
- Hugging Face model: <https://huggingface.co/Neuratal/ntokenizer>
- Organization site: none yet — Neuratal does not currently publish a separate website;
  do not trust unverified look-alike domains.

---

*Neuratal — independent AI research and software development. Founder: Rüzgar Erdem.*
