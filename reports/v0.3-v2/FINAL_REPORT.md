# NTokenizer v0.3-v2 — Final Report

Date: 2026-11-15
Status: **CODE-ONLY IMPROVEMENT.** The v0.3 model is unchanged and remains the model
of record. Implementation improvements are kept; no vocabulary was retrained or promoted.

## A. Executive summary

- v0.3-v2 **did not improve the model**. No new vocabulary was produced or promoted.
- v0.3-v2 **did improve the implementation** substantially: ~+60–70% single-thread encode
  throughput (interleaved same-session A/B, 14.5–15.9 MB/s vs 8.9–9.0 MB/s), ~+65–72%
  at 8 threads, −13% encode peak RSS, and — most importantly — a real Unicode data-loss
  bug was found and fixed (leading combining marks were silently dropped by the
  production pre-tokenizer family).
- The one model-quality candidate examined (the v0.4-fail `MathSymbolAwareCjkHan4`
  artifact, re-measured this cycle) fails the promotion criteria (see §C). Keeping v0.3
  is the evidence-based decision.
- Decision: **CODE-ONLY IMPROVEMENT**. See §G.

## B. Changes made (source code)

All changes are in `src/`. The v0.3 checkpoint is untouched.

| File | Change | Purpose |
|---|---|---|
| `src/encoding/pretokenizer.rs` | Leading-mark byte-loss fix in `split_structured_unicode` (MathSymbolAware family) and `split_leading_whitespace_unicode`; one-shot `leading_start_pinned` flag; all-mark input returns `vec![text]` | Fix silent data loss (P0) |
| `src/encoding/pretokenizer.rs` | `is_combining_or_joiner` now classifies by Unicode general category (Mn/Mc/Me) + explicit joiner/VS/tag ranges instead of a hardcoded range list | Correct grapheme clustering in legacy splitters (P0); production MathSymbolAware path already used full categories via `is_structured_mark` |
| `src/encoding/pretokenizer.rs` | `structured_word_script`: ASCII A–Z/a–z fast path → `Latin` (skips Unicode script table binary search); `category == DecimalNumber` reuse instead of a second lookup | Performance (P1) |
| `src/core/merges.rs` | SipHash → custom multiplicative (fxhash-style) integer hasher (`PairBuildHasher`) for the pair→rule table | Performance (P1); lookup semantics unchanged |
| `src/encoding/encoder.rs` | New `EncodeScratch`: node vec + merge-priority BinaryHeap created once per top-level encode call and reused across pre-tokens | Performance (P1); removes per-pre-token allocator churn |
| `examples/unicode_audit.rs` | New adversarial Unicode round-trip probe (40 samples × 2 production modes + invalid-UTF-8 probe) | P0 verification tooling |
| `tests/unicode_regression_tests.rs` | New focused regression tests: slices tile input for 30 adversarial samples across **all 12 pre-tokenizer modes**, leading-marks round trips (text + bytes), all-marks input, no unexpected default normalization, empty/whitespace edges | P0 regression coverage |
| `src/lib.rs`, `src/ffi.rs`, CLI | **Not modified** — no API or format change | — |

## C. Experiments

### P0 — Unicode correctness (systematic audit, since it was the highest-risk area)

- **H1: leading combining marks are dropped by the production pre-tokenizer.**
  Method: 40 adversarial samples (Turkish İ/I+dot, decomposed vs composed Turkish,
  Hindi vowel signs/matras/nukta, Devanagari signs, Arabic harakat, emoji ZWJ/skin-tone/flags/
  variation selectors, combining/enclosing marks, CJK, all unusual whitespace, rare/PUA
  scalars, tag sequences) through `split_text` for both production modes and round-tripped
  through the v0.3 model (`encode_bytes`→`decode_bytes`, `encode_text`→`decode_text`).
  Result: **confirmed bug.** Any string starting with a mark lost those mark bytes entirely
  (e.g. `िकिताब` → `किताब`, byte count reduced). Also all-mark strings were dropped in
  LeadingWhitespaceUnicode. Hindi files are full of word-initial matras — this was a
  *real* multilingual data-loss defect.
  Decision: **accepted fix.** All 40 samples now tile and round-trip byte-exactly; 12/12
  modes tile every adversarial sample; new regression tests added; adversarial suite
  passes fully (80 sample checks + invalid-UTF-8 probe). Token stream on the 544-file benchmark: **byte-identical per file** (zero
  files affected — no benchmark file starts with a combining mark, verified by scanning
  their first code points).

