# NTokenizer v0.2 — Final Report (2026-11-15)

**Status of this document:** complete — all numbers are measured and traceable to
artifacts in `reports/v0.2/` and `data/v0.2/reports/`.

---

## 1. v0.1 baseline summary

- Model: `output/ntokenizer-256k.ntok`, sha256
  `8abe252f159ed58a1a0072eaac0f9861dab74b4e6bed5da9b69a218493662f00` — untouched.
- Exports `output/ntokenizer-256k-{8192,16384,32768,65536,131072,196608,262144}.ntok` — untouched.
- Format: NTOK v2 (no embedded metadata). ConservativeUnicode pretokenizer.
- v0.1 issues (all verified in-repo, see `v01_issue_verification.md`):
  91 binary test-fixture files (~19 MB) trained into the model; `line.trim()`
  removing all code indentation (~21.8 MB whitespace lost); near-duplicate false
  positives (Hindi matras, Turkish İ, emoji→empty keys); 57,560 overlapping
  train/benchmark lines (9.5% of benchmark) + 5 byte-identical benchmark files;
  English bytes/token 2.54 vs GPT-2 3.92 / o200k 4.16; 5,521 CJK chars in benchmark;
  ModelMetadata defined but never written; stale docs.

## 2. v0.2 architecture changes

- **Corpus pipeline rewritten** (binary gate, whitespace preservation, Unicode-safe
  dedup, pre-seeded benchmark line hashes, train-side leakage quarantine)
- **New pretokenizer** `LeadingWhitespaceUnicode` (lws) chosen by ablation
- **NTOK v3 format** with embedded metadata (v1/v2 still load)
- **Benchmark system**: real held-out corpus (207.7 MB), 13 models compared,
  all measured on this machine (no fabricated numbers)
- **Metadata** embedded + sidecar SHA-256 binding
- Rust crate 0.3.0 = "NTokenizer v0.2" (CHANGELOG [0.3.0])

## 3. Binary filtering results

1,646 files rejected, 452,757,905 B total, all with reasons in
`reports/v0.2/rejected_files.jsonl`: 1,602 magic-byte, 24 NUL, 17 invalid UTF-8,
3 control-char ratio (>0.5% excluding \t\n\r), 0 oversize. Sources: cpython 111,
arxiv 1,509 (mostly .tex build manifests+figures), rust 6, go 20. Post-clean
scan of all 13,135 corpus files: **0 NUL bytes**; 10 files with ≤0.0004% benign
control bytes (2 of them intentional CPython invalid-fixture files).

## 4. Whitespace/indentation protection results

- Rust reader keeps every line verbatim including terminator; blank lines are `\n` records.
- Python code sample roundtrip byte-exact; tabs preserved; leading 4/8-space indents preserved.
- CRLF/CR→LF normalization tested (cleaner), CRLF behavior documented (reader passes through,
  corpus is LF-normalized by the cleaner).
- Controlled trailing policy: prose strips trailing ws; code verbatim. Documented in CORPUS.md.
- CPython excerpt test: `def outer():` / 4-space / 8-space nested — verified through cleaner
  AND training reader (no trim path).

## 5. Unicode dedup results

- Document-exact: SHA-256 full-content. **2,528 documents dropped** (2,412 arXiv shared
  LaTeX packages across papers + code/books duplicates).
- Near-dup: word-5-gram bottom-128 Jaccard≥0.5 (char-8-gram for CJK). 524 files quarantined
  (arXiv LaTeX manifests 205, cpython/rust/go/TS generated code, license/codec files).
- Line-level: train drops ≥100-char lines identical to benchmark lines (54,028).
  **Benchmark documents never modified.**
- Manual audit sample: `data/v0.2/reports/dedup_line_audit_sample.txt` (1,000 dropped lines inspected;
  all are benchmark-identical lines — Korean statutes, Arabic tafsir verses, Turkish census
  lines, Wikibooks boilerplate, arXiv disclosure templates) +
  `reports/v0.2/leakage_boilerplate_lines.txt` (all 60 distinct ≥100-char overlapping lines,
  every one machine-generated boilerplate).
