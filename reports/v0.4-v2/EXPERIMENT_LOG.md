# NTokenizer v0.4-v2 — Experiment Log

Date: 2026-11-16
All experiments were run on the same machine, same benchmark files, same evaluation
harness (`examples/benchmark_ntok`), same byte counts, same normalization (None), same
pretokenizer (`MathSymbolAware`), seed 42. Pilots use 64K vocabulary to iterate cheaply;
they are comparisons between recipes, not final results.

## E-00 — Baseline reproduction (operational baseline)

- Reproduced polished v0.3-v2 on the development benchmark: 37,397,319 tokens, 5.5542 B/T,
  equal-category 5.1056, decode 544/544 byte-exact, 245,334/262,144 used (93.59%).
- Historical v0.3 figures retained for the record (37,397,108 / 5.5543 / 5.1057).
  Reconciliation: see `BASELINE_AUDIT.md` §3 — origin is pre-session source drift, not this cycle.
- Command and JSON: `experiments/v0.4-v2/results/baseline_single.json`, per-file:
  `experiments/v0.4-v2/results/baseline_perfile.json`.
- Decision: this is the operational baseline for every comparison below.

## E-01 — Corpus expansion pilot (plain)

- Hypothesis: the corpus (1.96 GB) under-covers the benchmark's dominant buckets
  relative to what the same clean tree can provide; fresh in-distribution data improves
  merge frequency estimates at fixed recipe.
- Method: build `data/v0.4-v2/manifest/manifest_corpus.json` = v0.3 manifest + unused,
  licensed, non-holdout files from `data/v0.2/clean/train`, per-bucket caps
  (turkish +330, arabic +302, hindi +212, english +252, korean +160, spanish +95,
  japanese +61, stem +45, chinese +31 MB) → 11,480 files, 3.45 GB, manifest SHA-256
  `e204ce6b1b6641348ea0bcbe3540fd842984a52147eb701b8f6f7d5e813bd536`.
  Pilot A: v0.3 manifest, 64K, mwf=2, min-freq=2, math, seed 42 → 3m51s, 4.12 GB RSS.
  Pilot B: expanded manifest, identical args → 6m07s, 5.84 GB RSS.
- Result (dev benchmark, 544 files):
  - A: 4.6869 B/T (equal-category 4.2786) — matches the recorded v0.3 64K export exactly.
  - B: 4.7125 B/T (equal-category 4.2629).
  - Paired bootstrap (n=544, 2000 resamples): overall **+0.55% [+0.28%, +0.81%]**.
  - Per-bucket (B vs A): turkish +1.99% [+1.57,+2.34], arabic +1.17% [+0.75,+1.60],
    hindi +0.83% [+0.59,+1.05], korean +0.72% [+0.19,+1.34], english +0.54% [−0.01,+1.09];
    **programming −3.25% [−3.63,−2.89], stem −1.55% [−2.05,−1.10], japanese −1.71%
    [−2.42,−1.11], chinese −1.45% [−2.86,−0.45], spanish −0.61% [−0.96,−0.30]**.
  - Equal-category: 4.2786 → 4.2629 (−0.37%).
- Diagnosis: fixed 64K vocabulary slot competition — new Turkish/Arabic/Hindi/Korean
  merges displace code/STEM/CJK merges. The effect is CI-separated, not noise.
- Decision: **rejected as a promotion candidate at full scale.** Guardrail "programming
  must not become worse" is violated, and two more guardrails (stem, CJK) are also violated.
  The +0.55% average gain is real but too small to approach 6.00 B/T (needs +8.0%) and
  is not worth the projected ≈5.5–6.5 GiB full-run memory risk.

## E-02 — Source-Attributed BPE (Research A) — evidence review, not implemented

- Hypothesis: a mild source-diversity-aware merge priority (e.g. `F·log(R+1)`) recovers
  code compression by de-prioritizing one-source repository-private merges.
- Method: audit what attribution the corpus actually provides. Manifest carries coarse
  `source`/`domain`: cpython_code (2275 files / 45.8 MB), typescript_src (686 / 31.7 MB),
  go_src (2117 / 30.4 MB), rust_src (1419 / 15.6 MB), cpython_docs (601 / 17.2 MB).
  No per-repository or per-file repo keys exist inside these buckets (cpython_code is the
  whole CPython tree; go_src is the whole Go tree). Programming languages count L≈5 as well.
- Consequence: `log(R+1) ≈ log(5) ≈ 1.6` for essentially every code pair — a constant
  multiplier that cannot reorder merges. Any "source attribution" implementation here would
  be decorative: R≈4–5 at bucket granularity, R=1 at nothing finer.
- Benchmark context: programming is 2.0% of benchmark bytes; even a 20% code gain ≈ +0.4% overall.
- Decision: **rejected on evidence before implementation.** Absence of the required
  instrumentation is the finding; no fake "repository" key was fabricated, per instructions.
  A provenance-aware merge audit (count one-source merges post-hoc) is also impossible:
  `.ntok` stores no provenance and the pair index is discarded after training.
- Cost: 0 training runs. Archived notes here as the record.

## E-03 — Parity-aware BPE (Research B) — not repeated

- Prior failure reproduced from `reports/v0.4-fail/ALGORITHM_COMPARISON.md`: parity
  constraint improved Gini (0.0917→0.0552, −40%) while overall compression collapsed
  (B/T 1.6735→1.3302, +25.9% tokens); hybrid variant had no benefit.