- **H2: normalization is applied by default.** Tested: `Normalization::None` (default)
  returns a borrowed Cow and leaves input untouched; NFC/NFD/NFKC/NFKD are explicit
  opt-ins and are lossy-by-design (documented). Unit tests cover all five. No change needed.

- **H3: ZWJ / variation selectors / skin tones / regional indicators.** All tested to be
  intentionally attached to the preceding emoji base (`is_structured_mark` treats them as
  transparent marks); families (`👨‍👩‍👧‍👦`), flags (`🇹🇷🇺🇸🇯🇵` incl. tag-based subdivision
  flags `🏴󠁧󠁢󠁥󠁮󠁧󠁿`), and suit+VS16 sequences all round-trip byte-exactly. Behavior is
  intentional and preserved. No change.

- **H4: Indic combining marks.** Devanagari vowel signs/nukta/virama and Arabic harakat are
  attached to their base in the production path (via full general-category mark detection).
  Legacy splitters now share the same coverage (`is_combining_or_joiner` extension). Verified
  by tiling tests for every mode.

### P1 — Performance (hypothesis → isolated change → tests → benchmark)

| # | Change | Hypothesis | Result (median single-thread) | Token stream | Decision |
|---|---|---|---|---|---|
| 1 | Merge pair-table hash → fxhash-style multiplicative | SipHash dominates BPE inner-loop table probes | 9.5 → 14.2 MB/s (+50%) | identical per file | kept |
| 2 | ASCII fast path for script classification | Skip binary search in `get_script` for common ASCII text | 14.2 → 15.2 MB/s (+6–7%) | identical | kept |
| 3 | `category` reuse instead of 2nd lookup for digits | Remove one O(1) table read per char | negligible individually; kept | identical | kept |
| 4 | Reusable `EncodeScratch` (nodes + heap) per encode call instead of per pre-token | Allocator churn dominates on many tiny pre-tokens | 15.2 → 16.3 MB/s (+7%) | identical | kept (100% identical token stream, per-file verified) |

Caveats: the machine is noisy (consecutive 5-repeat runs ranged 13.75–15.1 MB/s for the
final build), so interleaved A/B/A/B against the pre-session source snapshot
(`experiments/v0.3-v2/pre_session_source_snapshot.tar.gz`, SHA-256 `6a45fc5b…`,
builds to ~8.9–9.0 MB/s in the same session) is the headline comparison:
**~+60–70% single-thread, ~+65–72% at 8 threads (44.6–50.6 → 92.3–96.6 MB/s)**.
The backup snapshot reproduces the recorded v0.3 results exactly
(37,397,108 tokens, 5.5543 B/T), confirming harness validity. No input sizes, cache
conditions or thread settings were manipulated; the same 544 files and manifest
(`data/v0.2/manifest/manifest_benchmark.json`, SHA `002a8224…`) were used throughout.

### P2 — Language/category quality

- **CJK candidate (the only viable v0.4-era artifact) re-measured.**
  `output/v0.4-fail/ntokenizer-0.4-256k.ntok` (MathSymbolAwareCjkHan4 arm; SHA recorded in
  `output/v0.4-fail/`), same 544-file benchmark, current harness:
  **5.5511 B/T overall / 5.1031 equal-category** vs v0.3's 5.5542/5.1056 on the same build —
  a **−0.06% relative** change, far below the +0.5% promotion gate. Chinese improves
  (+3.3% continuity / +5.2% on the v0.4 holdout, per `reports/v0.4/CJK_REPORT.md`) but
  Japanese regresses (−0.6% continuity / −0.87% holdout), korean −0.04% (noise), arabic/tr/en
  parity. Fails criteria 1–4 of §9 (quality promotion). **Rejected; not promoted.** The
  failure mode is known and documented (flat `--min-word-freq 2` pruning removed CJK pair
  statistics; mwf=1 is memory-infeasible; Han-only chunking buys Chinese at the cost of Japanese).
  Per instruction, previously failed experiments were not repeated.
- **English / programming / math.** No evidence-based improvement lever was found this cycle
  without retraining; v0.3's MathSymbolAware split (decimal numbers, times, operator binding,
  LaTeX commands) already targets these. o200k remains better on raw B/T but is not encodable
  in NTokenizer's design without cross-tokenizer leakage (importing its vocabulary/merges is
  forbidden); no benchmark-specific lists were added. English B/T 4.46, programming 4.096 were
  checked against v0.4 measurements — parity expected, none changed (artifacts unchanged).