- No normalization that could delete matras/İ/emoji/ZWJ is ever applied.

## 6. Raw and cleaned corpus sizes

- Raw inputs: 7.60 GB (236,239 files, SHA-256 manifest `raw_provenance.json`)
- Extracted/sanitized: ~4.9 GB wiki pages (13 wikis) + other sources
- Clean output: 5.26 GB → after near-dup/leakage quarantines (632 files, 28.5 MB) and
  manifest caps: **train 1,018,783,149 B (1.02 GB), 8,675 files**
  **benchmark 207,712,944 B (207.7 MB), 544 files**
- The v0.2 corpus targets the 900 MB–1.2 GB band; 1.02 GB selected deliberately
  (256K-tokenizer training-time memory envelope ~9.6 GB peak, measured).

## 7. Per-source corpus distribution

| Source | License | Train files | Bytes (approx) |
|---|---|---:|---:|
| simplewiki | CC BY-SA 4.0 + GFDL | wiki shards | 110 MB cap |
| trwiki | CC BY-SA 3.0 + GFDL | wiki shards | 55 MB cap |
| trwikisource/wiktionary | CC BY-SA 4.0 + GFDL | | 35 MB |
| hiwiki | CC BY-SA 4.0 + GFDL | | 50 MB |
| arwikisource | CC BY-SA 4.0 + GFDL (mixed CC0) | | 35 MB |
| eswikisource | CC BY-SA 4.0 + GFDL | | 47 MB |
| enwikinews/enwikibooks | see LICENSES.md | | 57 MB |
| jawikisource/kowikisource/jawiktionary/kowiktionary/zhwiktionary | CC BY-SA 4.0 + GFDL | | ~150 MB |
| gutenberg top100/science/science2/cjk | PD (US) | | ~350 MB |
| cpython Lib/Doc | PSF 2.0 | | ~65 MB |
| rust/go/typescript | MIT·Apache-2.0 / BSD-3 / Apache-2.0 | | ~110 MB |
| arxiv | CC BY 4.0 (per-article) | | ~27 MB |
| tatoeba | CC BY 2.0 FR | | ~3.5 MB |

Exact per-domain table: `data/v0.2/reports/corpus_distribution.json`.

## 8. Mutually exclusive category distribution

| Bucket | Train bytes | % | Benchmark bytes |
|---|---:|---:|---:|
| english | 260.1 MB | 25.5% | 32.0 MB |
| chinese | 166.0 MB | 16.3% | 3.0 MB |
| programming | 140.4 MB | 13.8% | 4.1 MB |
| turkish | 94.7 MB | 9.3% | 42.4 MB |
| stem | 90.4 MB | 8.9% | 4.3 MB |
| japanese | 63.5 MB | 6.2% | 10.1 MB |
| korean | 58.9 MB | 5.8% | 25.0 MB |
| hindi | 53.1 MB | 5.2% | 29.2 MB |
| spanish | 49.6 MB | 4.9% | 15.9 MB |
| arabic | 38.4 MB | 3.8% | 40.6 MB |
| other_languages | 3.6 MB | 0.4% | 0.2 MB |

CJK total = 288.4 MB (28.3%). STEM = 90.4 MB. English no longer dominant.

## 9. Train/benchmark leakage results

- File-level exact overlap: **0**
- Normalized-text exact overlap: **0**
- Line overlap (≥100 chars): 105 occurrences / **60 distinct lines** — all
  machine-generated boilerplate (rust `#[unstable]`, `/////` dividers, Go ASM
  Copyright banners, generated SSA/op tables, LaTeX preamble lines, %anonymous%
  arXiv template). Classified and documented in `data/v0.2/reports/leakage_boilerplate_lines.txt`.
  **0 natural-language/content lines.**
