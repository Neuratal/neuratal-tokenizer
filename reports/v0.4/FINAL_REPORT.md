# NTokenizer v0.4 — Final Report (clean rebuild)

Date: 2026-11-15
Status: **v0.4 (this attempt) does NOT replace v0.3. v0.3 stays the default.** The
candidate arm is a documented, measured result that fails the task's own release
criteria on two of four core metrics.

## 1. How the previous v0.4 was archived

All v0.4-fail artifacts moved to `*-fail/` preserving layout; hashes verified unchanged:
`experiments/v0.4` → `experiments/v0.4-fail`, `reports/v0.4` → `reports/v0.4-fail`,
`output/v0.4` → `output/v0.4-fail`. The candidate model
(`output/v0.4-fail/ntokenizer-0.4-256k.ntok`, sha256
`fbb5d2d386071f94993130ee2a196e02990c240307d56996487ee6878590874e`) is preserved
untouched for reference. `data/v0.4/` (sealed benchmark v1) was kept in place.

## 2. How the v0.3 checkpoint is preserved

`output/v0.3/ntokenizer-0.3-256k.ntok` — sha256 `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` —
untouched. A full source snapshot was taken before this session (`/tmp/opencode/v04_source_backup_2026-11-15.tar.gz`,
sha256 `6a45fc5b936a4037c934beaf384521b9eb2530cba0782f4e3e3c03d5a9a3ef50`). v0.3 behavior
re-verified from the current (additively changed) source: **37,397,108 tokens,
5.5543 / 5.1057 B/T** — exact match with v0.3's recorded results.

## 3. Code changes this session (additive only)

- `src/encoding/pretokenizer.rs`: one general-category lookup per char (was two)
  + iterator-based run loop without the `Vec<(usize,char)>` allocation; new arm
  `MathSymbolAwareCjkHan4` (Han-only 4-char chunking, kana/Hangul stay whole)
  + `test_cjkhan4_chunks_han_only` unit test. Pretokenizer hot path −15% on a
  3 MB Arabic file; token streams byte-identical on the full benchmark.
- `src/main.rs`: `cjkhan4` CLI alias; error/help text extended.
- New atomic-save fault-injection tests (`tests/atomic_save_tests.rs`) — added earlier
  this cycle, kept.
- No NTOK format change (still v3); no dependency added; no fuzz retried.

## 4. Japanese performance

| Model | Japanese B/T (continuity benchmark) | Holdout |
|---|---:|---:|
| v0.3 | 5.0140 | 4.963 |
| v0.4 cj8 (v0.4-fail) | 4.9820 (−0.64%) | 4.936 (−0.5%) |
| v0.4 cjkhan4 (this arm) | **4.983 (−0.61%)** | **4.920 (−0.87%)** |

Unrecoverable at this scale: the only full fix (mwf=1 unpruning) is memory-infeasible.

## 5. Chinese performance

| Model | Chinese B/T (old) | Holdout |
|---|---:|---:|
| v0.3 | 3.538 | 3.691 |
| v0.4 cj8 | 3.566 (+0.8%) | 3.727 (+1.0%) |
| **v0.4 cjkhan4** | **3.656 (+3.33%)** | **3.884 (+5.23%)** |

Chinese improved meaningfully; Japanese did not.

## 6. Overall B/T

5.5543 (v0.3) → **5.5529** (cjkhan4) — a tie, −0.03% (statistically indistinguishable).
On the sealed Benchmark v1 holdout: **+0.60%** (4.7555 → 4.7843).

## 7. Equal-category B/T

5.1057 (v0.3) → **5.1081** (cjkhan4, continuity) — **+0.05%**.
On holdout: 5.3705 → 5.3833 (**+0.24%**).

## 8. Encode/decode throughput

| | v0.3 | v0.4 cjkhan4 arm |
|---|---|---|
| Encode (1 thread, same session) | 9.54 MB/s (10.18 across sessions; ±10% machine variance) | 9.27 MB/s |
| Encode (8 threads) | 56.30 MB/s | not measured |
| Decode | 544/544 exact | 544/544 exact |
| Pretokenizer hot path | 117.8 ms/3MB | **100.0 ms/3MB (−15%)** |

Encode speed is at parity within machine noise; a claimed ≥10% system-level
improvement was NOT achieved (pretokenizer is only ~33% of encode time; the model
BPE phase is the rest).

## 9. Peak RAM

3.97 GiB for the 256K candidate (target ≤ 4.5 GiB ✓); v0.3 is 3.70 GiB; v0.2 was
≈ 9.7 GB. No swap used.

## 10. Tests passed

Rust lib + integration: **130/130** (129 base + CJK-chunk tests included).
Python bindings: **14/14**. Atomic-save fault injection: **2/2**.
Byte-exact decode: 544/544 on all models. Migration v1→v3: OK.

## 11. Failed experiments (all recorded, none hidden)

- RawByte — 7.5 GiB at 325/32,505 merges; killed.
- CJK exemption (`--min-word-freq-cjk 1`) — 235M heap entries, 8.3 GB at 3.5% of
  merges; killed at full scale.
- cjk2 (Japanese −5%), cjk4 (Japanese −2.6%, encode −7.5%) — rejected.
- cjk8 (Japanese −0.6%) — shipped as v0.4-fail; overall tie with v0.3.
- Parity-aware BPE — Gini −40% but tokens +26%.
- Unigram (SentencePiece) — not byte-exact; worse B/T.
- **cjkhan4 (Han-only 4-char chunk, this arm) — fails two of four release criteria**
  (overall B/T tie-fails, japanese regresses 0.6–0.9%); **not released**.
- min-freq ablation — inert (C ≡ A exactly).
- Benchmark-v1 "carved" corpus — showed other_languages collapses when its only
  source data is removed (−28.8%); used only for arm selection, never shipped.

## 12. Should v0.4 replace v0.3?

**No.** Per the task's acceptance table (overall B/T, equal-category B/T,
japanese, chinese), two core criteria fail (overall B/T is a tie against v0.3,
and japanese is ~0.6–0.9% below it). The candidate is kept as an experiment in
`experiments/v0.4-fail/models/` only — nothing in `output/v0.4/`. **v0.3 remains
the default model and the recommended release.**

If the user wants Chinese-favored deployment, the cjkhan4 arm is measurably better
on chinese/korean (+3.3/+0.8%) with japanese −0.6/−0.9%; present both numbers to
them and let them choose explicitly. Do not call it a win for Japanese.

## 13. Is the tokenizer ready for the 50M LLM training?

**Yes, as a component** — v0.3's model is stable, byte-exact, leakage-audited, and
fits the machine easily. For the 50M run:

- Use **32K** vocabulary (tied embeddings; 33.6% of the 50M parameter budget —
  see `reports/v0.4-fail/VOCABULARY_REPORT.md`), exported from the v0.3 master.
- Keep Benchmark v1 **sealed**; evaluate the 50M model on it without tuning.
- Do **not** use v0.4 candidate exports for the 50M run (they change merge order
  and the recommendation is tied to the corrected v0.3 master).
- The CJK gap (japanese −6.75% vs v0.2) is not resolved; it is understood,
  bounded, and documented.

## 14. Recommendation

Keep v0.3 as the default and only release artifact. Freeze v0.4-fail as the
reference for the next attempt. Fix the pretokenizer classifier hot path (the
measured 15% win proves the headroom) and, for Japanese, pursue either
`mwf=1` on Latin-only pilot-filtered data or a Japanese morphological
pre-segmentation — a v0.5 research track, not this cycle's.
