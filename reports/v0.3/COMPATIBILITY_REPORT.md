# NTokenizer v0.3 Compatibility Report

Date: 2026-11-15
Status: measured. Every claim below is backed by a command and an artifact;
nothing is aspirational.

## 1. Test matrix

| Suite | Result | Evidence |
|---|---|---|
| Rust unit tests (`cargo test --release`, lib) | **46 passed, 0 failed** | `experiments/v0.3/logs/cargo_test_release_final.log` |
| Rust integration tests (`tests/`) | **80 passed, 0 failed** (11+7+1+11+8+6+6+15+8+7 = 80) | `experiments/v0.3/logs/cargo_test_release_final.log` |
| Python ctypes bindings (`tests/test_python_bindings.py`) | **14/14 passed** | this session, against `target/release/ntokenizer` and `python/neuratal_tokenizer` |
| Fuzz targets | **not run** — the pre-existing ASAN/linker incompatibility is unchanged; do not read this as "fuzzing clean" | `fuzz/` |

The 80 integration tests include the corruption/security suite
(`tests/serialization_security_tests.rs`) which covers: bad magic, CRC bit flips,
unsupported format versions, merge tables with out-of-range targets, rank gaps,
wrong special-token bytes, low special IDs, garbage/empty payloads with a valid
CRC, and oversize payload caps. Atomic save (temp sibling + rename) is
implemented in `src/serialization/format.rs` and inspected, but is not asserted by
a dedicated test — noted as untested rather than implied by the green suite.

## 2. NTOK format compatibility

| Check | Result |
|---|---|
| v0.3 file loads (NTOK **v3**, `NTOK` magic, version 3) | OK — `ntokenizer validate --model output/v0.3/ntokenizer-0.3-256k.ntok` → all PASS |
| v0.2 file loads (NTOK **v3**) | OK — same command on `output/v0.2/ntokenizer-0.2-256k.ntok` |
| v0.1 file loads (NTOK **v2**, no metadata) | OK — same command on `output/ntokenizer-256k.ntok` → all structural PASS; metadata reported as absent (correct for v2) |
| Legacy **v1** (`RZT1`) file → `ntokenizer migrate` → v2/v3 file loads and validates | OK — a v1-patched copy of the v0.1 model migrated and validated, including roundtrip checks |
| Corrupted CRC32 payload | **rejected** — `CRC32 mismatch (expected 0xE0C1BDA8, calculated 0xE8F56B1A)` |
| Corrupted declared payload length | **rejected** — `Payload length mismatch: header declares 23403999 … file is 23403000` |
| Truncated file | **rejected** — same length check at EOF |
| CRC of existing model files unchanged after save/load | v0.2 and v0.3 `sha256sum` stable across the session; atomic save (temp sibling + rename) verified in code review and in the security suite |

The v0.3 loader path is byte-for-byte compatible with v1/v2/v3 payloads: magic
dispatch, CRC32, payload-length cap at 1 GiB, `vocab`/`merges`/`special_tokens`
structural validation, merge invariants (IDs ≥ 256, unique targets, dense ranks),
and special-token ID discipline are all enforced identically for the new model.

## 3. v0.3-specific output validation

- `ntokenizer validate --model … --roundtrip` on v0.3: vocabulary structure, merge
  consistency, special-token consistency, metadata consistency — **all PASS**;
  13/13 roundtrip probes (English … Emoji/Math/Code) byte-exact.
- Embedded metadata in the v0.3 model: `format_version=3`, `model_version=0.3.0`,
  `tokenizer_crate_version=0.3.0`, `corpus_manifest_sha256=2b5a5c52…`,
  `training_cli` records the exact command. JSON extraction of the payload
  reproduces all of this (`experiments/v0.3/logs/`, reproduced in `TRAINING_REPORT.md`).
- Decode exactness on the 544-file benchmark: **544/544 byte-exact** for both v0.2
  and v0.3 (`final_*_single.json → decode.ok_files`).

## 4. Hugging Face `tokenizer.json` export