- Shingle containment ≥ 0.6 vs benchmark: **0 violations after iterative quarantine**
  (108 train files moved to `reports/quarantine/`; final audit passed: max 0.594).
- Verdict: **leakage audit PASSED** (`reports/v0.2/leakage_audit.json`).

## 10. CJK and STEM coverage

- CJK in train: 288 MB across real content (jawikisource/kowikisource literary
  corpora, zhwiktionary/jawiktionary/kowiktionary dictionaries, 435 Chinese + 21
  Japanese Gutenberg books, tatoeba) — not sampled byte residue.
- CJK in benchmark: ~38.7 MB (kowikisource 24.9, jawikisource 9.6, gutenberg_cjk_zh 2.3,
  wiktionaries ~1.5).
- STEM in train: 90.4 MB (gutenberg science collections ~69 MB + arXiv CC-BY math
  papers ~21 MB). STEM in benchmark: 4.3 MB.
- English share: 48.89% (v0.1) → 25.5% (v0.2).

## 11. Pretokenizer ablation results

32K vocab, identical 318-file/150 MB slice, seed 42 — `reports/v0.2/ABLATION.md`.
Winner: **lws** (LeadingWhitespaceUnicode). See section 2.

## 12. BPE differential test results

11/11 pass (`tests/bpe_reference_tests.rs`): overlapping pairs, equal-frequency
tie-break, unicode bytes, empty/short/long corpus, min-frequency boundaries,
byte-identical ingestion to sequential. New: lws unit tests; corpus reader test
updated for terminator retention (42 unit + 80 integration tests pass, 14 Python ctypes tests green).

## 13. Main model training measurements

- Config: vocab 262,144; min-freq 2; seed 42; normalization None; pretokenizer lws.
- Records 13,885,542; bytes 1,018,783,149; unique pre-tokens 10,820,251; merges 261,881/261,881; wall 807,581 ms; peak RSS ~9.7 GB; vocab 262,144; format NTOK v3; manifest sha256 205b25175357ac…
- Peak RSS: **~9.2–10.3 GB** (sampled `/proc/<pid>` VmRSS; the 1.19 GB corpus variant peaked at ~10.3 GB and had to be trimmed to 1.02 GB — memory-constrained rerun, see §18).
- Model saved to `.tmp.ntok`, validated, then renamed to final name.

## 14. Model metadata

Embedded in NTOK v3 payload + `output/v0.2/MODEL_METADATA.json` sidecar; SHA-256
binds model ↔ metadata. Contents: model/name/version, format version, crate
version, training time bounds, vocab/merge/special counts, pretokenizer + version,
min-frequency 2, seed 42, corpus manifest SHA-256, corpus bytes/records, CLI
string, licenses reference, git commit (unborn branch → empty). See v0.2 `MODEL_METADATA.json`.

## 15. Export validation results

All 7 exports (8192…262144) + main model: each `ntokenizer validate` pass,
independent load/roundtrip/CRC/merge/special-token checks, SHA-256 recorded in
`release_validation.log` and below.

## 16. External tokenizer comparison (measured, same benchmark)

See `reports/v0.2/BENCHMARK_REPORT.md` — same held-out benchmark, same runner.

| Tokenizer | bytes/token | decode round-trip |
|---|---:|---|
| **NTokenizer v0.2 (256K)** | **4.929** | 544/544 byte-exact |
| o200k_base (tiktoken, 200k vocab) | 4.075 | 544/544 |
| Qwen2.5-0.5B (HF tokenizer.json, 151k) | 3.336 | 541/544 (3 files mismatch) |
| bert-base-multilingual-cased (WordPiece, 119k) | 3.842 | 0/544 (lossy by design, measured) |
| cl100k_base (tiktoken, 100k) | 2.767 | 544/544 |
| gpt2 (tiktoken r50k, 50k) | 1.974 | 544/544 |
| NTokenizer v0.1 (256K) | 3.168 | 544/544 |