- Guardrail check for any re-tests would be: byte-weighted B/T must stay ≥ baseline and
  worst-category regression bounded. No pilot was run; repeating the failed arm unchanged
  is explicitly forbidden by the brief.
- Decision: **not attempted**, documented.

## E-04 — Guarded expansion pilot (code/STEM ×2 weight)

- Hypothesis from E-01: the programming/STEM regression is vocabulary competition;
  multiply the *file* count of code/stem sources (weight=2.0, trainer's native mechanism)
  to defend those merges while keeping the winning-bucket additions (turkish +221,
  arabic +161, hindi +142, korean +110, english +90 MB). No japanese/chinese/spanish/stem-new.
  Manifest: `data/v0.4-v2/manifest/manifest_guard.json` (10,891 entries, SHA-256
  `289bad2b98983d6efeabe79a7e8091beb742f6503c0cd100ea6b3aacfda6c33f`); protection weight
  applied to cpython_code, cpython_docs, typescript_src, go_src, rust_src, arxiv,
  gutenberg_science, gutenberg_science2 (9459 files, ≈330 MB repeated ⇒ nominal 3.02 GB).
- Result (64K, same everything): overall **4.6496 B/T (−0.79% vs pilot A)**, equal-category
  4.2274; programming **+2.91%**, stem **+2.73%**, turkish +0.64%, but chinese −4.74%,
  japanese −5.13%, spanish −4.58%, korean −0.93%, arabic −0.81%.
  paired bootstrap (n=544, 2000 resamples): B is worse than A by 0.037 B/T absolute
  (95% CI [0.019, 0.057]; ≈ −0.8% relative).
- **Peak RSS 6.63 GB** — exceeds the 6 GiB hard experimental ceiling (duplicated file
  bytes expand the pair index). Per protocol, the arm was stopped at 64K and archived;
  no full-scale run.
- Decision: **rejected.** Worse overall, same guardrail failures, over the memory ceiling.

## E-05 — Research C review (SentencePiece documentation)

- No SentencePiece import; no Unigram; no copied defaults.
- Applicable pieces applied: corpus sampling via manifest caps/weights (used in E-01/E-04);
  script-boundary behavior already present in MathSymbolAware; digit/number handling already
  present; character coverage already total (byte-level). Sampling with mild category
  temperature was effectively realized and measured in E-01/E-04.
- Decision: no further code change; documented in `RESEARCH_NOTES.md`.

## E-06 — CJK levers

- Fixed-size Han chunking: rejected historically (CJK4/CJK8), and v0.4's only positive arm
  (Han4) re-measured this cycle region at 5.5511 B/T continuity, japanese −0.6%: still failing.
- `--min-word-freq-cjk 1` at pilot scale: was the only positive CJK signal (chinese +15–22%
  at pilot, korean +11%, japanese −0.4%), but it is not memory-feasible at 256K on this corpus
  (pair index explosion; the v0.4-fail analysis already ruled full-scale E infeasible),
  and it does not help the target buckets enough to matter even if it were safe (chinese is
  1.4% of benchmark bytes).
- Decision: not attempted at full scale; no CJK arm was added to any candidate. Japanese
  guardrail therefore trivially satisfied (unchanged), at the cost of no CJK gain.

## E-07 — min-freq / min-word-freq sensitivity

- `min-freq` was shown inert in the v0.3 ablations; rerunning it is a poor use of budget.
  `min-word-freq 3` would only prune more aggressively (the v0.4-fail mwf=1 arm exploded
  to ~235M heap entries / 8.3 GiB and was killed). **Not attempted** — no safe upside demonstrated.
  Archived as considered-and-skipped.

## E-08 — Vocabulary utilization / merge-utility audit

- Benchmark-used tokens: 245,334 / 262,144 (93.59%); unused-on-benchmark: 16,810.
- Post-hoc provenance/marginal-utility reconstruction is impossible from `.ntok` (no
  provenance stored, pair index discarded). No token removal was done or proposed;
  intermediate merge tokens must be preserved for later merges in any case.
- Decision: audit completed as far as artifacts allow; limitation documented in
  `BASELINE_AUDIT.md` §6. No candidate mutation based on it.

## Summary table

| Experiment | Pilot result | Guardrails | Memory | Decision |
|---|---|---|---|---|
| E-01 plain expansion (3.45 GB) | +0.55% overall (CI-separated) | **programming −3.25%, stem −1.55%, japanese −1.71%, chinese −1.45% — violated** | 5.84 GB @64K | rejected |
| E-02 source-attributed merge | not implementable (R≈4–5, no repo keys) | n/a | n/a | rejected on evidence |
| E-04 guarded expansion (code/stem ×2) | −0.79% overall | japanese −5.13%, spanish −4.58%, chinese −4.74% — violated | **6.63 GB > 6 GiB ceiling** | rejected; stopped |
| E-05 SentencePiece review | n/a | n/a | n/a | applied (sampling only) |
| E-06 CJK arms | none positive & feasible | CJK guardrail | constraint | rejected/not attempted |
| E-07 min-freq sensitivity | not attempted (v0.3 showed inert) | n/a | memory-infeasible up | skipped |
| E-08 vocab audit | 93.59% used; audit limited by artifacts | n/a | n/a | documented |

**No full 256K candidate was trained.** The authorization gate (pilot shows a credible quality
improvement *and* acceptable guardrails) was not met by any arm; running ~25 minutes of
probable-OOM training on that evidence would be exactly the "repeated full-scale job without
a clear reason" the brief forbids.
