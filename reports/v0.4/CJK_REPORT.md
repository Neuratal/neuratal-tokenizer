# NTokenizer v0.4 — CJK and Japanese Report

Date: 2026-11-15
Scope: CJK performance of v0.3 (baseline) and the new v0.4 candidate arms.
Machine: i7-12700H, 14 GiB RAM, rustc 1.99.0.

## 1. Root cause (confirmed in the previous attempt, re-verified)

Flat `--min-word-freq 2` prunes every pre-token whose corpus frequency < 2 before
the pair index is built. Unsegmented CJK runs (sentences of 10–30 ideographs with
no spaces) are near-unique, so **essentially all Chinese text contributes zero
pairs** to the merge table. Ablations that remove the floor (`mwf=1`) recover the
loss, but the pair-index explodes (12.7M pre-tokens, 235M heap entries, 8.3 GiB at
3.5% of merges — killed). The production-scale fix must therefore keep the floor
and instead *shorten* the unsegmented runs: CJK chunking.

| Arm | chinese (old bench) | japanese | korean | Verdict |
|---|---:|---:|---:|---|
| A — v0.3 (math, mwf=2) | 2.954 | 4.202 | 4.038 | reference |
| B — math mwf=1 | 3.420 (+15.8%) | 4.314 | 4.172 | infeasible at scale |
| E — math mwf2 + CJK-exempt | 3.445 (+16.6%) | 4.347 | 4.219 | infeasible at scale |
| F1 — cjk4 (Han+kana+Hangul ≤4) | 3.124 (+5.7%) | 4.142 (−1.4%) | 4.029 | japanese −2.6% at 256K; rejected |
| F2 — cjk2 | 3.272 (+10.7%) | 4.000 (−4.8%) | 3.896 | japanese collapse — rejected |
| F3 — cjk8 | 2.999 (+1.5%) | 4.187 | 4.038 | v0.4-fail candidate (shipped as "cjk8", overall tie, japanese −0.6%) |
| **H — cjkhan4 (Han-only ≤4)** | **3.124 (+5.7%)** | **4.203 (±0)** | **4.078 (+1.0%)** | **full-scale candidate — abandoned (fails release bar)** |

## 2. Full-scale 256K results (our new candidate)

Training: full v0.3 corpus (1.963 GB, 20,041,192 records, seed 42, mwf=2),
7 min 03 s, peak RSS **3.97 GiB**.

| Bucket | v0.3 | cjkhan4 | Δ |
|---|---:|---:|---:|
| chinese | 3.538 | **3.656** | **+3.33%** |
| korean | 5.000 | **5.040** | **+0.79%** |
| japanese | 5.014 | 4.983 | **−0.61%** |
| arabic | 7.198 | 7.182 | −0.23% |
| english | 4.463 | 4.455 | −0.17% |
| hindi | 9.928 | 9.907 | −0.21% |
| other_languages | 3.320 | 3.303 | −0.50% |
| programming | 4.096 | 4.089 | −0.18% |
| stem | 3.564 | 3.556 | −0.22% |
| spanish | 4.736 | 4.726 | −0.21% |
| turkish | 5.305 | 5.292 | −0.24% |
| **Overall B/T** | **5.5543** | **5.5529** | **−0.03% (tie)** |
| Equal-category B/T | 5.1057 | **5.1081** | **+0.05%** |
| Tokens | 37,397,108 | 37,406,269 | +0.02% |
| Used vocab | 245,334 (93.59%) | 245,543 (93.67%) | — |
| Encode 1 thread | 10.18* MB/s | 9.27* MB/s | −9% |
| Decode exact | 544/544 | 544/544 | ✓ |
| Peak inference RSS | 1,119 MiB | 1,099 MiB | −20 MiB |

(*throughput varies ±10% between sessions; same-session comparison shows parity.)

On the sealed Benchmark v1 holdout (unseen-by-all-NTokenizer-models files):
overall **+0.60%** (4.7555→4.7843), equal-category **+0.24%** (5.3705→5.3833),
chinese **+5.23%**, korean **+2.58%**, japanese **−0.87%**.

## 3. Acceptance vs the v0.3 baseline (the task's own table)

| Criterion | v0.3 → cjkhan4 | Pass? |
|---|---|---|
| Overall B/T higher | 5.5543 → 5.5529 | **FAIL** (tie) |
| Equal-category not lower | 5.1057 → 5.1081 | PASS |
| Japanese not worse | 5.014 → 4.983 | **FAIL** (−0.61%) |
| Chinese not worse | 3.538 → 3.656 | PASS (+3.33%) |
| Turkish preserved | 5.305 → 5.292 | PASS (within 0.3%) |
| Arabic preserved | 7.198 → 7.182 | PASS (within 0.3%) |
| Programming not significantly worse | 4.096 → 4.089 | PASS |
| Encoding ≥ v0.3 | 10.18* → 9.27* MB/s | TIE/noise (same-session parity) |
| Byte-exact decode | 544/544 | PASS |
| Peak RAM (target ≤4.5 GiB) | 3.97 GiB | PASS |

**Two failures → do not release.** The candidate is kept as an experiment in
`experiments/v0.4-fail/models/` and is *not* in `output/v0.4/`.

## 4. Why Japanese unfixable at this scale

The only arms that recover Japanese fully (mwf=1 variants) are memory-infeasible
(>12 GB pair index). The CJK-chunk arms protect Chinese/Korean but always
sacrifice ≥0.6% Japanese because chunking 4-char Han pieces destroys the long
kana/kanji compounds BPE would otherwise learn (and kana interleaves with Han,
so Han-only restriction does not fully protect it). The root-cause fix in a future
version: per-script merge priorities or Japanese morphological pre-segmentation
(open research, not in this timebox).

## 5. Bottom line

- v0.3's Japanese (−6.75% vs v0.2) is NOT recovered by any affordable arm.
- The production CJK arms recover Chinese (+3–5%) but shift Japanese by −0.6/−0.9%.
- Per the task's acceptance table, no CJK arm beats v0.3 on the required criteria;
  v0.3 remains the recommended default; cjkhan4 documented as an experimental arm.
