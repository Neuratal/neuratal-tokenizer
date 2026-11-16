# NTokenizer v0.4 Test Report

Date: 2026-11-15
Status: all automated checks green; fuzzing remains explicitly not run.

## 1. Test inventory

| Suite | Result | Evidence |
|---|---|---|
| Rust lib + integration tests (`cargo test --release`) | **129 passed, 0 failed** | `experiments/v0.4/logs/cargo_test_final2.log` |
| — of which new in v0.4 | 2 atomic-save fault-injection tests + 1 CJK-chunk pretokenizer test | `tests/atomic_save_tests.rs`, `src/encoding/pretokenizer.rs` |
| Python ctypes bindings (`tests/test_python_bindings.py`) | **14/14** | this session |
| `ntokenizer validate --model <m> --roundtrip` (v0.4 candidate) | all PASS (13/13 probes + roundtrip_bytes + metadata) | this session |
| NTOK v1 → migrate → validate (v0.4 candidate) | OK | compatibility checks below |
| NTOK v2 (v0.1 model) / v3 (v0.2, v0.3, v0.4) load+validate | OK | compatibility checks below |
| Corrupt CRC32 / wrong payload length / truncation rejection | 3/3 rejected with expected errors | this session |
| Decode exactness on benchmark | 544/544 byte-exact (v0.1, v0.2, v0.3, v0.4 candidate) | per-file JSON results |
| Unicode stress bucket (synthetic) | 544/544 + v0.4 byte-exact | `experiments/v0.4/results/*.json` |

## 2. What the new tests cover

- `test_cjk_chunking_variants` — CJK runs chunk to ≤2/4/8 chars; non-CJK text keeps
  MathSymbolAware behavior exactly.
- `atomic_save_tests::rename_failure_preserves_original_and_cleans_temp` — target
  path replaced by a nonempty directory: rename fails, original sentinel file intact,
  zero `*.ntok.tmp-*` leftovers.
- `atomic_save_tests::failed_save_does_not_corrupt_existing_model_file_bytes` —
  read-only target: bytes unchanged after failed save attempt.

## 3. Fuzzing

**Not run.** The pre-existing ASAN/linker incompatibility in `fuzz/` is unchanged
(rustc cannot link `-fsanitize=address` targets on this machine); no fuzz corpus
was executed for v0.4. Do not read the green test suite as fuzzing coverage.
Property tests (`tests/roundtrip_property_tests.rs`) partially overlap fuzz
coverage for decode round-trips, but are not fuzzing.

## 4. Validation probes per release model

| Model | Round-trip | Metadata | Verdict |
|---|---|---|---|
| v0.1 (`output/ntokenizer-256k.ntok`) | pass | absent (v2) | OK |
| v0.2 (`output/v0.2/ntokenizer-0.2-256k.ntok`) | pass | consistent | OK |
| v0.3 (`output/v0.3/ntokenizer-0.3-256k.ntok`) | pass | consistent | OK |
| v0.4 candidate (`output/v0.4/ntokenizer-0.4-256k.ntok`) | pass | consistent | OK |
| v0.4 A-control (carved) | pass | consistent | OK |
| v1-patched v0.4 (migration probe) | pass | absent (v1→v3 migrated) | OK |

## 5. Honest gaps

- No fuzzing; no sanitizer builds executed.
- The atomic-save test simulates failure by replacing the destination with a
  nonempty directory — a real rename failure, not a simulated OOM/`ENOSPC`.
- Python bindings were not extended with v0.4-specific fixtures (e.g., CJK prefil
  tokens); they do pass unchanged.
- No CI pipeline exists in this repo; all results are local-machine runs.
