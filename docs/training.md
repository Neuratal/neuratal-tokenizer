# Training Documentation

Neuratal Tokenizer's trainer turns a text corpus into a `.ntok` byte-level BPE
model. Training is deterministic: the same corpus, seed, and configuration
produce byte-identical models and merge tables (verified by tests).

## Command line

```bash
ntokenizer train --input corpus.txt --vocab-size 16384 --min-freq 2 \
                  --pretokenizer conservative --seed 42 --output model.ntok
# manifest-based, mixed corpora (example): examples/manifest_example.json
ntokenizer train --manifest manifest.json --output model.ntok
```

`--pretokenizer` accepts `raw`, `conservative`, `script-digit`, `lws`
(`LeadingWhitespaceUnicode`, the v0.2 default via ablation), plus the v0.3 arms
investigated in `reports/v0.3/`: `script-transition` (`ScriptTransitionAware`),
`hybrid` (`UnicodeHybrid`), `code` (`CodeAware`), and `math`
(`MathSymbolAware`, the v0.3 release pretokenizer), plus the v0.4 CJK-chunk
variants `cjk2` (`MathSymbolAwareCjk2`), `cjk4` (`MathSymbolAwareCjk4`) and `cjk8`
(`MathSymbolAwareCjk8`, the v0.4 release candidate). Progress is printed to
stderr; `--quiet` suppresses it.

`--min-word-freq N` (default 1 = legacy behavior) drops unique pre-tokens that
occur fewer than `N` times before the pair index is built. `N=2` prunes 70–76%
of unique pre-tokens on multilingual corpora and reduces trainer peak RSS by
~3–4×; it was required to train the 256K v0.3 and v0.4 models on this machine
(see `reports/v0.3/TRAINING_REPORT.md`).

`--min-word-freq-cjk N` (v0.4) applies a *lower* frequency floor to pre-tokens
composed entirely of CJK (Han/Hiragana/Katakana/Hangul) characters. Note the
flat `--min-word-freq 2` floor is the root cause of the v0.3 CJK regression, and
the naïve remedy (exempting CJK entirely) is infeasible at production scale —
the CJK chunk arms are the memory-safe approximation. See
`reports/v0.4/CJK_UNICODE_REPORT.md`.

## Library API

```rust
use neuratal_tokenizer::{
    train_from_manifest, train_from_manifest_with_progress,
    CorpusManifest, CorpusManifestEntry, Normalization, PretokenizationMode,
    TrainingConfig,
};
```

`train_from_manifest(&manifest, &config)` runs silently;
`train_from_manifest_with_progress(&manifest, &config, true)` prints progress
to stderr. Both return `(Tokenizer, TrainingStats)`.
`train_from_manifest_with_progress_and_min_word_freq(&manifest, &config, true, n)`
additionally prunes unique pre-tokens below frequency `n` before building the
pair index; its `TrainingStats` carries `pruned_words_count` and
`training_words_count`.

### `TrainingConfig`

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `target_vocab_size` | `usize` | 16384 | Total vocab budget: 256 base bytes + learned merges + special tokens |
| `min_frequency` | `usize` | 2 | Minimum pair occurrence count to merge |
| `pretokenizer` | `PretokenizationMode` | `ConservativeUnicode` | `RawByte` / `ConservativeUnicode` / `ScriptAndDigitAware` / `LeadingWhitespaceUnicode` / `ScriptTransitionAware` / `UnicodeHybrid` / `CodeAware` / `MathSymbolAware` / `MathSymbolAwareCjk2` / `MathSymbolAwareCjk4` / `MathSymbolAwareCjk8` |
| `normalization` | `Normalization` | `None` | `None` (byte-exact), `Nfc`, `Nfd`, `Nfkc`, `Nfkd` |
| `special_tokens` | `Vec<String>` | bos/eos/pad/unk | Special token strings; IDs follow the learned merges |
| `seed` | `u64` | 42 | Deterministic sampling and tie-breaking |

`TrainingStats` reports: records read, bytes processed, malformed-record count
(with file:line samples), unique pre-tokens, initial pair/index sizes, merges
learned, duplicate-merge candidates skipped, final vocabulary size, and elapsed
milliseconds.

## Corpus formats

