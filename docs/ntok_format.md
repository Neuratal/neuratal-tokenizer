# The `.ntok` File Format (NTOK)

`.ntok` is the binary serialization format for Neuratal Tokenizer (NTokenizer)
models. Format **version 3** ("NTOK" magic) is current (v0.2, crate 0.3.0).
Version 2 ("NTOK", v0.1) and legacy version 1 ("RZT1", pre-rename) still load.
The format wraps a deterministic JSON payload in a small integrity header.

The format is designed for: exact round-trip reproduction of models, strict
rejection of corrupted data, and simple cross-language reading (the Python
bindings load through the same Rust code path — they never parse `.ntok`
themselves).

## Layout

All integers are little-endian.

| Offset | Size | Field | Description |
|-------:|-----:|-------|-------------|
| 0 | 4 | magic | ASCII `NTOK` (`0x4E 0x54 0x4F 0x4B`) |
| 4 | 2 | format_version | `3` for the current format (`2` v2, `1` legacy) |
| 6 | 4 | crc32_ieee | CRC-32 (IEEE 802.3 polynomial, reflected, init `0xFFFFFFFF`, final XOR) of the payload bytes |
| 10 | 8 | payload_len | Length of the payload in bytes (max 1 GiB) |
| 18 | payload_len | payload | UTF-8 JSON encoding of the `Tokenizer` structure |

Header size (`HEADER_SIZE`) is 18 bytes. Total file size must equal
`18 + payload_len` exactly; any missing or extra bytes are rejected.

## Payload structure

The payload is `serde_json`-serialized `Tokenizer` with fields:

```json
{
  "vocab": { "id_to_bytes": [[0], [1], ..., [255], [72, 101], ...] },
  "merges": { "rules": [ { "left": 101, "right": 104, "target": 256, "rank": 0 }, ... ] },
  "special_tokens": { "tokens": [ { "id": 300, "content": "<|bos|>", "lstrip": false, "rstrip": false }, ... ] },
  "pretokenizer": "LeadingWhitespaceUnicode",
  "normalization": "None",
  "metadata": { ... }               // v3 only — see "Embedded metadata" below
}
```

### Embedded metadata (v3)

v3 payloads may embed a `metadata` object describing how the model was produced.
v2 files carry none (they load as `metadata: None`). v3 writer always emits it.
Fields:

```json
{
  "magic": "NTOK", "format_version": 3, "algorithm": "byte_level_bpe",
  "model_name": "ntokenizer-0.2-256k", "model_version": "0.3.0",
  "tokenizer_crate_version": "0.3.0", "training_tool": "neuratal-tokenizer (Rust)",
  "created_at": "2026-11-15T...Z", "training_started_at": "...", "training_finished_at": "...",
  "vocab_size": 262144, "base_byte_tokens": 256, "merges_count": 261881,
  "special_tokens_count": 7, "special_token_list": ["<|bos|>", ...],
  "pretokenizer_mode": "LeadingWhitespaceUnicode", "pretokenizer_version": "1",
  "normalization_mode": "None", "normalization": "None",
  "min_frequency": 2, "seed": 42,
  "corpus_manifest_sha256": "<sha256 of the training manifest>",
  "corpus_bytes": 1190081343, "corpus_records": 8033987,
  "training_cli": "ntokenizer train --manifest ... --vocab-size 262144 ...",
  "licenses_reference": "data/v0.2/LICENSES.md", "git_commit": ""
}
```

The loader cross-checks `metadata.vocab_size`, `merges_count`, `special_tokens_count`
and `base_byte_tokens` against the actual payload and rejects mismatches.
A `MODEL_METADATA.json` sidecar next to the model carries the same object plus
`model_file`, `model_sha256` and `metadata_sha256`, binding the metadata to the
exact model bytes.

### `vocab`

- `id_to_bytes[i]` is the byte sequence of token id `i`. Ids are dense from 0.
- IDs 0..=255 are immutable single bytes `[0]..[255]` (verified on load).
- Higher IDs are learned merge targets or special tokens.

### `merges`

- `rules` is ordered by `rank`; ranks are contiguous starting at 0 (verified).
- Each rule merges the byte sequences of `left` and `right` into `target` whose
  bytes must equal the concatenation (verified).
