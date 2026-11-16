# NTokenizer v0.4 Test Report

Date: 2026-11-15
Scope: all checks re-run after the v0.4 checkpoint/rebuild; fuzzing remains
explicitly unrun.

## 1. Test inventory

| Suite | Result | Evidence |
|---|---|---|
| `cargo test --release --lib` | **48 passed, 0 failed** (incl. new `test_cjkhan4_chunks_han_only` and `test_cjk_chunking_variants`) | this session |
| `cargo test --release` (all) | **130 passed, 0 failed** (126 from base + 2 CJK-chunk + 2 atomic-save + 0 regressions) | `cargo test --release` after rebuild |
| Python bindings (`python3 tests/test_python_bindings.py`) | **14/14** | this session |
| `ntokenizer validate --model output/v0.4-fail/ntokenizer-0.4-256k.ntok` | all PASS (vocab/merge/special/metadata/roundtrip) | this session |
| `ntokenizer validate --model output/v0.3/ntokenizer-0.3-256k.ntok` | all PASS | this session |
| Decode byte-exactness | 544/544 for v0.1, v0.2, v0.3, v0.4 candidate (archived), v0.4 cjkhan4 arm | benchmark harness JSONs |
| New unit tests for v0.4 | CJK chunking (`test_cjk_chunking_variants`), `test_cjkhan4_chunks_han_only` | `src/encoding/pretokenizer.rs` |
| Migration v1→v3 (v0.4 cjkhan4 probe) | OK | previous session probe; v0.3/v0.4 revalidated |
| Corrupt CRC/length/truncation rejection | 3/3 rejected | earlier in session |
| Atomic save (temp+rename) | PASS (2 tests) | `tests/atomic_save_tests.rs` |
| Unicode/ZWJ/flag/combining/NUL fixtures | byte-exact | unicode_stress bucket, 544/544 files |
| Model integrity re-verification | v0.1/v0.2/v0.3 SHA-256 match reference values | this session |

## 2. Not run

- **Fuzzing.** `fuzz/fuzz_targets/*` fail to link on this host (pre-existing
  `-fsanitize=address`/linker incompatibility with this rustc). Not retried.
  Property tests in `tests/roundtrip_property_tests.rs` partially overlap.

## 3. New-v0.4 specific test coverage

- `test_cjk_chunking_variants` — Han/Hiragana/Katakana/Hangul runs chunk to the
  configured cap; Latin/code untouched.
- `test_cjkhan4_chunks_han_only` — Han ideographs chunk to ≤4; pure kana runs
  stay whole (the exact behavior the new arm relies on).
- Atomic-save fault-injection tests — target replaced by a nonempty directory
  (rename fails): original intact, zero `*.ntok.tmp-*` leaks.

## 4. Validation of release artifacts (if any were chosen)

No new release model was chosen this cycle (see FINAL_REPORT.md). v0.3 is the
release artifact; its validation is re-confirmed above.
