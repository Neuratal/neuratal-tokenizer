# Contributing to Neuratal Tokenizer

Thanks for working on NTokenizer. This document describes how to build, test,
and submit changes.

## Ground rules

1. **Independent implementation.** Never import, vendor, or copy a tokenizer
   engine (Hugging Face Tokenizers, SentencePiece, tiktoken, …). No pretrained
   vocabularies or merge tables. Tokenizer logic lives in this repository only.
2. **Exact byte preservation.** `Normalization::None` (default) must round-trip
   byte-exactly. Do not weaken this.
3. **Determinism.** Same corpus + same `TrainingConfig` must produce
   byte-identical models and merge tables. Any ingestion or merge change must
   preserve single-thread/multi-thread equivalence.
4. **Verification before merge.** `cargo build --release` and
   `cargo test --release` must pass (they are the CI gate). `cargo clippy`
   is encouraged but not yet clean on the whole tree; the CI does not gate
   on it. Run `cargo fmt --all` before submitting.
5. **No fabricated results.** Report real measured numbers with machine context;
   never claim untested features (e.g., PyTorch integration) as verified.

## Development setup

```bash
cargo build                 # debug build
cargo build --release       # optimized: CLI + cdylib + rlib
cargo test                  # debug tests
cargo test --release        # release-mode tests (memory profiling runs here)
python3 tests/test_python_bindings.py   # Python integration tests
./examples/train_example.sh             # end-to-end demo
```

Requirements: Rust 1.85+ (edition 2024); Python 3.8+ for bindings;
optional `nightly` toolchain + `cargo-fuzz` for fuzz targets.

## Code style

- Match the surrounding style; `cargo fmt --all -- --check` is authoritative.
- No `unsafe` outside `src/ffi.rs` (C ABI layer) and FFI-adjacent paths; the
  FFI layer is panic-safe via `catch_unwind`.
- Comment *why*, not *what*. Public APIs carry rustdoc.
- Errors must be actionable: include the offending ID/path/value.

## Serialization changes

- Any change to the `.ntok` payload or header layout increments `FORMAT_VERSION`
  and updates `docs/ntok_format.md`.
- Backward compatibility for earlier versions is deliberate: older files must
  keep loading, or a documented migration must exist. Tests for the legacy
  format live in `src/serialization/format.rs` and
  `tests/serialization_security_tests.rs`.
- Corrupted files must always be *rejected*, never silently accepted.

## Tests

- Rust unit tests live next to the code; integration tests live in `tests/`.
  Fuzz targets live in `fuzz/` (see below).
- Add regression tests for every fixed bug.
- Multilingual coverage tests live in `tests/domain_coverage_tests.rs`; add
  samples to preserve exact round-trips.
- Determinism is a test, not a hope: see
  `tests/training_regression_tests.rs` for the pattern.

## Fuzzing

```bash
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz
CC=gcc CXX=g++ RUSTFLAGS="-C linker=gcc" cargo +nightly fuzz run roundtrip
CC=gcc CXX=g++ RUSTFLAGS="-C linker=gcc" cargo +nightly fuzz run load_model
```

(Note: on some systems the default `gcc` points to a zig-based shim that cannot
link ASan; the commands above pin a system gcc. See docs/benchmarks.md.)

## Documentation

- `README.md` is the entry point; deep dives live in `docs/`.
- If you change the user-visible surface (CLI, Python API, format), update all
  three: code, tests, docs — in the same commit.

## Licensing of contributions

By contributing, you license your contribution under the project's licenses
(MIT OR Apache-2.0, at your option), consistent with `LICENSE`.
