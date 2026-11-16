# Changelog

All notable changes to Neuratal Tokenizer are documented here. The format is
inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

No release has been tagged yet; entries through 0.2.0 document the development
history honestly, including renames.

## [Unreleased] — 2026-11-15 — NTokenizer v0.3-v2: engineering release, model unchanged

**v0.3-v2 is a CODE-ONLY IMPROVEMENT, not a new model.** The model of record remains
`output/v0.3/ntokenizer-0.3-256k.ntok` (SHA-256
`1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9`, unchanged).
Encoding is ~+60–70% faster single-thread and ~+65–72% faster at 8 threads (same-session
interleaved A/B), with equal-or-lower peak RSS and an identical token stream. A real
Unicode data-loss bug was fixed: combining marks at the start of a string were silently
dropped by the production pre-tokenizer (MathSymbolAware family) and by
LeadingWhitespaceUnicode. Fix + regression tests (`tests/unicode_regression_tests.rs`).
Details: `reports/v0.3-v2/FINAL_REPORT.md`.

### What changed
- Fix: leading combining marks no longer lost in `split_structured_unicode` and
  `split_leading_whitespace_unicode`; all-mark inputs are kept whole.
- Fix: `is_combining_or_joiner` classifies marks by Unicode general category
  (Mn/Mc/Me) in the legacy splitters (production MathSymbolAware unaffected).
- Perf: merge pair-table uses a multiplicative integer hasher instead of SipHash.
- Perf: ASCII fast path in structured script classification (skips script-table search).
- Perf: reusable BPE scratch buffers per top-level encode call (fewer allocations).
- New `examples/unicode_audit.rs` adversarial Unicode round-trip probe (40 samples x 2 production modes).
- Fuzzing: `roundtrip` (924k runs) and `load_model` (83k runs) pass with
  `cargo +nightly fuzz run <target> --sanitizer=none`; ASAN still blocked
  (pre-existing linker issue) — reported, not claimed as complete.
- Tests: 136/136 Rust green (was 130); Python bindings 14/14; cleaner 19/19;
  decode 544/544 byte-exact. Benchmark bytes/tokens slightly differ from the recorded
  v0.3 run (+211 tokens, +0.00056%, pre-existing in the delivered source; per-file
  identical to the pre-fix build) — see `reports/v0.3-v2/FINAL_REPORT.md` §D.
- v0.4-fail CjkHan4 candidate re-measured and again rejected (5.5511 B/T continuity,
  japanese −0.6%); v0.3 stays the default. No vocabulary was retrained or promoted.

## [Unreleased] — 2026-11-15 — NTokenizer v0.4: release candidate + benchmark v1

The v0.4 release candidate is `output/v0.4/ntokenizer-0.4-256k.ntok`
(MathSymbolAwareCjk8): identical corpus/seed to v0.3 with CJK runs capped at 8
characters. Measured (see `reports/v0.4/`): 5.5511 B/T overall / 5.1032
equal-category — parity with v0.3 (5.5543/5.1057); chinese +0.8% (old benchmark),
japanese −0.6%, everything else within 0.13%; encode speed identical to v0.3
(10.18 / 56.4 MB/s). v0.3 remains the recommended default (`output/v0.3/`);
the candidate exists for CJK-favoring deployments. Crate remains 0.3.0, model
family label "v0.4"; next tag will bump the crate.

### What changed
- New `MathSymbolAwareCjk2/4/8` pretokenizer modes: Han/Hiragana/Katakana/Hangul
  runs split into ≤2/4/8-character chunks. Root-caused the v0.3 CJK regression to
  flat `--min-word-freq 2` pruning of unsegmented CJK runs (ablations, incl. the
  infeasible-at-scale CJK-exempt arm).
- `--min-word-freq-cjk N` trainer flag: lowers the word-frequency floor for
  pre-tokens composed entirely of CJK script characters (memory-heavy; use with
  care — it is infeasible at production scale in full-exempt form).
- Benchmark v1: unused-pool holdout + validation splits, paper-level arXiv
  grouping, synthetic Unicode stress fixtures, leakage audit passed
  (`reports/v0.4/BENCHMARK_SPEC.md`, `leakage_audit.json`), paired file-level
  bootstrap CIs in the harness (`--per-file-json`).
- New HF-export tooling/validation: `examples/export_hf.rs` produces a structural
  tokenizer.json (byte-exact decode both ways); token-ID parity explicitly NOT
  claimed (46/523 measured; plain export is structural only).
