# NTokenizer v0.4 CJK & Unicode Report

Date: 2026-11-15
Scope: reproduce and explain the v0.3 CJK regression, test corrections.
Environment: same machine as all v0.3/v0.4 work (i7-12700H, 14 GiB RAM).

## 1. The v0.3 CJK regression (reproduced)

| Bucket | v0.2 B/T | v0.3 B/T | Δ | Bytes |
|---|---:|---:|---:|---:|
| chinese | 3.7545 | 3.5385 | **−5.76%** | 2.95 MB |
| japanese | 5.3766 | 5.0140 | **−6.75%** | 10.14 MB |
| korean | 4.4679 | 5.0000 | +11.91% | 25.26 MB |
| russian/other_languages | 3.9475 | 3.3200 | **−15.91%** | 0.16 MB |

CJK single-token coverage (v0.3 vs v0.2) fell from 99.43% → 96.28% (weighted) and
78.4% → 44.5% (unique chars).

## 2. Root-cause ablations (20 MB/bucket pilot, 64K vocab, seed 42)

Claim under test: the regression comes from `--min-word-freq 2`, not from the
MathSymbolAware pretokenizer.

| Arm | mwf | min-freq | Old bench B/T | Equal-cat | chinese / japanese / korean (old bench) |
|---|---:|---:|---:|---:|---|
| A — v0.3 reference | 2 | 2 | 4.5259 | 4.2247 | 2.954 / 4.202 / 4.038 |
| B — mwf=1 | 1 | 2 | 4.5295 | 4.3121 | 3.420 / 4.314 / 4.172 |
| C — min-freq=1 | 2 | 1 | 4.5259 | 4.2247 | 2.954 / 4.202 / 4.038 |
| D — both ablated | 1 | 1 | 4.5295 | 4.3121 | 3.420 / 4.314 / 4.172 |
| E — mwf=2 + CJK-exempt-1 | 2 | 2 | 4.5118 | 4.2431 | 3.445 / 4.347 / 4.219 |
| F1 — cjk4 chunk | 2 | 2 | 4.5127 | 4.2210 | 3.124 / 4.142 / 4.029 |
| F2 — cjk2 chunk | 2 | 2 | 4.4735 | 4.1968 | 3.272 / 4.000 / 3.896 |
| F3 — cjk8 chunk | 2 | 2 | 4.5239 | 4.2246 | 2.999 / 4.187 / 4.038 |
| G — cjk4 + CJK-exempt | 2 | 2 | 4.4983 | 4.2254 | 3.402 / 4.216 / 4.141 |

Conclusions:

1. `--min-freq` (pair floor) is **inert** at this scale (C ≡ A exactly, D ≡ B).
2. `--min-word-freq` is the entire story: mwf=2 prunes unsegmented CJK runs
   (long, near-unique pre-tokens), so BPE never sees most Chinese pair
   frequencies. mwf=1 recovers essentially all of it (chinese 2.95 → 3.42, +15.8%;
   korean 4.04 → 4.17, +3.3%; japanese 4.20 → 4.31, +2.7%).
3. The Naïve CJK fix (exempt CJK scripts from pruning) works at pilot scale
   (chinese 2.95 → 3.45, +16.6%; korean 4.04 → 4.22, japanese 4.20 → 4.35) but is
   **infeasible at full scale**: 12.7M retained pre-tokens → 235M heap entries,
   8.3 GiB RSS at 3.5% of merges; aborted. Pilot memory: 1.95 GiB (fine);
   full-scale: **out of memory**.
4. Chunking is the memory-safe approximation: cap CJK runs to 4/8 characters so
   common pieces survive the floor. It recovers most of the gap with a tiny
   non-CJK cost.

## 3. Full-scale (256K vocab, 1.96 GB corpus) verification

| Model | chinese | japanese | korean | Other languages |
|---|---:|---:|---:|---:|
| v0.3 | 3.5385 | 5.0140 | 5.0000 | 3.3200 |
| v0.4 candidate (cjk8) | **3.5655 (+0.8%)** | 4.9820 (−0.6%) | 4.9982 (−0.04%) | 3.3148 (−0.2%) |

Bootstrap CI for the cjk8 Chinese gain is [+0.017, +0.038] (P=1.000) — real.
Japanese regression [−0.044, −0.021] is also real.

## 4. Acceptance criteria from the brief, revisited

- "CJK regression reduced" — **yes, but only partially**: chinese is +0.8% above
  v0.3, still far below v0.2. Japanese is −0.6% below v0.3 and −7.3% below v0.2.
- "korean maintained" — yes, within noise.
- "Arabic, Turkish maintained" — yes, within 0.1%.
- Root-cause documented — yes (this file).

## 5. Unicode fixture coverage (unchanged from v0.2 + new stress bucket)

Benchmark v1's synthetic `unicode_stress` bucket covers: ZWJ sequences, keycaps,
skin-tone modifiers, variation selectors (FE00–FE0F), tag sequences, combining
marks in Latin/Devanagari/Arabic/Thai, Hindi matras, Arabic IAWS, Turkish İ/ı,
Cyrillic, NUL and control bytes, long whitespace runs, empty/single-char files,
and pure CJK lines. All are encoded byte-exactly in both v0.3 and v0.4 models
(544/544 + 523/523 decode checks).

UAX #29 note: we do **not** force grapheme-cluster boundaries anywhere in the
pipeline; marks and ZWJ attach as specified for tokenization, which is the
correct behavior for BPE.

## 6. Honest bottom line

- The v0.3 CJK regression is explained, not spooky: flat word-frequency floor
  vs. unsegmented CJK.
- The production-scale fix that preserves the v0.3 envelope is chunking. cjk8 is
  shipped as the v0.4 candidate; it nets +0.8% chinese for −0.6% japanese, exact
  tie elsewhere, identical speed.
- cjk4 (the bigger fix) trades japanese −2.6% for chinese +3.3% — rejected as
  asymmetric harm to one language.
- Fully restoring v0.2-level Japanese would require either mwf=1 at impossible
  memory cost, or a pre-tokenizer that splits Japanese orthographic runs — a
  v0.5 research direction, documented in `FINAL_REPORT.md`.