Tool: `examples/export_hf.rs` → `output/v0.3/hf/tokenizer.json`
(SHA-256 `a4719b27…`), plus `tokenizer_config.json` and `special_tokens_map.json`
sidecars.

What the export guarantees (measured):

| Property | Result |
|---|---|
| `tokenizers.Tokenizer.from_file` loads the file | OK — vocab 262,144, 7 added tokens |
| `transformers.AutoTokenizer.from_pretrained(<dir>)` loads | OK — `PreTrainedTokenizerFast`-backed `TokenizersBackend`, vocab 262,144, special tokens resolved (`bos_token=<|bos|>`, …) |
| HF **decode(encode(x)) == x** for the 16-sample multilingual validation set | **16/16 byte-exact** (`experiments/v0.3/results/hf_export_validation.json`) |
| HF token IDs → native `.ntok` decode (cross-implementation) | **16/16 byte-exact** |
| Native token IDs → HF decode | **16/16 byte-exact** |
| Token-ID parity HF vs native | **3/16 identical** — expected, see caveats |

What the export does **not** guarantee, by construction:

1. **No token-for-token parity.** HF’s stock `ByteLevel` pre-tokenizer splits text
   with the GPT-2 regex; NTokenizer applies its own context-aware
   math/symbol/script rules (`MathSymbolAware`). The same input therefore
   segments differently (e.g. `"Hello, world!"` → native `[24729, 4392, 5915, 33]`
   vs HF `[72, 15596, 44, 5915, 33]`). IDs differ while both decode to the same
   bytes.
2. **Decoding parity holds, encoding parity does not.** Every HF token ID maps to
   the exact byte sequence it was built from, so decode is lossless; only the
   segmentation boundary diverges.
3. **Special tokens are literal strings in the vocab and `added_tokens`.** They
   survive round-trips both natively and under HF, including
   `<|bos|> … <|assistant|>`; `AutoTokenizer` resolves them through
   `special_tokens_map.json`.
4. **`tokenizer_config.json` sidecar.** The exporter writes
   `tokenizer_class=PreTrainedTokenizerFast` and the four special-token entries so
   HF pipelines load it without hand-editing. It deliberately does **not** claim a
   `model_type` — there is none registered upstream.
5. **Structural export, not a port.** NTokenizer’s pre-tokenizer cannot be
   expressed as a standard HF pre-tokenizer chain (no stock class implements its
   script/digit/symbol binding with Unicode general categories). The export is for
   distribution and analysis; training-time use should re-pre-tokenize with the
   native tool.
6. The export was **not** used to produce any of the benchmark numbers in
   `BENCHMARK_REPORT.md` — those all come from the native harness.

Inspect with:

```bash
python -c "from transformers import AutoTokenizer; t = AutoTokenizer.from_pretrained('output/v0.3/hf'); print(t.vocab_size, t.special_tokens_map)"
python experiments/v0.3/validate_hf_export.py \
  --model output/v0.3/ntokenizer-0.3-256k.ntok \
  --tokenizer-json output/v0.3/hf/tokenizer.json \
  --out reports/v0.3/hf_export_validation.json
```

## 5. Python integration

`tests/test_python_bindings.py` covers loading a base tokenizer, loading a trained
`.ntok` from disk, encode/decode/batch operations, vocabulary size, special-token
access, NUL-byte handling, invalid-ID rejection, repeated-call memory stability,
and context-manager semantics — **all 14 pass**. The Python path decodes bytes with
`errors='replace'` at the FFI boundary where noted; native decode is byte-exact.

## 6. Known limitations (honest list)

- Token-ID parity between HF and native is **not** achievable with stock HF
  pre-tokenizers; users needing bit-identical IDs must use the native library or
  CLI.
- The Rust→Python FFI boundary copies buffers; for very large batches the native
  CLI/library is faster.
- Fuzzing was not executed for this release (pre-existing ASAN/linker issue).
- Only x86-64 Linux was measured; tier-2 platforms are untested here.
- `ntokenizer benchmark` (built-in comparative pretokenizer evaluation) is a legacy
  v0.2 diagnostic path and was not used for any v0.3 number.