- Tests: 129/129 Rust green; new atomic-save fault-injection tests
  (`tests/atomic_save_tests.rs`); new CJK chunk unit tests; Python bindings 14/14.
- Full battery: parity-aware BPE, Unigram, and Nouri MDL-SG (skipped) evaluated —
  all rejected; see `reports/v0.4/ALGORITHM_COMPARISON.md`.

## [Unreleased] — 2026-11-15 — NTokenizer v0.3: measured multilingual rebuild

The v0.3 model series is a retrained checkpoint with the same NTOK v3 format and
the same 256K vocabulary budget as v0.2. The crate version in this working tree
is intentionally unchanged (0.3.0); the v0.3 model was trained and benchmarked with
crate 0.3.0, and the next tagged release will bump the crate (see
`output/v0.3/ntokenizer-0.3-256k.ntok`, embedded metadata). v0.2 artifacts are
untouched and remain the baseline.

### What changed
- New `MathSymbolAware` pretokenizer (`math`): decimal/time/operator binding,
  Unicode general-category symbol and prefix/suffix rules, selected by a controlled
  tournament (32K/20MB → 64K/60MB) against `UnicodeHybrid`, `CodeAware`,
  `ScriptTransitionAware`, LWS, and RawByte.
- New `UnicodeHybrid`, `CodeAware`, `ScriptTransitionAware` pretokenizer modes
  (legacy modes frozen).
- `--min-word-freq N` trainer flag: drops unique pre-tokens below frequency `N`
  before pair indexing (default 1 = legacy). `N=2` cut final training peak RSS to
  3.70 GiB (from ~9.7 GB for v0.2) on 1.93× the corpus.
- New native benchmark harness (`examples/benchmark_ntok.rs`): byte-weight- and
  equal-category-weighted B/T, per-bucket, used vocab, latency, ≥5-rep warm medians,
  single/multi thread, decode verification.
- `--min-word-freq`, pretokenizer aliases and help text; `export_hf` example:
  structural Hugging Face `tokenizer.json` export (GPT-2 byte-level alphabet, merges
  in rank order, special tokens as added tokens) with compatibility caveats —
  byte-exact decode, no token-ID parity guarantee.

### Measured result (unchanged held-out benchmark, 544 files / 207.7 MB)
- v0.3: **5.5543 B/T** byte-weighted, **5.1057 B/T** equal-category,
  37,397,108 tokens — **+12.7% / +9.0%** vs v0.2, **−26.6% tokens** vs o200k_base,
  544/544 files byte-exact decode.
- Regressions vs v0.2 disclosed: chinese −5.76%, japanese −6.75%,
  other_languages −15.91% (root cause: CJK single-token coverage drop from
  `--min-word-freq 2`); encode throughput −12%/−14% (heavier pre-tokenizer).
- 8.15 B/T (2× o200k) stretch target **not achieved**; see `reports/v0.3/`.
- Full evidence: `reports/v0.3/{AUDIT,TRAINING,BENCHMARK,PERFORMANCE,MEMORY,
  COMPATIBILITY,LICENSE_AUDIT,FINAL}_REPORT.md` and `EXPERIMENT_RESULTS.csv`.

## [0.3.0] — 2026-11-15 — NTokenizer v0.2: Clean Multilingual Release

The v0.2 model series is a full rebuild of the data pipeline, pretokenizer,
benchmark and metadata layers. v0.1 (`output/ntokenizer-256k.ntok`) is retained
unchanged as the baseline.

### Pretokenizer (breaking, ablation-driven)
- New `LeadingWhitespaceUnicode` pretokenizer ("lws"): script-aware Unicode
  splitting with leading-whitespace attachment — runs of spaces/tabs merge
  with the following lexical run, so BPE can learn `" the"`, `"  def"`,
  `"  漢字"`. Newlines never attach; combining marks/ZWJ stay attached;
  CJK/Hangul/kana and emoji runs stay whole; tabs are never normalized.
- Selected by ablation pilots (32K vocab, seed 42, same training slice):
  english 2.31→3.30 bytes/token averaged +43% vs v0.1's default; no language
  regressed vs v0.1 (arabic +46%, hindi +40%, spanish +55%, turkish +36%);
  CJK unchanged-to-slightly-better; raw-byte pilot wins on compression but is
  infeasible at full scale (6.9 GB RSS for a 150 MB slice).

