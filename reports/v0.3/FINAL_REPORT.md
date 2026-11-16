# NTokenizer v0.3 — Final Report

Date: 2026-11-15
Status: complete. v0.1/v0.2 artifacts untouched; all v0.3 artifacts are new; nothing
was pushed to GitHub or uploaded to Hugging Face.

## 1. Executive summary

NTokenizer v0.3 is a retrained 256K-vocabulary byte-level BPE tokenizer:
`output/v0.3/ntokenizer-0.3-256k.ntok` (SHA-256
`1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9`), trained on
1.96 GB of the leakage-audited v0.2 clean corpus with a new math/symbol-aware
pre-tokenizer and a frequency floor that makes training fit this machine.

Measured on the unchanged held-out benchmark (544 files, 207,712,944 bytes):

| Metric | v0.2 | **v0.3** | o200k_base | v0.3 vs v0.2 | v0.3 vs o200k |
|---|---:|---:|---:|---:|---:|
| Tokens | 42,140,653 | **37,397,108** | 50,971,306 | −11.26% | **−26.6%** |
| Byte-weighted B/T | 4.9290 | **5.5543** | 4.0751 | **+12.7%** | **+36.3%** |
| Equal-category B/T | 4.6857 | **5.1057** | 4.0899 | **+9.0%** | **+24.8%** |
| Tokens/byte | 0.20292 | **0.18004** | 0.24540 | −11.3% | −26.6% |
| Used vocab | 232,659 / 262,144 | **245,334 / 262,144 (93.59%)** | 133,550 / 200,019 (66.77%) | +5.4 pp | — |
| Decode exactness | 544/544 | **544/544** | n/a | equal | equal |
| Model size | 22,712,471 B | **23,403,000 B** | — | +3.0% | — |
| Encode (1T / 8T) | 11.61 / 65.10 MB/s | **10.18 / 56.30 MB/s** | 16.66 MB/s (Python incl. I/O) | −12% / −14% | not comparable |

v0.3 beats v0.2 on **9 of 11** buckets (max +27.5% programming, +19.1% arabic) and
loses three buckets that must be disclosed: chinese (**−5.76%**), japanese
(**−6.75%**) and the tiny other_languages (**−15.91%**) — see `BENCHMARK_REPORT.md`
§4 for the root cause (CJK single-token coverage drop from the mandatory
`--min-word-freq 2` pruning).

**The aggressive target — 2× o200k_base (8.1502 B/T) — was not achieved.** v0.3
reaches 5.5543 B/T. The honest gap analysis is in `BENCHMARK_REPORT.md` §6 and
below in §6. No metric was redefined, no file added to the benchmark, and no
training data was leaked to inflate the result.

## 2. What changed vs v0.2

1. **Data**: production manifest `data/v0.3/manifest/manifest_corpus.json` —
   1,963.48 MB, 10,555 files, deterministic per-bucket caps aimed at the v0.2
   headroom table (Arabic 24×, Turkish 11×, Hindi 12×, Korean 9×, Spanish 7×
   unused before). Same leakage-audited `clean/train` tree; zero benchmark bytes.
2. **Pretokenizer**: `MathSymbolAware`, selected by a controlled tournament
   (32K/20MB → 64K/60MB) over LeadingWhitespaceUnicode, ConservativeUnicode,
   ScriptAndDigitAware, UnicodeHybrid, CodeAware, MathSymbolAware,
   ScriptTransitionAware, and RawByte (the two legacy v0.1 controls are dominated
   by LWS by 25–28%). Winner at 64K: math 4.6033/
   4.2640, hybrid 4.5992/4.2585, code 4.6004/4.2366 — code and hybrid are within
   0.1%; math was chosen for STEM/code binding. RawByte failed feasibility
   (7.52 GiB RSS at 325/32,505 merges) and is reported, not shipped.
3. **Trainer floor**: `--min-word-freq 2` prunes 71.9% of unique pre-tokens
   before pair indexing; final training peaked at
   **3.70 GiB** vs ~9.7 GB for v0.2, on 1.93× the corpus.
4. **Artifacts**: v0.3 model, HF export (`output/v0.3/hf/`), independent benchmark
   harness runs, 10 required reports under `reports/v0.3/`.

## 3. Verification status

| Check | Result |
|---|---|
| `cargo test --release` | 126 tests pass, 0 fail |
| Python ctypes bindings | 14/14 pass |
| `ntokenizer validate --model v0.3 [--roundtrip]` | all PASS (13/13 probes) |
| Decode exactness on benchmark | 544/544 byte-exact, v0.2 and v0.3 |
| HF `tokenizer.json` load (tokenizers + transformers) | loads; special tokens resolved |
| HF round-trip vs native | 16/16 byte-exact both directions; token-ID parity 3/16 (expected, documented) |
| CRC/length corruption rejection | 3/3 rejection classes verified on fixture copies |
| NTOK v1→migrate→load | OK |
| Atomic save | temp+rename implemented; not unit-tested (noted) |
| Fuzzing | **not run** (pre-existing ASAN/linker issue) |

Full evidence: `COMPATIBILITY_REPORT.md`, `PERFORMANCE_REPORT.md`,
`MEMORY_REPORT.md`, `experiments/v0.3/logs/`, `experiments/v0.3/results/`.

