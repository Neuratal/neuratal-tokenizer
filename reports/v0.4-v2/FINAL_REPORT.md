# NTokenizer v0.4-v2 — Final Report

Date: 2026-11-16
Decision: **NOT PROMOTED — keep polished v0.3-v2.** No candidate reached the target;
no new model was produced or promoted. The polished v0.3-v2 implementation and the
v0.3 checkpoint remain the model of record, byte-identical and unmodified.

Source code was **not modified** in this cycle (no `src/`, `tests/`, `examples/` changes).

## A. Executive summary

- Target: overall byte-weighted B/T strictly > 6.00. Current polished model: **5.5542 B/T**.
  Not achieved. Not approached by any arm that respects the guardrails (best pilot: +0.55%).
- Diagnostics found one legitimate, leakage-free lever — **3.12 GB of unused, licensed,
  non-holdout clean training data** withheld from v0.3's manifest. A 64K pilot trained on it
  gained **+0.55% overall (CI-separated)** but regressed programming (−3.25%), stem (−1.55%),
  japanese (−1.71%), chinese (−1.45%), spanish (−0.61%) and equal-category (−0.37%).
- A guarded variant duplicating code/STEM sources recovered programming (+2.91%) but lost
  more elsewhere (−0.79% overall) and **peaked at 6.63 GB — over the 6 GiB hard ceiling**.
  Stopped and archived.
- A full 256K expanded candidate was therefore **not run**: pilots did not demonstrate the
  required credible improvement, and projected memory (≈5.5–6.5 GiB) risked the hard ceiling.
- Source-Attributed BPE (Research A) is not implementable on this corpus (no per-repo keys;
  R≈4–5, `log(R+1)` is a constant; programming is 2.0% of the benchmark). Research B
  (parity-aware) was not repeated. Research C (SentencePiece docs) informed sampling only.
- Nothing shipped. `output/v0.4-v2/` contains no model bytes by design.

## B. Changes made

**No production source changes.** New files only (reports, manifests, pilots, logs, this
report). The v0.3-v2 polished implementation is 100% retained. The v0.3 model is untouched:
SHA-256 `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9`.

New artifacts:
- `data/v0.4-v2/manifest/manifest_corpus.json` (expanded, 11,480 files, 3.45 GB, SHA-256 `e204ce6b…`)
- `data/v0.4-v2/manifest/manifest_guard.json` (guarded, rejected, SHA-256 `289bad2b…`)
- `experiments/v0.4-v2/models/{pilot_a_v03mix_64k,pilot_b_expanded_64k,pilot_guard_64k}.ntok`
- `experiments/v0.4-v2/results/*.json`, `experiments/v0.4-v2/logs/*.log`
- `reports/v0.4-v2/{RESEARCH_NOTES,BASELINE_AUDIT,EXPERIMENT_LOG,FINAL_REPORT}.md`

## C. Experiments summary

Full detail with commands, CIs, memory and decisions: `EXPERIMENT_LOG.md`.

| Arm | Outcome |
|---|---|
| E-01 corpus expansion pilot (plain, 64K) | +0.55% overall [+0.28,+0.81]; programming −3.25% [−3.63,−2.89]; stem −1.55%; japanese −1.71%; chinese −1.45%; spanish −0.61%; turkish +1.99%; arabic +1.17%; hindi +0.83%; korean +0.72%; english +0.54%. Equal-category −0.37%. **Rejected** (guardrails) |
| E-02 source-attributed merge priority | Not implementable on this corpus (R≈4–5, no repo keys). **Rejected on evidence** |
| E-03 parity-aware BPE | Not repeated (documented prior failure: compression collapse +25.9% tokens) |
| E-04 guarded expansion (code/STEM ×2) | −0.79% overall; peak RSS 6.63 GB **> 6 GiB ceiling — stopped**. **Rejected** |
| E-05 SentencePiece doc review | Applied as sampling-only mechanism (E-01/E-04); no Unigram, no imports |
| E-06 CJK levers | None positive-and-feasible; Han4 re-measured 5.5511 B/T (continuity), japanese −0.6%. Not promoted |
| E-07 min-freq/mwf sensitivity | Not attempted (v0.3 ablations: inert; mwf=1 previously OOM-killed at ~8.3 GiB). Documented as skipped |
| E-08 vocabulary utilization audit | 245,334/262,144 used (93.59%); post-hoc provenance audit impossible from artifacts; limitation documented |

## D. Benchmark comparison

Historical table (dev benchmark, byte-weighted overall B/T; equal-category in parentheses):

| Tokenizer | Vocabulary | Overall B/T | Equal-category | Relative to o200k | Status |
|---|---:|---:|---:|---:|---|
| o200k_base | 200K | 4.0751 | 4.0899 | Reference | External |
| NTokenizer v0.1 | 256K | 3.168 | — | −22% | Historical |
| NTokenizer v0.2 | 256K | 4.9290 | 4.6857 | +21.0% | Historical |
| NTokenizer v0.3 original | 256K | 5.5543 | 5.1057 | +36.3% | Superseded |
| NTokenizer v0.3-v2 polished | 256K | **5.5542** (37,397,319 tok) | **5.1056** | **+36.2%** | **Baseline (model of record)** |
| NTokenizer v0.4-fail | 256K | 5.5511 | 5.1031 | +36.2% | Rejected |
| NTokenizer v0.4-v2 | — | **not produced** | — | — | **NOT PROMOTED** |