### Corpus pipeline (rewritten)
- Binary detection that actually works (v0.1 silently fell back to Latin-1 and
  ingested 91 binary test-fixture files ≈19 MB): magic-byte signatures, NUL
  bytes, strict UTF-8, >0.5% unexpected control characters, 64 MiB cap.
  **1,646 files (453 MB) rejected and logged with reasons** in this build.
- Whitespace preservation: leading indentation and tabs are NEVER stripped
  (v0.1's `line.trim()` destroyed ~21.8 MB of code indentation); line endings
  normalized CRLF/CR→LF; trailing-whitespace policy per content class
  (prose strips, code verbatim); blank lines preserved; reader keeps the
  line terminator in each record so `\n` merges are learnable.
- Unicode-safe dedup: exact-hash document dedup + word-5-gram bottom-128
  near-dup (char-8-gram for CJK), Jaccard ≥ 0.5, train-side quarantine.
  No case folding, no punctuation stripping, no prefix truncation, no
  normalize-then-compare. Hindi matras, Turkish İ, emoji/ZWJ lines are never
  mangled; symbol-only lines are never auto-dropped.
- Benchmark is now genuinely held out: page/book/sentence-hash assignment +
  108 train files quarantined by ≥0.6 shingle containment vs benchmark.
  Leakage audit: file-exact 0, normalized-exact 0, containment 0, and 60
  distinct ≥100-char overlapping lines, all machine-generated boilerplate
  (attribute lists, banner comments, generated tables, LaTeX preambles).
- CJK is now real content: jawikisource, kowikisource, zhwiktionary,
  jawiktionary, kowiktionary + 435 Chinese/21 Japanese Gutenberg books
  (~321 MB train). STEM tripled (~90 MB); English share dropped from 48.9% to 25.5%.
- The corpus has per-document SHA-256 + per-source license/URL/retrieval-date
  (236,239 raw files fingerprinted in `data/v0.2/manifest/raw_provenance.json`).

### Serialization (v3)
- Format version 3 (`NTOK`) embeds a `ModelMetadata` object (model name,
  version, created_at, vocab/merge/special counts, pretokenizer + version,
  min frequency, seed, corpus manifest SHA-256, corpus bytes/records, training
  CLI, licenses reference, git commit). v1 (RZT1) and v2 (`NTOK`) still load.
- Loader validation extended: embedded metadata is cross-checked against the
  payload (vocab/merges/special/base-byte counts).
- `ntokenizer train` writes metadata automatically (incl. `SOURCE_DATE_EPOCH`
  support for reproducible bytes). `ntokenizer export` re-derives per-export
  metadata. `MODEL_METADATA.json` sidecar binds model+metadata via SHA-256.

### CLI/tests
- New pretokenizer selectable via `--pretokenizer lws`.
- Corpus reader keeps line terminators; blank lines are records.
- Rust: 122 tests pass (42 unit + 80 integration).
- New tests: leading-whitespace pretokenizer behaviors (code, CJK, emoji,
  Arabic, Hindi, tabs), metadata round-trip + cross-check, v3 load/validate.

### Documentation
- v0.2 reports in `reports/v0.2/`, corpus docs in `data/v0.2/`,
  `MODEL_METADATA.json` next to models, format docs updated for v3.

## [0.2.0] — 2026-11-14 — Project rename: RZTokenizer → Neuratal Tokenizer

### Renamed
- Project officially renamed to **Neuratal Tokenizer**; short name **NTokenizer**.
- Rust crate: `neuratal-tokenizer` (library target `neuratal_tokenizer`).
- CLI executable: `ntokenizer`.
- Python package: `neuratal_tokenizer` (installable via `pip install -e python/`).
- FFI symbols: `rztokenizer_*` → `neuratal_tokenizer_*`.
- Model extension: `.rzt` → `.ntok`; file magic `RZT1` → `NTOK`; format version 1 → 2.

### Compatibility
- **`.rzt` (v1) files still load** via the same code path (identical header
  layout and payload; only the magic/version pair differ). Verified by unit
  tests (`test_load_legacy_v1_file`) and end-to-end CLI migration:
  `ntokenizer migrate --input model.rzt --output model.ntok`.
- Serialization always writes v2 (`NTOK`) after migration/re-saving.

### Changed
- BPE trainer: pairs decremented but still eligible for merging are re-queued
  (previously, such pairs could vanish from selection); zero-frequency pairs
  release their occurrence lists; word-ID assignment is byte-sorted.
- Duplicate merge candidates (same bytes produced by two segmentations) are
  discarded and counted (`TrainingStats::skipped_duplicate_merges`) instead of
  aborting training with an error. Special tokens whose literal bytes were
  trained reuse the existing vocabulary entry.
- Encoder: O(n²) rescan replaced by a min-heap keyed on (rank, position) —
  same token output, better asymptotics.
- Normalizer: `Nfc`/`Nfd`/`Nfkc` were silent identity no-ops; they now apply
  UAX #15 via `unicode-normalization` (`Nfkd` added). `Normalization::None`
  remains byte-exact and is the default.
- Training ingestion is parallelized (Rayon) with deterministic per-record
  sampling; progress reporting on stderr; `TrainingStats` gains
  `skipped_duplicate_merges`.
- `.ntok` loader: exact file-size/payload-length check (truncation/splice/
  trailing-garbage detection), 1 GiB payload cap enforced before allocation,
  structural validation (vocab/merges/special tokens) on load; saves are atomic.
- Python bindings: binary-safe `encode` (embedded NULs round-trip), decode via
  raw bytes (93 MB/s measured), `special_tokens` access, context manager,
  `TokenizerError`, fast `array.array`-based ID validation. PyO3 A/B benchmark
  added as an optional probe (not shipped by default).
- `validate` CLI: integrity + structural checks + optional multilingual
  round-trip probes; exit code 0/1.
- New tests: property-style round-trip (random bytes/Unicode), serialization
  adversarial suite, training regression suite, multilingual/math/code domain
  coverage suite, Python integration suite.
- New docs: `CONTRIBUTING.md`, `CHANGELOG.md`, `docs/ntok_format.md`,
  `docs/training.md`, `docs/benchmarks.md`, `docs/dependency-audit.md`,
  `docs/python_integration.md`, `docs/vocab_size_analysis.md`.

### Verification
- 102 Rust tests pass in debug and release; 14 Python integration tests pass;
  coverage-guided fuzzing (300k roundtrip runs, 60k seeded loader runs): 0 crashes.

## [0.1.0] — 2026 (pre-rename, "RZTokenizer")

Unreleased development history (kept for honesty, not published):

- Byte-level BPE engine, training pipeline, CLI, serialization, Python ctypes
  bindings, three pre-tokenization strategies, Phase 2–5 feature work.
- Project renamed to Neuratal Tokenizer in 0.2.0; model format v2 with v1
  compatibility introduced at the same time.

## [Unreleased] — 2026-11-16 — NTokenizer v0.4-v2: targeted quality cycle — NOT PROMOTED

Goal was overall B/T strictly > 6.00 (polished v0.3-v2 measures 5.5542). Not achieved and
not approached. **No source changes; no model promoted; polished v0.3-v2 and the v0.3
checkpoint (`output/v0.3/ntokenizer-0.3-256k.ntok`, SHA-256 `1f0d4ad7…`) remain the model
of record.** `output/v0.4-v2/` contains no model bytes by design.

### What happened
- Baseline reproduced (37,397,319 tokens, 5.5542 B/T, 544/544 byte-exact, 93.59% vocab use).
- Discovered 3.12 GB of unused, licensed, non-holdout training data in the clean tree;
  64K pilot on it: +0.55% overall (CI-separated) but programming −3.25%, stem −1.55%,
  japanese −1.71%, chinese −1.45%, equal-category −0.37% → rejected.
- Guarded variant (code/STEM ×2 weight): programming +2.91%/stem +2.73% but overall −0.79%;
  peak RSS 6.63 GB > 6 GiB hard ceiling → stopped, rejected.
- Source-Attributed BPE not implementable on this corpus (no per-repo keys; R≈4–5); parity-aware
  not repeated; SentencePiece docs applied as sampling-only design reference.
- Full 256K expanded run **not authorized by the pilot evidence**; would have risked >6 GiB.
- Reports: `reports/v0.4-v2/{RESEARCH_NOTES,BASELINE_AUDIT,EXPERIMENT_LOG,FINAL_REPORT}.md`.
- Honest status: the 6.0 B/T target is open. Tokenizer-quality work on this track should
  pause; downstream LM results, not B/T, are the next decision point.
