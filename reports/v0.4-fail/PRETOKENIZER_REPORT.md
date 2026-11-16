# NTokenizer v0.4 Pretokenizer Report

Date: 2026-11-15
Scope: the new arms evaluated in v0.4. LWS / UnicodeHybrid / CodeAware /
MathSymbolAware suites are covered in `reports/v0.3/PRETOKENIZER_REPORT.md`;
this file covers only the v0.4 deltas. All arms: 20 MB/bucket pilot, seed 42,
64K vocab, `mwf=2` unless noted; the 544-file benchmark (continuity set) is
used for the headline columns.

## Arms

| Arm | Mode | Description |
|---|---|---|
| A | `math` | MathSymbolAware — v0.3 winner, reference |
| B | `math` with `mwf=1` | frequency-floor ablation |
| C | `math --min-freq 1` | pair-frequency ablation |
| D | `math --min-freq 1 --min-word-freq 1` | both floors ablated |
| E | `math --min-word-freq-cjk 1` | CJK-script exhaustion-floor exception |
| F1 | `cjk4` | MathSymbolAware + CJK runs chunked to ≤4 chars |
| F2 | `cjk2` | same at ≤2 chars |
| F3 | `cjk8` | same at ≤8 chars |
| G | `cjk4 --min-word-freq-cjk 1` | chunk + CJK exemption hybrid |

## Pilot results (64K vocab, continuity benchmark / new holdout)

| Arm | Bytes/T | Equal-cat | holdout B/T | holdout equal | CJK read-out | Notes |
|---|---:|---:|---:|---:|---|---|
| A (math, ref) | 4.5259 | 4.2247 | 3.9582 | 4.1161 | chinese 3.014, korean 3.137 | baseline |
| B (mwf=1) | 4.5295 | 4.3121 | 4.1520 | 4.2501 | chinese 3.652, korean 3.444 | mostly small-bucket gains |
| C (min-freq 1) | 4.5259 | 4.2247 | — | — | identical to A | **min-freq inert at this scale** |
| D (both 1) | 4.5295 | 4.3121 | — | — | identical to B | — |
| E (cjk-exempt) | 4.5118 | 4.2431 | 4.0559 | 4.1591 | chinese 3.688, korean 3.481 | pair index explodes at full scale; aborted |
| F1 (cjk4) | 4.5127 | 4.2210 | 3.9878 | 4.1260 | chinese 3.254 (+8.0% vs A) | japanese −1.7% heldout |
| F2 (cjk2) | 4.4735 | 4.1968 | 4.0055 | 4.1305 | chinese 3.513 (+16.6%) | **japanese −5.1% — rejected** |
| F3 (cjk8) | 4.5239 | 4.2246 | 3.9650 | 4.1184 | chinese 3.065 (+1.7%) | safe, small gain |
| G (cjk4+exempt) | 4.4983 | 4.2254 | 4.0359 | 4.1445 | chinese 3.622 (+20.2%), korean 3.442 (+9.7%) | others −1.4–1.8% |

## Full-scale 256K results (1.96 GB corpus)

| Model | Old bench B/T | Equal-cat | CJK bucket | Verdict |
|---|---|---|---|---|
| v0.3 (math) | 5.5543 | 5.1057 | chinese 3.538 | baseline |
| **v0.4 candidate (cjk8)** | 5.5511 | 5.1032 | chinese 3.566 (+0.8%) | parity B/T; optional |
| v0.4 cjk4 (rejected) | 5.5267 | 5.0860 | chinese 3.654 (+3.3%) | japanese −2.6%, encode −7.5% |
| v0.4 A control (carved corpus) | 5.5577 | 5.0224 | chinese 3.534 | carved-corpus control |

## Findings

1. **Root cause confirmed**: flat `--min-word-freq 2` prunes unsegmented CJK runs
   (long, near-unique pre-tokens), starving BPE of Chinese pair frequencies.
   Ablation E recovered chinese +20–22% and korean +10–11%, but the pair index
   exploded (235M entries, 8.3 GiB RSS at 3.5% of merges) — infeasible at scale.
2. **CJK chunking is the production-scale fix.** Capping CJK runs at ≤4/8 chars
   keeps common short pieces in the index. At 256K the advantage narrows:
   cjk4 costs japanese −2.6% and is rejected; cjk8 recovers chinese to only
   +0.8% at ≈0 cost elsewhere.
3. **cjk8 is the safe v0.4 candidate**: identical B/T to v0.3 within noise
   (5.5511 vs 5.5543, paired bootstrap −0.0032 [CI −0.006, 0.000]), identical
   encode speed (10.18 / 56.4 MB/s), identical decode exactness.
4. `--min-freq` has no measurable effect here (C == A exactly); the v0.3 CJK
   story lives entirely in `--min-word-freq`.
5. MathSymbolAware remains the reference pretokenizer; the CJK arms are purely
   additive sub-splits of CJK runs.
