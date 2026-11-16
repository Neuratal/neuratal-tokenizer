# THIRD PARTY NOTICES

Neuratal Tokenizer is an independent implementation written from scratch in pure Rust.
It does not use, adapt, or copy source code, merge tables, or vocabularies from
Hugging Face Tokenizers, SentencePiece, tiktoken, or any other third-party
tokenizer engine.

The project depends on standard, permissively-licensed Rust infrastructure crates.
The list below was generated from `cargo metadata` (Cargo.lock at the time of this
writing). License identifiers are as reported by the corresponding crates.

## Direct dependencies

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| `clap` (+ `clap_builder`, `clap_derive`, `clap_lex`) | 4.6.7 / 4.6.7 / 4.6.7 / 1.1.1 | MIT OR Apache-2.0 | CLI argument parsing |
| `rayon` (+ `rayon-core`, `crossbeam-deque`, `crossbeam-epoch`, `crossbeam-utils`, `either`) | 1.12.0 / 1.13.0 / 0.8.8 / 0.9.21 / 0.8.23 / 1.19.0 | MIT OR Apache-2.0 | Parallel batch encoding/decoding and parallel corpus ingestion |
| `serde` (+ `serde_core`, `serde_derive`) | 1.0.229 | MIT OR Apache-2.0 | Serialization framework (feature `derive`) |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 | JSON payload encoding of the `.ntok` format |
| `unicode-normalization` (+ `tinyvec`) | 0.1.26 / 1.13.3 | (MIT OR Apache-2.0) AND Unicode-3.0 / Zlib OR Apache-2.0 OR MIT | UAX #15 normalization (NFC/NFD/NFKC/NFKD) |

### Optional: `pyo3` feature (probe, **not** built by default)

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| `pyo3` (+ `pyo3-ffi`, `pyo3-build-config`, `pyo3-macros`, `pyo3-macros-backend`) | 0.30.1 | MIT OR Apache-2.0 | Optional native Python binding (Cargo feature `pyo3`); see docs/python_integration.md |
| `target-lexicon` | 0.13.6 | Apache-2.0 WITH LLVM-exception | Target-triple parsing (PyO3 build-time dependency) |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 | One-time initialization (PyO3 dependency) |
| `portable-atomic` | 1.15.0 | Apache-2.0 OR MIT | Atomics used by PyO3 |
| `libc` | 0.2.191 | MIT OR Apache-2.0 | C library interface (PyO3 dependency, also used by the fuzz crate) |

### Dev-only: `fuzz/` crate (coverage-guided fuzzing; never linked into the library)

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| `libfuzzer-sys` | 0.4.13 | (MIT OR Apache-2.0) AND NCSA | libFuzzer bindings |
| `arbitrary` | 1.5.0 | MIT OR Apache-2.0 | Structured input generation |
| `cc` | 1.11.2 | MIT OR Apache-2.0 | C/C++ compile driver (build-time) |
| `cfg-if` | 1.0.5 | MIT OR Apache-2.0 | Conditional compilation helper |
| `find-msvc-tools` | 0.1.15 | MIT OR Apache-2.0 | MSVC toolchain discovery (Windows, build-time) |
| `getrandom` | 0.4.3 | MIT OR Apache-2.0 | Randomness (build-time) |
| `jobserver` | 0.1.35 | MIT OR Apache-2.0 | Build parallelism (build-time) |
| `libc` | 0.2.191 | MIT OR Apache-2.0 | C library interface |
| `r-efi` | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | UEFI-target support of libc; never compiled on Linux/macOS/standard Windows, LGPL alternative is not used |
| `shlex` | 2.0.1 | MIT OR Apache-2.0 | Shell lexing (build-time) |

## Transitive dependencies (as resolved by Cargo.lock)

| Crate | Version | License |
|-------|---------|---------|
| `anstream` | 1.0.0 | MIT OR Apache-2.0 |
| `anstyle` | 1.0.14 | MIT OR Apache-2.0 |
| `anstyle-parse` | 1.0.0 | MIT OR Apache-2.0 |
| `anstyle-query` | 1.1.5 | MIT OR Apache-2.0 |
| `anstyle-wincon` | 3.0.11 | MIT OR Apache-2.0 |
| `colorchoice` | 1.0.5 | MIT OR Apache-2.0 |
| `heck` | 0.5.0 | MIT OR Apache-2.0 |
| `is_terminal_polyfill` | 1.70.2 | MIT OR Apache-2.0 |
| `itoa` | 1.0.19 | MIT OR Apache-2.0 |
| `memchr` | 2.8.3 | Unlicense OR MIT |
| `once_cell_polyfill` | 1.70.2 | MIT OR Apache-2.0 |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 |
| `quote` | 1.0.47 | MIT OR Apache-2.0 |
| `strsim` | 0.11.1 | MIT |
| `syn` | 3.0.10 | MIT OR Apache-2.0 |
| `unicode-ident` | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| `utf8parse` | 0.2.2 | Apache-2.0 OR MIT |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 |
| `zmij` | 1.0.23 | MIT |

Unicode data tables used by `unicode-normalization` (via Unicode Character Database
files) are © Unicode, Inc., licensed under the Unicode License (Unicode-3.0).

Note: `memchr` and `utf8parse` are present in the dependency graph through other
crates (e.g., `serde_json`/`clap`); the project does not call their APIs directly.

## License texts

The license texts for MIT, Apache-2.0, Unlicense, BSD-style, Zlib, and Unicode-3.0
are available at:

- MIT: https://opensource.org/licenses/MIT
- Apache-2.0: https://www.apache.org/licenses/LICENSE-2.0
- Unlicense: https://unlicense.org/
- Zlib: https://opensource.org/licenses/Zlib
- Unicode-3.0: https://www.unicode.org/license.txt

## Project license

Neuratal Tokenizer itself is released under the terms in `LICENSE` (MIT OR Apache-2.0,
at your option). Copyright 2026 Neuratal Tokenizer Contributors.
