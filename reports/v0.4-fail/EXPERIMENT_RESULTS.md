# NTokenizer v0.4 — Experiment Results (narrative)

Machine-readable data: `EXPERIMENT_RESULTS.csv` (35 rows, every one traceable to a
benchmark JSON in `experiments/v0.4/results/` and, for trained rows, a training
log in `experiments/v0.4/logs/`).

## Headline comparisons (continuity benchmark, 544 files, 207,712,944 B)

| Arm | B/T | Equal-cat | Note |
|---|---:|---:|---|
| o200k_base (tiktoken) | 4.0751 | 4.0899 | independent measurement |
| NTokenizer v0.1 | 3.1676 | 3.1203 | ConservativeUnicode |
| NTokenizer v0.2 | 4.9290 | 4.6857 | LeadingWhitespaceUnicode |
| NTokenizer v0.3 | 5.5543 | 5.1057 | MathSymbolAware |
| **NTokenizer v0.4 candidate (cjk8)** | **5.5511** | **5.1032** | parity with v0.3; chinese +0.8% |
| NTokenizer v0.4 (cjk4, rejected) | 5.5267 | 5.0860 | japanese −2.6% |
| NTokenizer v0.4 (A control, carved corpus) | 5.5577 | 5.0224 | corpus artifact, not a model |

Token reduction vs o200k_base: v0.3 −26.6%, v0.4 candidate −26.6%.
All NTokenizer models: byte-exact decode on 544/544 files.

## What was measured, ordered by the brief's experiment queue

1. **CJK root-cause** — confirmed: flat `--min-word-freq 2` starves CJK pair
   frequencies. `min-freq` is inert (C≡A). mwf=1 recovers chinese +16% (2.95→3.42),
   japanese +2.7%, korean +3.3%.
2. **E (CJK exemption, pilot)** — chinese 3.45, japanese 4.35, korean 4.22; works,
   but at full scale the pair index exploded (8.3 GiB at 3.5% of merges) — killed.
3. **F arms (CJK chunking)** — F1 cjk4 best at pilot (chinese +7.97% holdout) but
   −7.5% encode speed and japanese −2.6% at 256K; F2 japanese −4.8%–−5.1%; F3 cjk8
   the only safe full-scale arm; G (cjk4+exempt) beats F1 on CJK but costs
   non-CJK −1.3–1.8%.
4. **Full-scale 256K** — v0.4 candidate trains in 4:38 (faster than v0.3's 5:47),
   3.75 GiB peak RSS, chinese 3.566 (+0.8%, bootstrap CI [+0.017,+0.038]), japanese
   −0.6%, korean −0.04%, overall −0.06% (parity), identical encode speed (10.18/56.4 MB/s).
5. **Parity-aware BPE** — Gini −40% (0.0917→0.0552) but tokens +25.9% — rejected;
   hybrid (250 global merges) shows no benefit (Gini 0.0905, worse B/T).
6. **Unigram LM** — 32K Unigram (SP default) 4.1103 B/T, *below* 32K frequency BPE
   (4.1967), and 0/544 decode-exact — rejected; identity+byte_fallback much worse
   (3.7422) and still lossy (192/544).
7. **MDL-SG** — not attempted; single-author preprint, machinery too heavy for the
   v0.4 budget, CJK was the binding constraint. Documented as skipped, not hidden.
8. **Export ladder** — 8/16/32/64/256K at 3.25/3.72/4.20/4.69/5.55 B/T.
9. **Small LM** — tiny GPT BPB: 8K 3.73 → 16K 3.61 → 32K 3.38 → 64K 3.18 → 256K 2.86;
   monotonically decreasing in our undertrained regime; recommendation: 32K for 50M.
10. **HF compatibility** — plain tokenizer.json: 523/523 decode-exact, **46/523 ID
    parity**; custom Python backend: 2/523 (only printable-ASCII no-space succeeds);
    recommendation: native bindings for exact parity; export is structural only.

## Failures and negative results (none hidden)

- E (CJK exemption) infeasible at scale — recorded.
- F2 cjk2 rejected for Japanese collapse.
- v0.4 cjk4 rejected for Japanese −2.6% and encode −7.5%.
- Parity-aware and Unigram rejected for measured quality regressions.
- The carved corpus (v0.4-A control) shows the corpus-exclusion artifact:
  other_languages −28.8% on the old benchmark — proving that arm was measuring
  data removal, not tokenizer quality.
- The old benchmark's `other_languages` bucket remains fragile for any corpus
  change; reported, not fixed.

## Bootstrap (paired, file-level, 10,000 resamples)

- v0.3 vs v0.2: +0.625 B/T [CI +0.543, +0.708], equal-cat +0.420 [+0.341, +0.483].
- v0.4 candidate vs v0.3: −0.003 B/T [CI −0.006, −0.000], equal-cat −0.003
  [−0.005, −0.000] — statistically significant but practically a tie.
- v0.4 candidate vs v0.3 Chinese: +0.028 [+0.017, +0.038], P=1.000; Japanese:
  −0.032 [−0.044, −0.021].

## What v0.4 actually improved

(Equal-disclosure: not the headline B/T.)

- New Benchmark v1 (validation + final holdout), leakage-audited, bootstrap CIs.
- `--min-word-freq-cjk` trainer option + `cjk2/4/8` pretokenizer arms (unit-tested).
- Root-caused CJK regression to the flat word-frequency floor; shipped a fix.
- v0.4 candidate: byte-exact, speed-at-parity, chinese +0.8%, japanese −0.6%.
