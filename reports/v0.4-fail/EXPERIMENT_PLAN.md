# NTokenizer v0.4 Experiment Plan

Date: 2026-11-15
Status: executed iteratively; see EXPERIMENT_RESULTS.csv / EXPERIMENT_RESULTS.md.
Every experiment uses the same benchmark discipline: (a) `data/v0.2/clean/benchmark`
for continuity with v0.2/v0.3 (fair for all NTokenizer-derived models), (b) the new
Benchmark v1 holdout/validation splits for decisions made during this cycle
(fair for all models on unused-pool buckets; carved buckets flagged).

## Controlled variables

Unless a row says otherwise, all trained arms use: same corpus manifest, same
`--min-freq 2`, same `--min-word-freq 2`, same seed 42, same vocab budget per arm,
same native benchmark harness (5 measured runs × warmup 1, median).

## Experiment matrix (executed)

| ID | Hypothesis | Arm | Status | Verdict |
|---|---|---|---|---|
| E-00 | v0.3 baseline reproduces | v0.3 256K re-benchmark | done | reproduced (5.5543 / 5.1057) |
| E-01 | CJK loss is caused by flat `mwf=2` pruning of unsegmented runs | A=math mwf2 vs B=math mwf1 (pilot 64K) | done | confirmed: chinese +21.2%, korean +9.8%, japanese +1.7% for B |
| E-02 | `min-freq` (not `min-word-freq`) is the cause | A vs C (min-freq 1 vs 2) | done | no effect — C == A exactly |
| E-03 | CJK exemption from pruning fixes CJK without mwf=1 cost | E=math mwf2 + `--min-word-freq-cjk 1` (pilot) | done | chinese +22.4%, korean +11.0% on holdout; japanese −0.4% |
| E-04 | Same exemption at production scale is feasible | E on 0.63 GB pilot (64K) | done | works (2.0 GiB RSS, 2:05 wall) |
| E-05 | Same exemption at full 1.96 GB × 256K | E on full corpus (killed at 3.5% merges, 8.3 GiB RSS, 235M heap entries) | failed | infeasible: pair index explodes; aborted |
| E-06 | Chunked CJK runs keep merge signal without exemption | F1/F2/F3 = cjk4/cjk2/cjk8 (pilot) | done | F1: chinese +8.0%, japanese −1.7%; F2: japanese −5.1%; F3: chinese +1.7%, ≈0 cost |
| E-07 | Chunked CJK at production scale preserves gains | cjk4 full-scale | done | chinese +3.3%, korean −1.6%, japanese −2.6%, overall −0.5%, encode −7.5% — **rejected for release** |
| E-08 | cjk8 at production scale is a safe CJK improvement | cjk8 full-scale | done (this report) | see ALGORITHM_COMPARISON |
| E-09 | Parity-aware merge selection improves balance without hurting compression | reference impl, 300 merges, ratio=1.0, vs classical | done | Gini 0.0917→0.0552 (−40%) but tokens +25.9% (B/T 1.67→1.33) — not adopted; hybrid-with-global-merges variant under test |
| E-10 | Unigram LM (Reference) beats BPE at same bytes | SentencePiece 32K Unigram, NFKC and identity normalizers | done | NFKC arm wins B/T by ~1.1% but **0/544 decode-exact** — rejected for byte-exact pipeline; identity arm much worse |
| E-11 | Encoding throughput regressed due to MathSymbolAware cost, not a bug | v0.2 vs v0.3 single-thread & 8-thread medians, profile pending | done | −12.3% / −13.6% measured; profiling pending (see PERFORMANCE_REPORT) |
| E-12 | CJK arms affect model file size / vocab usage | all pilot arms | done | all within ~1%; reported |
| E-13 | Small-LM pilot decides best vocab for 50M model | TinyGPT on 8K/16K/32K exports (BPB) | planned/in progress | see SMALL_LM_EVALUATION |

## Acceptance criteria for v0.4 release candidate

1. Byte-exact round-trip 544/544 (old benchmark) and on unicode_stress fixtures.
2. v0.3-level overall B/T preserved or improved; no unexplained >2% bucket regression.
3. CJK regressions vs v0.3 eliminated or reduced with an explicit, reported trade-off.
4. Training fits in RAM (≤ 4.5 GiB preferred, ≤ 8 GiB hard limit, no swap thrash).
5. Tests 100% pass; compatibility preserved (v1/v2/v3 load, migration, HF export).
6. Publish posture decided by the user; nothing uploaded without approval.

## Arms considered and rejected (kept for honesty)

- **RawByte**: infeasible at scale (7.5 GiB RSS at 325/32,505 merges in v0.3).
- **ScriptTransitionAware**: catastrophically worse (2.90 B/T pilot).
- **Full CJK retention (E, mwf2 + CJK-exempt-1)**: pair index explodes at production scale; killed.
- **cjk2 chunking**: Japanese −5.1%; rejected.
- **Parity-aware merge rule**: see E-09.

## Command log

All training commands are in `experiments/v0.4/logs/*.log` with their `/usr/bin/time -v`
output; benchmark commands are in the respective JSON headers; SHA-256 of all
artifacts in `reports/v0.4/SHA256SUMS`.