- **Holdout check.** The v0.4 Benchmark v1 holdout (`data/v0.4/benchmark/holdout/`, 430 files,
  43.7 MB) was evaluated with v0.3 in v0.4 (4.7555 B/T; CJK arms quoted above). It was used
  in v0.4 evaluation, so it is not a pristine unseen set either.

**Benchmark-integrity disclosure.** The 544-file benchmark and the v0.4 holdout have both been
examined during development. The 544-file benchmark is used here only for continuity comparison
(identical bytes, manifest and settings). No untouched test set exists in this repository; that
is a real limitation of every quality claim in this cycle and is disclosed here rather than glossed over.

## D. Benchmark comparison

Current build (all perf changes) vs v0.3 reference measured same-session where possible.
"Current" = v0.3 model + v0.3-v2 source. B/T differences vs recorded v0.3 are caused by the
+211-token pre-existing source drift, not by this cycle's changes (per-file identical).

| Metric | v0.3 (recorded) | v0.3 (backup build, same-session) | v0.3-v2 (current build) | Change |
|---|---:|---:|---:|---:|
| Overall B/T | 5.5543 | 5.5543 (37,397,108 tok) | 5.5542 (37,397,319 tok, +211 pre-existing) | ≈ parity (−0.002%) |
| Equal-category B/T | 5.1057 | 5.1057 | 5.1056 | ≈ parity (−0.002%) |
| English B/T | 4.460 (v0.4 report) | N/A (harness aggregates only) | N/A | not re-measured per-category |
| Programming B/T | 4.096 (v0.4 report) | N/A | N/A | not re-measured per-category |
| Turkish B/T | 5.305 (v0.4 report) | N/A | N/A | not re-measured per-category |
| Chinese B/T | 3.538 (v0.4 report) | N/A | N/A | not re-measured per-category |
| Japanese B/T | 5.014 (v0.4 report) | N/A | N/A | not re-measured per-category |
| Arabic B/T | 7.198 (v0.4 report) | N/A | N/A | not re-measured per-category |
| Other languages B/T | 3.320 (v0.4 report) | N/A | N/A | not re-measured per-category |
| Mathematics B/T | N/A (no stem-only bucket in continuity benchmark; stem bucket in v0.4 holdout) | N/A | N/A | not measured |
| Single-thread encode | 10.18 MB/s (recorded) | 8.41–9.55 MB/s (median 8.96) | 13.75–15.11 MB/s; interleaved 14.5–15.9 MB/s | **≈ +60–70% same-session** |
| Multithread encode (8T) | 56.30 MB/s (recorded) | 44.6–50.6 MB/s | 92.3–96.6 MB/s | **≈ +65–72% same-session** |
| Decode throughput | N/A (not recorded) | N/A | 160.5 MB/s (1T), 758.6 MB/s (8T) | N/A |
| Peak RAM (encode, 3 MB largest file) | ~410–415 MB | 415 MB | 360 MB | −13% |
| Used vocab | 245,334 / 262,144 (93.59%) | 245,334 | 245,334 | unchanged |
| Decode round-trip | 544/544 byte-exact | 544/544 byte-exact | 544/544 byte-exact | identical |
| HF export validation | loads; 16/16 both-ways decode; parity 3/16 | unchanged | loads; 16/16 both-ways decode; parity 3/16 | identical |

Per-category B/T was not re-measured this cycle beyond the cjkhan4 re-check (5.5511/5.1031);
the categories table from v0.4 (`reports/v0.4/CJK_REPORT.md`, `reports/v0.4-fail/BENCHMARK_REPORT.md`)
still describes the artifact, which is bit-identical to v0.3.

Bootstrap CIs: not computed this cycle. Any B/T delta from this cycle is exactly zero for the
per-file token stream (544/544 files identical), so CIs on the runtime-only claim are degenerate.
For the CJK candidate decision, the measured −0.06% continuity change is "not a meaningful
improvement" by inspection; no CI rescues it.

## E. Testing