- **`.txt`**: one document per line; lines are pre-tokenized as-is.
- **`.jsonl`**: one JSON object per line; the text field is `text` by default
  (override via `text_field` in the manifest entry). Optional `language` and
  `domain` fields are carried into records (not used for weighting beyond
  `weight`). Malformed lines are counted and their samples reported in
  `TrainingStats` — they are never silently dropped, and they never abort
  training.
- Streaming: files are read line-by-line; the full corpus is never loaded into
  memory. Unique pre-token counts and the pair index are held in RAM (see
  *Memory* below).

## Manifest format

```json
{
  "seed": 42,
  "entries": [
    { "path": "data/en.txt", "weight": 1.0, "language": "en", "domain": "general", "text_field": null },
    { "path": "data/code.jsonl", "weight": 2.0, "text_field": "content", "language": null, "domain": "code" }
  ]
}
```

### Weighted sampling

For entries with `weight < 1.0`, each record is kept iff
`record_sample_value(seed, global_record_index) < weight`, where
`record_sample_value` is a SplitMix64-style mix of `(seed, index)` producing a
uniform value in [0, 1). This decision depends only on the seed and the
record's global index — not on thread scheduling or read order — so
single-threaded and parallel ingestion keep exactly the same records.

For `weight >= 1.0`, the record is repeated `round(weight)` times. Weight 2.0 =
every record twice.

## Ingestion architecture

Records are read in file order, grouped into fixed chunks of 1024, and each
chunk is processed with Rayon into per-record partial word-count maps that are
folded **in chunk order** into the global map (integer counts → order-independent
merge, byte-identical results). Malformed records are counted in all paths;
their file/line/reason samples are kept (first 10 only).

## BPE merge selection

- Array-backed doubly-linked lists per unique pre-token; word IDs assigned in
  byte-sorted order.
- Inverted pair-occurrence index (`(u32, u32) -> Vec<Occurrence>`).
- Frequency updates are incremental; adjacency-pair frequencies decremented by
  a merge are re-pushed into the priority queue with updated counts (stale
  entries are discarded lazily on pop).
- Priority selection is by `(frequency desc, pair asc)` — ties break toward the
  lexicographically smaller token pair, always deterministically.
- When a candidate merge's concatenated bytes already exist as a token (a
  different segmentation of the same byte string), the pair is discarded and
  selection continues; the event is counted as `skipped_duplicate_merges`.
- Rare sequences are never pruned. Unique pre-token counts are kept whole.

## Memory

Unique pre-token count + pair-occurrence index dominate. Measured growth
≈ 0.28–0.35 KB per unique pre-token (see `examples/memory_profile.rs` and
docs/benchmarks.md). The trainer is **not** constant-memory: for billion-scale
corpora, budget accordingly.

## Determinism guarantees

- Same corpus + same `TrainingConfig` (incl. `seed`) → byte-identical vocab,
  merge table, serialization. Covered by tests
  (`tests/training_regression_tests.rs`, `tests/phase3_training_tests.rs`).
- Parallel ingestion produces the same word counts as sequential (unit-tested
  against a sequential reference).
- Repeated serialization of the same model is byte-identical.

## Special tokens

Special tokens (`<|bos|>`, `<|eos|>`, …) receive IDs after the learned merges.
If a special token's literal bytes were merged as an ordinary token (corpus
contained e.g. `<|assistant|>` verbatim), the special token reuses that
existing vocabulary entry; see docs/ntok_format.md for the invariant this
creates and why it is safe.

## Exporting smaller vocabularies from a trained model

A trained model's learned merge order can be sliced into smaller-vocabulary
models. The exported model keeps the **first N merges in rank order** (merge
token IDs stay identical, bases are preserved, special tokens are renumbered
to the compact range), and is a fully valid, independent model.

```bash
# Single export
ntokenizer export --model main.ntok --vocab-size 16384 --output main-16k.ntok
# Batch export of the whole family
ntokenizer export --model main.ntok --sizes 8192,16384,32768,65536,131072,196608,262144 \
                  --output-dir exported/
```

The exported budget is a *cap*, not an exact size: if the model doesn't have
enough merges to fill `N`, the exported model keeps everything it has. Special
tokens may be renumbered (they are moved after the kept merges); merge token
IDs are always preserved. The exported model is validated (`validate`) and
saved as v2. Round-trip and independent-load tests for every export size live
in `tests/vocab_export_tests.rs`.
