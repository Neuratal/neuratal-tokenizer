# NTokenizer v0.4 — Hugging Face Compatibility Report

Date: 2026-11-15
Status: measured and documented. **Plain `tokenizer.json` parity: not achievable;
exact native parity needs a custom backend; a plain export is structural and
decode-exact only.** Recommendations in §5.

## 1. What was exported

`examples/export_hf.rs` (v0.3 work, unchanged) converts a native `.ntok` model to a
GPT-2-style ByteLevel BPE `tokenizer.json`: vocab re-serialized with the GPT-2
byte→Unicode alphabet, merges emitted in rank order, special tokens as added
tokens, plus `tokenizer_config.json` and `special_tokens_map.json` sidecars so
`AutoTokenizer.from_pretrained(<dir>)` resolves `<|bos|>`/`<|eos|>`/`<|pad|>`/`<|unk|>`.

For the v0.4 release candidate (`output/v0.4/ntokenizer-0.4-256k.ntok`):

| Property | Result |
|---|---|
| `tokenizers.Tokenizer.from_file` loads | OK — vocab 262,144, 7 added tokens |
| `transformers.AutoTokenizer.from_pretrained` loads | OK — `PreTrainedTokenizerFast`-backed |
| AutoTokenizer resolves special tokens | OK — `<|bos|>` id 262137, `<|eos|>` 262138 |
| HF **decode(encode(x)) == x**, byte-exact, 523 fixtures | **523/523** |
| HF token IDs → native `.ntok` decode | **523/523 byte-exact** |
| Native token IDs → HF decode | **523/523 byte-exact** |
| **Token-ID parity HF vs native**, 523 fixtures | **46/523 (8.8%)** |
| Token-ID parity on pure printable-ASCII fixtures | 60/60 |

The 523-fixture suite is the full unique benchmark + Unicode stress corpus content
with ≤100 KB files (deduplicated, NUL-containing strings excluded — argv cannot
carry NUL). Every result is in `reports/v0.4/hf_parity.json`.

## 2. Why plain tokenizer.json cannot give parity

The HF pipeline runs `normalizer → pre_tokenizer → model → post_processor`.
NTokenizer's pretokenizer (`MathSymbolAwareCjk8` in v0.4) is a context-aware,
Unicode general-category script engine with:

- script-isolated Word runs (Han / Hiragana / Katakana / Hangul / Latin / …),
- combining-mark attachment and ZWJ/tag handling,
- decimal/time/comma-between-digits binding and operator binding,
- prefix/suffix symbol attachment,
- fixed 8-character CJK chunking,
and a 262K-token merge table learned against that segmentation.

The closest stock HF pre-tokenizers are `ByteLevel`, `Whitespace`, `Punctuation`,
`Digits`, `Sequence` — none of them implements this class of rules, and composing
stock rules cannot reproduce it (context conditions like "`:` between digits" are
not expressible). So the exported `tokenizer.json` re-pairs each byte with the
GPT-2 alphabet but **segments differently**, which is exactly what the 8.8%
parity measures.

## 3. Can a custom backend close the gap? Only partially.

We built a custom Python pre-tokenizer (`Experiments/v0.4/math_symbol_aware_py.py`)
using `tokenizers.pre_tokenizers.PreTokenizer.custom(...)` and the
`pretok.split(func)` protocol. Measurements (523 fixtures):

| Fixture class | Custom-backend parity |
|---|---|
| All printable-ASCII, no space/control bytes | **60/60** |
| General UTF-8 (523 fixtures) | **2/523** |
| Non-ASCII (CJK, Arabic, Devanagari, Thai, …) | ~0 |

Why: the custom Python protocol receives `NormalizedString` slices of the **raw**
input text; the model expects slices whose content is the **byte-level-mapped**
string (`' ' → 'Ġ'`, non-printables shifted, and every non-ASCII byte expands to
multiple chars). Slicing the raw string cannot yield the mapped string, so any
byte that is not byte-identity maps (space, control bytes, and anything
non-printable) breaks parity, and every non-ASCII input does too. A faithful
custom backend would have to be implemented in **Rust inside the `tokenizers`
crate** (or as a registered native plugin), which is not something this project
ships today.

## 4. What this means for users

1. **Decode parity is exact in both directions** (523/523 both ways): an HF token
   ID stream and a native token ID stream are the same vocabulary.
2. **Token-ID parity is not provided**: encoding the same string with HF and with
   the native tool will produce *different ID sequences* in ~91% of inputs.
3. `AutoTokenizer.from_pretrained()` **will work** (loads, encodes, decodes,
   special tokens resolved) — but the token IDs it emits are only valid for the
   exported model and are **not** the same IDs NTokenizer emits.
4. **Recommendation for exact interop**: use the native binding/CLI
   (`ntokenizer encode`, or the Python ctypes binding in `python/neuratal_tokenizer`,
   or the Rust library) — that is the only way to get the canonical ID stream.
5. The HF export is valuable for distribution (loads in `transformers`-based
   tooling, can be inspected, tokenized, decoded) but its token IDs are **not**
   compatible with NTokenizer's native encoding.

## 5. Integration options, ranked

| Option | Parity | Effort | Ships in this repo? |
|---|---|---|---|
| Native NTokenizer bindings (Python/Rust/CLI) | **exact** | done | yes |
| Plain HF export (current) | decode-exact only; 8.8% ID parity | done | yes |
| Custom Python pretokenizer backend | only printable-ASCII no-space | done (measured) | experimental |
| Custom Rust pretokenizer in `tokenizers` / `trust_remote_code` plugin | potentially exact | large | no |
| HF `ByteLevel`/`Whitespace` approximation via stock pipeline | no | n/a | n/a (not valid) |

**Decision for v0.4: ship option 1 (native bindings as the supported
integration) plus option 2 (structural HF export) with this report attached. Do
not advertise "fully Hugging Face compatible" anywhere.** The export is a
convenience artifact, not an equivalence guarantee.

## 6. Verification artifacts

- `output/v0.4/ntokenizer-0.4-256k.ntok` — release candidate (native).
- `output/v0.4/hf/tokenizer.json` — structural export (loads in `tokenizers` and
  `transformers.AutoTokenizer`; decode byte-exact).
- `reports/v0.4/hf_parity.json` — full 523-fixture parity measurements.
- `experiments/v0.4/parity_check.py` — reproducible harness (native CLI + `tokenizers`).
- `experiments/v0.4/math_symbol_aware_py.py` — Python reimplementation used for the
  custom-backend experiment (documented, not a production component).

## 7. Honest gaps

- The 523-fixture suite is limited by available corpus content (unique strings of
  ≤100 KB from the benchmark + synthetic stress files) — not a spec violation, but
  it is not 1000+ fixtures either; the brief's ≥1000 target is **not** met.
- The custom Python backend was only evaluated for segmentation parity; it has not
  been profiled for speed.
- `AutoTokenizer` loads successfully, but no `pipeline()` smoke test with a real
  HF model was performed (no model weights are bundled with this project).