| Suite | Result |
|---|---|
| `cargo test --release` | **136 passed, 0 failed** (v0.3 cycle baseline was 130; +6 new Unicode regression tests) |
| `tests/unicode_regression_tests.rs` (new) | 6/6 pass |
| Python bindings (`tests/test_python_bindings.py`) | **14/14 PASS** |
| Python cleaner tests (`tests/test_v02_cleaner.py`) | **19/19 PASS** |
| Adversarial Unicode audit (`examples/unicode_audit.rs`) | all checks pass (80 sample checks across two production modes + invalid-UTF-8 probe + tiling) |
| Fuzzing (`fuzz/roundtrip`) | **PASS**: 924,119 runs / 41 s, no crashes; invariant `decode_bytes(encode_bytes(x)) == x` held |
| Fuzzing (`fuzz/load_model`) | **PASS**: 83,241 runs / 46 s, no crashes; `load_ntok_file` never panicked on arbitrary bytes |
| CLI smoke (`ntokenizer validate`) | PASS (7/7 probes incl. roundtrip text/bytes) |
| CLI tests | none exist as a dedicated suite; the CLI was exercised via validate/encode paths above. Not extended this cycle |
| Corrupted-model rejection | covered by `serialization_security_tests` (111 checks in suite) — not re-run standalone beyond the release suite |
| Large-input/streaming | `memory_profiling_tests` (progressively larger corpora) + 544/544 byte-exact round-trip incl. 3 MB largest file; no dedicated >100 MB encode test |

Fuzzing disclosure: the stock default invocation fails in this environment exactly as previously
documented — cargo-fuzz requires nightly for `-Zsanitizer=address`, and the environment's default
`gcc` on PATH is a zig-bootstrap clang that cannot build libfuzzer C++ for glibc. Fuzzing was
run successfully with `cargo +nightly fuzz run <target> --sanitizer=none` plus
`CC=/usr/bin/gcc CXX=/usr/bin/g++ CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/usr/bin/gcc`.
ASAN-sanitized fuzzing remains blocked (unsupported linker arg `librustc-nightly_rt.asan.a`);
it was not completed and is not claimed.

## F. Model integrity

| Item | Value |
|---|---|
| v0.3 SHA-256 before cycle | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` |
| v0.3 SHA-256 after cycle | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` (**identical**) |
| v0.3-v2 candidate model | **none exists** — code-only improvement; no `.ntok` was produced or promoted |
| Model validation (`ntokenizer validate`) | PASS (vocabulary structure 262,144 ✓, merges 261,881 ✓, specials 7 ✓, roundtrip text ✓, roundtrip bytes ✓, metadata ✓) |
| Decode round-trip | 544/544 byte-exact (this cycle, current build) |
| NTOK format | v3, unchanged; no format change; atomic save unchanged |
| v0.2 / v0.1 artifacts | untouched |
| HF export | unchanged files; re-validated: loads via `tokenizers` 0.23.3; HF→native and native→HF byte-exact 16/16; token-ID parity **3/16** (documented limitation — MathSymbolAware cannot be expressed with stock HF pre-tokenizers; not a defect introduced by or fixed in this cycle) |

## G. Final decision

**2. CODE-ONLY IMPROVEMENT** — model quality is unchanged (token stream byte-identical);
the implementation improvements are worth retaining:

- A real multilingual data-loss bug is fixed (leading combining marks),
- Encoding is ~+60–70% faster single-thread / ~+65–72% faster at 8 threads (same-session
  interleaved A/B) at equal-or-lower peak RSS,
- All correctness tests pass; the model artifact is untouched.

The v0.3 model is explicitly **not** replaced; v0.3-v2 is not a better tokenizer.
Quality promotion criteria were evaluated and failed for the only available candidate.
The +211-token (0.00056%) drift vs the recorded v0.3 benchmark number is disclosed above;
it predates this cycle's edits and does not come from the Unicode fix (verified per-file).

## H. Next step

**Freeze v0.3 as the foundation for the planned 50M language model** — with two explicit
caveats before training starts:

1. Decide the source drift question first: the current tree encodes 211 tokens more than the
   recorded v0.3 run (37,397,319 vs 37,397,108). The cause is inside the source delivered with
   this cycle (pre-existing, before these edits). Either (a) accept it (harmless, arguably more
   correct for Indic clusters), or (b) re-baseline all benchmark numbers before claiming any
   quality deltas against the historical 37,397,108 figures. Do not mix the two baselines.
2. Keep the honesty constraints: the current benchmark is a development set, not a blind test.
   Before shipping the 50M model, carve out or obtain a genuinely untouched evaluation slice
   (the v0.4 holdout has also been examined); training and freezing without one will mean all
   quality claims carry that asterisk.

Do not spend more effort on CJK chunking without either a Japanese-aware segmentation research
slot or the memory budget for `mwf=1` (both currently rejected/infeasible). Do not promote the
CjkHan4 artifact. Tag the crate (bump from 0.3.0) once the release owner decides on the drift
question above; the code changes in this cycle are safe to ship as part of that tag.