- `target` IDs are unique and ≥ 256 (verified).

### `special_tokens`

- Token `id`s are unique, ≥ 256, and never point at bytes different from their
  `content`. A special token ID may legally reference either no vocabulary entry
  (decode uses the registry) or the vocabulary entry with exactly matching bytes
  (e.g., the literal special-token string was trained into the model and the
  special token reuses that entry).
- `lstrip`/`rstrip` are currently always `false` (reserved).

### `pretokenizer`

One of `"RawByte"`, `"ConservativeUnicode"`, `"ScriptAndDigitAware"`, `"LeadingWhitespaceUnicode"` (v0.2, ablation winner).

### `normalization`

One of `"None"`, `"Nfc"`, `"Nfd"`, `"Nfkc"`, `"Nfkd"`.

## Validation rules on load

The loader rejects the file on any of the following:

- File shorter than 18 bytes.
- Wrong magic or `format_version` other than `2` (v2) or `1` (legacy v1, see below).
- `payload_len` exceeding 1 GiB (`MAX_PAYLOAD_BYTES`), or `18 + payload_len`
  not equal to the actual file size (truncation, splicing, trailing garbage).
- CRC-32 mismatch (bit rot, truncation, manual editing).
- Payload that is not valid UTF-8 JSON or not a `Tokenizer`.
- Vocabulary/merge/special-token structure failing consistency checks:
  - Base bytes 0..=255 present and correct; vocabulary ≥ 256 tokens.
  - Merge ranks contiguous from 0, targets unique and ≥ 256, referenced IDs in range,
    `bytes(target) == bytes(left) ++ bytes(right)`.
  - Special token IDs ≥ 256, unique, and any referenced vocabulary bytes equal
    the special token's content.
- v3 embedded metadata (when present): `magic == "NTOK"`, `vocab_size`, `merges_count`,
  `special_tokens_count`, and `base_byte_tokens == 256` must match the actual payload.

## Versioning and backward compatibility

- **Format version 3 (current)**: magic `NTOK`, version 3. Embeds `metadata`.
- **Format version 2**: magic `NTOK`, version 2. No embedded metadata; loads as `metadata: None`.
- **Format version 1 (legacy)**: magic `RZT1`, version 1. Produced by earlier
  revisions of this project (pre-0.2.0, when the project was named RZTokenizer).
- Header layout is identical across all three; only magic/version pair and the
  optional `metadata` payload key differ. The loader accepts (`NTOK`, 3),
  (`NTOK`, 2), and (`RZT1`, 1), and rejects anything else (including mixed
  pairs) with a descriptive error.
- Serialization always writes v3. Migration path: `ntokenizer migrate --input model.rzt|model.ntok --output model.ntok`
  (loads v1/v2, re-saves v3 with metadata if the loaded model carries any), or
  simply load and re-save.
- v0.2's exports (`ntokenizer export`) re-embed *derived* metadata (correct
  vocab size / merge count / `<base>-<size>` name); the parent model is never
  mutated.
- If the payload or header layout ever changes again, `FORMAT_VERSION` will be
  incremented and loading older versions will fail with a descriptive error
  rather than silently misinterpreting the file.

## Determinism

Serialization is deterministic for a given tokenizer: struct field order is
fixed, all payload maps are serialized as ordered vectors/strings, and no
timestamps are embedded in the payload. Without embedded metadata, saving the same tokenizer twice
produces byte-identical output. With embedded metadata, output is identical for
the same model **and the same `SOURCE_DATE_EPOCH`** (metadata carries wall-clock
timestamps by default; set `SOURCE_DATE_EPOCH=<unix-seconds>` for reproducible
bytes). CRC-32 is for corruption detection only and is **not** a cryptographic
authenticity check.

## Size expectations

The JSON payload is verbose: expect ~40–60 KB per 1000 vocabulary entries plus
the merge table (e.g., ~1–3 MB for a 32k-token model). A compact binary encoding
was deliberately not introduced in v2 to keep the format introspectable
(decode with any JSON tool). Change is gated by a version bump, not silently.