Per-category (dev benchmark, this cycle, polished build; o200k from `reports/v0.3/BENCHMARK_REPORT.md`):

| Category | v0.3-v2 polished | o200k_base | v0.4-v2 | Change vs polished |
|---|---:|---:|---:|---|
| English | 4.4628 | 4.154 | N/A (no candidate) | — |
| Programming | 4.0961 | 3.677 | N/A | — |
| Turkish | 5.3054 | 3.447 | N/A | — |
| Chinese | 3.5380 | 2.748 | N/A | — |
| Japanese | 5.0135 | 3.424 | N/A | — |
| Korean | 4.9999 | 3.267 | N/A | — |
| Arabic | 7.1984 | 4.763 | N/A | — |
| Other languages | 3.3196 | 5.188 | N/A | — |
| Mathematics/STEM | 3.5638 | 2.936 | N/A | — |
| Hindi | 9.9277 | 7.237 | N/A | — |
| Spanish | 4.7363 | 4.148 | N/A | — |
| Equal-category | 5.1056 | 4.0899 | N/A | — |

Implementation performance (candidate does not exist; polished baseline measured/unchanged):

| Metric | v0.3-v2 polished | v0.4-v2 |
|---|---|---|
| Single-thread encode | 16.5–17.2 MB/s (this cycle, 3 repeats) | N/A (unchanged code) |
| 8-thread encode | 92.3–96.6 MB/s (v0.3-v2, not re-measured) | N/A |
| Decode throughput | 278.2 MB/s (this cycle) | N/A |
| Peak encode RSS | 360 MB (v0.3-v2, not re-measured) | N/A |
| Decode round-trip | 544/544 byte-exact | N/A |
| Training time | N/A (no training run) | — |
| Peak training RAM | N/A | — |
| Vocabulary utilization (dev benchmark) | 93.59% | — |

Pilot-phase measurements (not candidate results): A: 4.6869 / equal-cat 4.2786; B (expansion):
4.7125 / 4.2629; G (guarded): 4.6496 / 4.2274. See `EXPERIMENT_LOG.md`.

Bootstrap CIs reported (paired, file-level, n=544, 2000 resamples): overall +0.55%
[+0.28,+0.81] (B), −0.80% (G); per-bucket in `EXPERIMENT_LOG.md`. No CI supports a
candidate approaching 6.00 B/T; none was trained.

## E. Testing

No source code changed this cycle, so the polished test suite results carry over
unchanged (136/136 Rust, 14/14 Python bindings, 19/19 cleaner, 7/7 corpus integrity,
Unicode regression 6/6, fuzz roundtrip/load_model pass per v0.3-v2 report). This cycle
additionally verified:

- Model loads, validates, encodes, decodes: **PASS** (`ntokenizer validate`, 544/544 byte-exact).
- Baseline reproduction: **PASS** (tokens 37,397,319, B/T 5.5542).
- Pilot training runs: 3/3 completed without crashing; guard pilot stopped at analysis (no code ran beyond training; 6.63 GB RSS, over ceiling — noted, not shipped).

No tests were modified, weakened or deleted in this cycle.

## F. Model integrity

| Item | Value |
|---|---|
| v0.3 SHA-256 (before cycle) | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` |
| v0.3 SHA-256 (after cycle) | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` (identical) |
| v0.4-v2 candidate model SHA-256 | **none exists** |
| `output/v0.3/`, `output/v0.3-v2/` | untouched |
| v0.2 / v0.1 artifacts | untouched |
| Holdout (`data/v0.4/benchmark/holdout/`) | untouched; not used for training |
| NTOK format | unchanged (v3) |
| Special tokens / IDs | unchanged |

## G. Final decision

**3 — NOT PROMOTED. Keep polished v0.3-v2.**

- The candidate fails to exist: no arm produced a model worth shipping, let alone one
  exceeding 6.00 B/T. Best evidence-backed effect: +0.55% at 64K scale with guardrail-breaking
  regressions. The 8.0% gap to 6.00 is not closable with the levers available in a bounded
  cycle on this corpus.
- Conditions 13 (no benchmark hardcoding), 14, 15 were honored: no leakage was introduced,
  no benchmark files were trained on, the holdout stayed sealed, no test was weakened.
- The polished v0.3-v2 release from 2026-11-15 remains the model of record: fast,
  Unicode-correct, byte-exact, and honestly characterized (development-benchmark caveat
  applies to every number in this report).

## H. Next step

Stop tokenizer-quality work on this track and move to language-model training with the
polished v0.3 checkpoint — **but** with two explicit reminders carried forward:

1. The tokenizer is **not** "solved" at 6.00; it is the best measured general-purpose
   byte-level BPE for this corpus, at 5.55 B/T vs o200k's 4.08. Downstream LM results,
   not B/T, are the next decision point.
2. If the 6.0 question is to be revisited, the only credible path found is the sealed
   expansion recipe (E-01) plus a full 256K run with a >6 GiB memory budget **and** a
   genuinely untouched evaluation slice; neither was authorized or feasible here. Do not
   promise 6.0 internally from 64K pilots. Keep the dev-benchmark caveat attached to all
   external claims.

Also unresolved and worth one focused session (not this cycle): the +211-token drift of the
current tree vs the recorded v0.3 figure (37,397,319 vs 37,397,108) — decide whether to
re-baseline or formally accept the polished-tree tokenization, and tag the crate.
