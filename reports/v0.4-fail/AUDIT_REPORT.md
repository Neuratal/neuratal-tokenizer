# NTokenizer v0.4 Audit Report

Date: 2026-11-15
Status: audit complete (code read, baselines re-verified). No source changes were
made in Stage 0. Model hashes re-verified against the v0.3 release records.

## 1. Repository state

| Item | Observation |
|---|---|
| Git | `master` has no commits (unborn branch). All work is staged/untracked; 20 paths carry both staged and unstaged changes from earlier sessions. Nothing has been pushed anywhere. |
| v0.1 model | `output/ntokenizer-256k.ntok` SHA-256 `8abe252f159ed58a1a0072eaac0f9861dab74b4e6bed5da9b69a218493662f00` — intact |
| v0.2 model | `output/v0.2/ntokenizer-0.2-256k.ntok` SHA-256 `1fc3ee8568b7a08a124a5e27ee7a8078d5bdfae495a253b3f97ba0ad29ab54ac` — intact |
| v0.3 model | `output/v0.3/ntokenizer-0.3-256k.ntok` SHA-256 `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` — intact (checked against `reports/v0.3/SHA256SUMS`) |
| Working tree | contains v0.3 deliverables (`reports/v0.3/`, `experiments/v0.3/`, `output/v0.3/`, `data/v0.3/`, `data/v0.4/`), the v0.3 research harness (`examples/benchmark_ntok.rs`), and the pre-existing user changes to `src/*`, `tests/*`, `docs/*`, `Cargo.*` |
| Tests | `cargo test --release`: 126 unit+integration tests pass; Python ctypes bindings: 14/14 pass |

## 2. Architecture (as read, not as remembered)

`src/lib.rs` exposes `Tokenizer` (vocab/merges/special tokens/pretokenizer/normalization/metadata).
Encoding path: `encoding/pretokenizer.rs` → `encoding/encoder.rs` (BPE with lazy-validated pair index) → `decoding/decoder.rs` (byte-exact). Training path: `training/corpus.rs` (streaming, line-oriented, retains terminators) → `training/bpe_trainer.rs` (chunked parallel ingestion with thread-local maps, deterministic merge order, `HashMap<Vec<u8>, Vec<u32>>` words with counts, `pair_counter.rs` for the inverted index + lazy-invalidated `BinaryHeap` merge selection, `Word { count, nodes: Vec<SymbolNode> }` doubly-linked lists) → `serialization/format.rs` (NTOK v3: magic `NTOK` + CRC32 + JSON payload, atomic temp+rename save, loader cross-checks embedded metadata against payload).

Deterministic training: ingestion is chunked in fixed-size batches processed in parallel, with maps merged in batch order; the merge tie-break is `freq desc, pair asc`. `Normalization::None` is default; byte input API (`encode_bytes`) is byte-exact and tested for round-trip.

Trainer memory control: `--min-freq` (merge eligibility floor), `--min-word-freq` (pre-token corpus-frequency floor before the pair index is built). v0.3 used `--min-word-freq 2`, which pruned 14.56M of 20.25M unique pre-tokens (71.9%).

## 3. Verified v0.3 baseline (re-measured this session)

| Check | v0.3 result |
|---|---|
| Token count (544 files / 207,712,944 B) | 37,397,108 — reproduced |
| Byte-weighted B/T | 5.5543 (measured 5-rep median, single-thread) |
| Equal-category B/T | 5.1057 |
| Used vocabulary | 245,334 / 262,144 (93.59%) |
| Decode exactness | 544/544 byte-exact |
| o200k_base (tiktoken, independent) | 4.0751 B/T, 50,971,306 tokens — matches v0.3 report |
| v0.2 baseline | 42,140,653 tokens, 4.9290 B/T — reproduced |
| Training | 1.963 GB manifest, 261,881 merges, 5 min 47 s wall, 3.70 GiB peak RSS |

## 4. Known gaps carried into v0.4 (all confirmed in-code or by re-measurement)

1. **CJK tokenization regression** (v0.3 vs v0.2): chinese −5.76%, japanese −6.75%,
   other_languages −15.91% on the old benchmark. Root-caused this session: with
   `--min-word-freq 2`, unsegmented CJK runs are unique pre-tokens (long sentences),
   so ALL pairs inside Chinese/Japanese text are dropped from the pair index. See
   `CJK_UNICODE_REPORT.md`.
2. **Encoding throughput**: v0.3 10.18 MB/s single-thread / 56.30 MB/s 8-thread vs
   v0.2 11.61 / 65.10 MB/s. Not benchmarked against a profiler yet (planned).
3. **HF export is structural only**: token-ID parity 3/16. Decode is byte-exact both
   ways. Full parity impossible with stock HF pre-tokenizers (MathSymbolAware cannot
   be expressed); see `HUGGINGFACE_COMPATIBILITY.md`.
4. **Benchmark self-reference**: the old 544-file benchmark was used throughout
   v0.3 development; it is fair for v0.2/v0.3/v0.4 (none trained on it) but it is no
   longer a pristine final test. A new, independent holdout was built
   (`BENCHMARK_SPEC.md`, `data/v0.4/benchmark/`).
5. **Memory blow-up**: training the full 1.96 GB corpus with CJK pre-tokens exempt
   from pruning exploded the pair heap (235M entries, 8.3 GB RSS at 3.5% of merges);
   aborted. Evidence recorded in `CJK_UNICODE_REPORT.md` and `MEMORY_REPORT.md`.
6. **Fuzzing**: never run (ASAN/linker issue pre-exists). Atomic save has no unit
   test with fault injection.
7. **Reports cite `.venv` as the Python env for HF validation; in fact HF validation
   ran in `/tmp/opencode/hfvenv` (tokenizers 0.23.3, transformers 5.23.0)**; corrected
   in `LICENSE_AUDIT.md`.
8. **Chinese single-token coverage**: 99.43% (v0.2, weighted) → 96.28% (v0.3); unique
   chars 78.4% → 44.5%. Direct cause of (1).

## 5. Bottlenecks and risks (ranked)

1. CJK pair retention vs trainer memory — the central engineering problem of v0.4.
2. MathSymbolAware per-byte overhead (~1.4× LWS cost per byte) — needs profiling before any change.
3. Determinism of the parallel trainer across runs — verified on every benchmark repeat by the harness (panic on drift); no cross-version guarantee (models change when corpus or code changes).
4. License ambiguity of distributing CC BY-SA–derived vocabulary/merges (separate analysis in `LICENSE_AUDIT.md`).
5. External validity: the old benchmark mixes natural language with code/STEM; v0.4 adds per-bucket bootstrap CIs and splits by pool/domain.
6. Untested: atomic-save fault injection; fuzz corpus; Python binding coverage for the new arms.

## 6. Files likely to change in v0.4 (expected)

| Area | Files |
|---|---|
| Trainer (CJK arms) | `src/training/bpe_trainer.rs`, `src/main.rs`, `src/lib.rs` |
| Pretokenizer (CJK chunk arms) | `src/encoding/pretokenizer.rs` |
| Benchmark harness | `examples/benchmark_ntok.rs` (per-file output added) |
| Benchmarks/results | `experiments/v0.4/`, `reports/v0.4/` |
| Model metadata | `docs/`, `CHANGELOG.md`, `output/README.md` when a release candidate exists |

No release/tag/upload decisions have been made; nothing leaves this machine.