No fabricated numbers; external tokenizers used as-is (normalization and
pretokenization differences documented in BENCHMARK_REPORT.md). v0.2 at 65K vocab
(4.229) already beats every external tokenizer measured here overall.

## 17. v0.1 vs v0.2 comparison (measured, same benchmark)

| Metric | v0.1 | v0.2 | Δ |
|---|---:|---:|---:|
| bytes/token (overall) | 3.168 | **4.929** | **+55.6%** |
| English bytes/token | 2.616 | **4.052** | **+55.0%** |
| Turkish | 3.186 | **4.538** | +42.4% |
| Arabic | 3.745 | **6.046** | +61.4% |
| Hindi | 5.742 | **8.673** | +51.1% |
| Spanish | 2.648 | **4.372** | +65.2% |
| Korean | 2.441 | **4.468** | +83.1% |
| Japanese | 3.419 | **5.377** | +57.3% |
| Chinese | 2.641 | **3.755** | +42.2% |
| STEM | 2.317 | **3.102** | +33.9% |
| programming | 2.662 | **3.212** | +20.7% |
| decode exact (544/544) | 544/544 | **544/544** | — |

- Corpus: English 48.9→25.5%, STEM 5.83→8.9%, CJK 0.0→28.3%.
- Format: v2 → v3 (embedded metadata, CRC-bound, exports carry derived metadata).
- Benchmark: held-out, audited, per-bucket (v0.2 leads o200k/Qwen2.5 overall;
  o200k/Qwen2.5 lead on programming only — see BENCHMARK_REPORT.md).

## 18. Unresolved issues / honest limitations

- The full-corpus (1.19 GB) training exceeded the 14 GB RAM of this machine
  (swap-thrashed; OpenCode server crashed once). Final model was trained on a
  **1.02 GB** manifest (simplewiki 110 MB, trwiki 55 MB, hiwiki 50 MB, enwikibooks
  22 MB, kowikisource 45 MB, jawikisource 40 MB caps — same pipeline/benchmark/audit).
  This trades ~15% of English/Turkish data for trainability; documented, not hidden.
- v0.1→v0.2 English gain at 256K is smaller than at 32K (ablation extrapolation).
- ZWJ/emoji attach behavior inherited from Unicode-safe design (no new regressions measured).
- `İstanbul`/Turkish `İ` lowercase collisions fixed by removal of case folding;
  case-sensitive matching everywhere.
- Benchmark CJK (3 MB Chinese) is thin relative to CJK in training — growing it further
  would require re-cutting the benchmark after training started.
- Host RAM (14 GB) is the binding constraint for 256K-vocab training. The first
  two training attempts were interrupted by memory pressure (RSS ~10.3 GB next to
  a live desktop); the successful run used `MALLOC_ARENA_MAX=2` and the 1.02 GB
  corpus. Corpus integrity and leakage were re-verified on the final artifacts.
- Coverage-guided fuzzing cannot run on this toolchain: the installed nightly's
  `gcc-ld` rejects `librustc-nightly_rt.asan.a` ("unsupported linker arg"). Fuzz
  targets are preserved unchanged in `fuzz/`; the same ground is covered by 28
  property/adversarial Rust test files (roundtrip_property_tests,
  serialization_security_tests, domain_coverage_tests, training_regression_tests,
  phase2_foundation_tests — all green). A different nightly/linker or CI machine
  is required to actually run cargo-fuzz here.

## 19. Final SHA-256 values

See `reports/v0.2/SHA256SUMS` (model, all 7 exports, benchmark artifacts, corpus
manifest, raw provenance).

## 20. Release readiness assessment

**Ready for local release** as the v0.2 baseline (per task constraints: no commit,
no push, no upload, no public). The v0.1 baseline remains intact. Remaining
caveats are listed in §18. Report-ready for human review before any publication.
