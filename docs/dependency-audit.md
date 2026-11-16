# Dependency and Originality Audit

This document records the audit of every dependency used by Neuratal Tokenizer
(NTokenizer), how each one is used, and the review confirming the engine is an
independent implementation. It is updated whenever dependencies change.

Last verified: 2026-11-14, against `Cargo.lock` (see THIRD_PARTY_NOTICES.md for
the full machine-readable list).

## Audit method

```bash
cargo metadata --format-version 1 --no-deps
cargo metadata --format-version 1                # full graph with licenses
```

Plus manual review of the source tree for any tokenizer-engine code paths.

## Direct dependencies (default build)

| Crate | Version | License | Why it is needed | Restrictions |
|-------|---------|---------|------------------|--------------|
| `serde` (+ `serde_core`, `serde_derive`) | 1.0.229 | MIT OR Apache-2.0 | Serialization/deserialization of the `.ntok` payload (custom `Serialize`/`Deserialize` impls for `Vocabulary`, `MergeTable`, `SpecialTokenRegistry`) | None beyond license attribution |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 | JSON encoding of the payload; also used by benchmarks and corpus ingestion (JSONL) | None |
| `clap` (+ `clap_builder`, `clap_derive`, `clap_lex`) | 4.6.7/4.6.7/4.6.7/1.1.1 | MIT OR Apache-2.0 | CLI parsing for `ntokenizer` | None |
| `rayon` (+ `rayon-core`, crossbeam family) | 1.12.0 | MIT OR Apache-2.0 | Data-parallel batch encode/decode; parallel corpus ingestion (thread-local partial maps, deterministic fold) | None |
| `unicode-normalization` (+ `tinyvec`) | 0.1.26/1.13.3 | (MIT OR Apache-2.0) AND Unicode-3.0 / Zlib OR Apache-2.0 OR MIT | UAX #15 normalization (NFC/NFD/NFKC/NFKD) | Unicode-3.0 notice for Unicode data tables |

## Optional dependencies (not built by default)

| Crate | Version | License | Why it is needed | Restrictions |
|-------|---------|---------|------------------|--------------|
| `pyo3` family | 0.30.1 | MIT OR Apache-2.0 (+ `target-lexicon`: Apache-2.0 WITH LLVM-exception) | Optional native Python probe (`--features pyo3`), evaluated against ctypes in docs/python_integration.md | None beyond license attribution |

## Dev-only dependencies (fuzz crate, not shipped)

`fuzz/` contains a standalone `neuratal-tokenizer-fuzz` crate used with cargo-fuzz:
`libfuzzer-sys` (0.4.13, MIT OR Apache-2.0 AND NCSA) and its build/runtime
helpers (`arbitrary`, `cc`, `cfg-if`, `find-msvc-tools`, `getrandom`, `jobserver`,
`libc`, `r-efi`, `shlex`). These are never linked into the tokenizer library.
`r-efi` (MIT OR Apache-2.0 OR LGPL-2.1-or-later) is a UEFI-only dependency of
`libc`; the LGPL alternative path is never compiled in practice.

## What is NOT in the dependency tree

- **No external tokenizer engine.** No `tokenizers` (Hugging Face), no
  `sentencepiece`, no `tiktoken`, no `protobuf` model files, no pretrained
  `spm.model`, no pretrained merge tables or vocabularies.
- **No tokenizer data files** ship in this repository. The `.ntok`/`.rzt` models
  under `output/` are git-ignored and user-produced.
- The BPE training algorithm, pair counting, incremental updates, merge
  selection, encoding, decoding, serialization, and CLI are implemented
  in-tree (`src/`).

## Originality statement

Neuratal Tokenizer's engine (byte-level BPE training and inference, the
normalization layer, serialization format, CLI, Python bindings) is an
independent implementation written from scratch for this project. Generic
infrastructure crates (serde, clap, rayon, unicode-normalization) are used only
for supporting functionality — data serialization, CLI parsing, data parallelism,
and Unicode normalization respectively — never as a tokenizer. No source code
from Hugging Face Tokenizers, SentencePiece, tiktoken, or any other tokenizer
project was copied, adapted, vendored, or translated.

This statement applies to the code in this repository. It is not a claim about
the general ecosystem of tokenization research (BPE algorithms published in open
literature are used publicly by design).

## Restrictions and attribution

- All shipping dependencies are MIT OR Apache-2.0 or compatible (Unicode-3.0 for
  Unicode data tables). Full texts and links live in `THIRD_PARTY_NOTICES.md`.
- `unicode-normalization` embeds Unicode Character Database tables; the Unicode
  license (https://www.unicode.org/license.txt) must be retained for those
  tables. This is satisfied by the attribution in `THIRD_PARTY_NOTICES.md` and
  the Unicode-3.0 entries in its dependency list.
- Copyright 2026 Neuratal Tokenizer Contributors (see `LICENSE`).