## 4. Required deliverables — all present

| File | Purpose |
|---|---|
| `reports/v0.3/AUDIT_REPORT.md` | baseline re-measurement, target arithmetic, data headroom |
| `reports/v0.3/EXPERIMENT_PLAN.md` | executed plan; matrix, controls, acceptance criteria |
| `reports/v0.3/EXPERIMENT_RESULTS.csv` | 25 rows, machine-readable, incl. failure row |
| `reports/v0.3/EXPERIMENT_RESULTS.md` | narrative of the same |
| `reports/v0.3/TRAINING_REPORT.md` | final run + pilots + failures |
| `reports/v0.3/BENCHMARK_REPORT.md` | full buckets, equal vs weighted, 8.15-gap analysis |
| `reports/v0.3/PERFORMANCE_REPORT.md` | 5-rep medians, thread scaling, latency |
| `reports/v0.3/MEMORY_REPORT.md` | training + inference RSS tables |
| `reports/v0.3/COMPATIBILITY_REPORT.md` | tests, format versions, HF export, caveats |
| `reports/v0.3/LICENSE_AUDIT.md` | code/corpus/model obligations; no relicensing |
| `reports/v0.3/FINAL_REPORT.md` | this document |

Plus artifacts: `output/v0.3/ntokenizer-0.3-256k.ntok`,
`output/v0.3/MODEL_METADATA.json`,
`output/v0.3/hf/{tokenizer.json,tokenizer_config.json,special_tokens_map.json}`,
`data/v0.3/manifest/manifest_corpus.json`, `reports/v0.3/corpus_distribution.json`,
`reports/v0.3/hf_export_validation.json`, and all raw harness JSONs under
`experiments/v0.3/results/`.

## 5. Leakage and honesty controls

- All training manifests reference only `data/v0.2/clean/train`; the benchmark is
  only `data/v0.2/clean/benchmark`. No v0.3 manifest entry points into the
  benchmark tree (checked programmatically at manifest build time).
- Corpus selection is deterministic (`build_v03_manifest.py --profile final`),
  reproducible from the hash recorded in the manifest and the model metadata.
- Benchmark bucket assignments come from the benchmark manifest, never from
  filename heuristics.
- No benchmark-specific vocabulary injection; no post-hoc model selection; failed
  arms are reported (`RawByte`), not dropped.
- v0.3 wins on the equal-category metric too, so the win is not an artifact of
  byte-weighting the high-mass Turkish/Arabic buckets.
- chinese/japanese/other_languages regressions are reported, with root-cause as far
  as measured (mwf=2 pruning of rare whole-character merges; CJK single-token
  coverage 78.4%→44.5% unique / 99.43%→96.28% weighted).

## 6. The 8.15 B/T question, answered with measurements

Required tokens for 8.1502 B/T: **25,486,625**. v0.3 produced **37,397,108**
(−11.3% vs v0.2, still 46.7% above the target). Measured log/linear trends:

- Vocabulary scaling on the frozen v0.2 merge order buys: +27.8% (8K→32K),
  +10.7% (→64K), +8.9% (→128K), +4.2% (→196K), +2.7% (→256K). Each doubling pays
  less; a further doubling would buy ~+2%.
- Corpus scaling: +1.7% B/T from 20 MB→60 MB per bucket at 64K; the full 1.96 GB,
  256K, math config lands at 5.5543.
- A linear extrapolation of the measured curve cannot produce 8.15 at any feasible
  budget on this machine.

**Conclusion: 8.15 B/T via this architecture is not supported by the evidence.**
What would be needed (each unproven): materially larger clean multilingual corpus
(>20 GB), CJK whole-character merge seeding plus script-scoped frequency floors,
and possibly a >262K vocabulary (25× the measurement step size, far beyond the RAM
envelope we observed). These are new-experiment questions, not claims.

## 7. What is preserved and what is not pushed

- v0.1 (`output/ntokenizer-256k.ntok`) and v0.2 (`output/v0.2/`) are byte-identical
  to before v0.3 (read-only usage; their SHA-256s are recorded in
  `BENCHMARK_REPORT.md` §7).
- No `git push`, no Hugging Face upload, no release tag. The HF export is written to
  `output/v0.3/hf/` only; publishing it is a user decision.
- The git index was left as it was found (no destructive operations were run; no
  commit was created by this work).

## 8. Recommended next steps

1. Decide whether to publish v0.3 (tag, release notes, HF upload) — requires the
   CC BY-SA-compatible redistribution terms in `LICENSE_AUDIT.md` and user approval.
2. If CJK quality matters more than the memory headroom: retrain (or fine-tune the
   merge order) with `--min-word-freq 1` restricted to CJK script ranges, or seed
   CJK single characters as unprunable tokens; expect +4–6% on chinese/japanese and
   a lower but sufficient overall B/T.
3. Investigate a compact pair index (chunked or disk-backed) to make mwf=1 viable at
   1.96 GB; that recovers the full context quality without pruning.
4. Re-run the audit with a larger held-out CJK slice; the current Chinese bucket is
   2.95 MB / 18 files and dominates few aggregate decisions despite its regression.
5. Publish with `THIRD_PARTY_NOTICES.md` and `data/v0.2/LICENSES.md`, and keep the
   "derived from CC BY-SA corpus" notice on the model.
