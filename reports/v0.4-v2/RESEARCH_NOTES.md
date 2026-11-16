# NTokenizer v0.4-v2 — Research Notes

Date: 2026-11-16
Status: written before any v0.4-v2 code changes. Claims about the papers are based on the
project brief's detailed description plus general knowledge; the exact papers could **not be
retrieved via the available search backend** (zero hits for both titles). Where a result is
reported, it is quoted from the brief and must be verified against the primary source before
being used for publication-level claims. No paper is treated as proof of anything for NTokenizer.
Nothing here changes the byte-level BPE architecture; everything below is filtered by one
question: does this help overall byte-weighted B/T on our benchmark while keeping every
non-negotiable guardrail?

## Baseline context (as audited in-repo)

- Model of record: `output/v0.3/ntokenizer-0.3-256k.ntok`, SHA-256 `1f0d4ad7…` (verified).
- Polished source (v0.3-v2) encodes the 544-file development benchmark to **37,397,319 tokens
  / 5.5542 B/T**; the historical v0.3 report records 37,397,108 / 5.5543. The polished build is
  the operational baseline (per the project brief). Both figures are kept in the history.
- Corpus manifest `data/v0.3/manifest/manifest_corpus.json`: 1.96 GB, 10,555 files,
  seed 42, pretokenizer `math` (MathSymbolAware), min-freq 2, min-word-freq 2.
- Benchmark composition (share of 207.7 MB): turkish 20.7%, arabic 19.5%, english 15.4%,
  hindi 14.1%, korean 12.2%, spanish 7.6%, japanese 4.9%, stem 2.1%, programming 2.0%,
  chinese 1.4%. Byte-weighted gains must come mostly from the big non-English buckets and
  from English; CJK/programming alone cannot move the overall number by much.

## Research A — Source-Attributed BPE (ACL 2026), "From Where Words Come"

**What it does (per brief + general description).** Tracks, for each candidate pair,
`R` = number of repositories containing it, `L` = number of programming languages containing
it, `F` = pair frequency. Merge-priority adjustments and merge-skipping rules
(`F·L`, `F·log(R+1)`, `F·log(R)`, combinations) reduce merges that serve one repository's
private vocabulary. Reported effects: fewer under-trained tokens, small compression/coverage
gains on unseen repositories, at additional training cost.

**Applicability to NTokenizer.**

- Our corpus **does have coarse source identifiers**: the v0.3 manifest carries `source`/
  `domain` fields (`cpython_code`, `typescript_src`, `go_src`, `rust_src`, `arXiv`, `simplewiki`,
  …). It does **not** contain fine-grained repository boundaries — the ~140 MB of programming
  text is divided into exactly 5 coarse sources (cpython/typescript/go/rust/cpython-docs).
  The paper's `log(R+1)` with R≈4 is a constant multiplier on every code pair, i.e. it cannot
  differentiate merges beyond what frequency already does. The mechanism that gives the paper
  its signal (repo-level sparsity) is absent from this corpus.
- Language factor L is equally coarse (≈4 code languages).
- Programming is only **2.0%** of the benchmark's bytes. Even a 20% programming-B/T gain moves
  the overall number by ≈0.4%.

**Parts unsuitable for this cycle.** Full source-attributed merge re-weighting, per-language
merge partitions, any scheme requiring per-repository pair counts that do not exist in the
current trainer (the trainer currently keeps global pair stats; adding R/L per pair means
extending the pair index with per-pair source sets — a large invasive change for a 2% bucket).

**Decision.** Diagnostics-only pilot (measure merge provenance/undertraining), no full
re-weighted trainer implementation. If the diagnostic shows a pathological one-source tail in
the current vocabulary, consider a *skip-floor* on pairs with R==1 and F below a small
threshold as a pilot arm at 64K scale with the same seed; keep it only if it does not reduce
overall compression. Reject if code gain is trivial or overall regresses.

**Regression risks.** Code B/T regression if the floor is too aggressive; wasted budget on a
bucket that cannot move the headline number.

## Research B — Parity-aware BPE (EMNLP 2026), "Equity with Efficiency"

**What it does.** Constrains merge selection so per-language tokenization cost is balanced
(Gini ≈ 0.03 vs 0.22 for ordinary BPE in the brief's 90K example), at some compression cost
(0.0272 vs 0.0293 compression rate). Behaviour changes with vocabulary size; once useful
low-resource merges are exhausted, additional allocation drifts back to English.

**Prior NTokenizer work.** v0.4 implemented a parity-aware arm: Gini 0.0917→0.0552 (−40%) but
overall compression collapsed (B/T 1.6735→1.3302, +25.9% tokens) and the hybrid variant lost
its benefit entirely. Rejected. Root cause understood: the fairness constraint spends pairs
on low-frequency low-resource candidates whose frequency never recovers at 256K scale given
this corpus's composition.

**Applicability now.** Reimplementing it is explicitly off the table for the headline target.
The one piece worth retaining is the measurement discipline: report equal-category B/T (not
just byte-weighted) and per-language numbers with worst-category regression on every pilot we
run. That costs nothing and prevents a fairness-blind "improvement".

**Regression risks.** If revisited naively: catastrophic overall compression loss (as seen).
We do not revisit it.

## Research C — SentencePiece pretokenization/sampling documentation (Google)

Used as a design reference only; SentencePiece is **not** imported and Unigram is **not**
rebuilt (Unigram was already tested and rejected in this project).

Relevant documented ideas, filtered through NTokenizer's byte-level BPE + MathSymbolAware design:

1. **Vocabulary/coverage constraints** (`--character_coverage`, `--byte_fallback`): our byte-level
   design already guarantees coverage of 100% of bytes via 256 base tokens. Nothing to import.
2. **Sampling hard_vocabulary_limit** — not applicable (we are not redoming alpha).
3. **Script-aware boundaries**: SentencePiece splits on Unicode script transitions similar to
   our `split_scripts`. Our MathSymbolAware does this; the v0.4 CJK chunk experiments showed
   fixed-size Han chunks were net-negative. More principled improvement available: **CJK
   punctuation-aware boundaries are already handled** (U+FF0C etc. are Punct), and Han runs
   attach to their following kana — the real problem is unsegmented long Han *sentences* whose
   pair statistics are too sparse. The correct lever for that is **corpus**, not chunking.
4. **Number boundaries**: SentencePiece isolates digits optionally. Our tokenizer already keeps
   digit runs whole with decimal/time/grouping handling — this is closer to optimal for math.
5. **Whitespace**: SentencePiece (by default) replaces spaces with U+2581 and can
   treat ▁as part of words. Our leading-space attachment achieves the same effect without
   altering the byte stream. Keep.
6. **Corpus sampling**: SentencePiece offers `--input_sentence_size` sampling. Our equivalent is
   the manifest weight/cap mechanism already used for v0.3's headroom targeting. The v0.4-v2
   experiment extends exactly this mechanism with newly discovered unused data.
7. **Training data scale**: SentencePiece docs recommend enough sentences for vocab coverage;
   our 1.96 GB was already 5× the heuristic; expansion to ~3.6 GB is our lever.

**Decision.** Use (6) as the primary mechanism this cycle (expanded manifest from the
already-clean, non-holdout, unused corpus pool). (1–5) are already satisfied by the design.

## Feasibility arithmetic for the 6.0 B/T target

Required: 207,712,944 bytes / 6.0 tokens ≈ 34.62M tokens vs 37.40M → **−7.4% tokens**
(+8.0% B/T) against polished baseline.

Historical yield: v0.2→v0.3 gained +12.7% B/T via a **2× corpus expansion to 1.96 GB +
MathSymbolAware pretokenizer + mwf=2**. v0.4-v2 has no new pre-tokenizer breakthrough; the
one comparable lever remaining is **corpus expansion from the same clean tree**: auditing
`data/v0.2/clean/train` shows **3.12 GB of unused, licensed, non-holdout data** (Turkish 702 MB,
Arabic 635 MB, Hindi 418 MB, English books 367 MB, Korean 345 MB, simplewiki 179 MB, Japanese
128 MB, …). Training a new 256K model on ~3.6 GB of the same clean distribution with the same
recipe is the single most evidence-backed route to additional gains.

Risk: v0.3's ablations show diminishing returns when over-weighting low-B/T buckets; memory
must stay ≤ ~5 GiB (mwf=2 recipe; monitor RSS; abort if >5 GiB sustained).

Realistic expectation: **+2–5% overall**, not guaranteed to reach 6.0. It is still the best
available move; the result will be decided by measurement, not by hope. If the candidate lands
below 6.0, the outcome is NOT PROMOTED and v0.3 stays the model of record.

## Experiment shortlist (pilots, 64K scale, 20–60 MB, seed 42, identical recipe except the one variable)

- **P1 — Expanded mixture**: manifest built from v0.3 + unused clean data with per-bucket caps
  that do *not* target the development benchmark beyond the standing v0.3 headroom logic
  (restore source parity across buckets; e.g. Turkish to 700 MB-class, Arabic 750 MB-class,
  Hindi/Korean/English/Japanese all get their withheld share). Hypothesis: fresh in-distribution
  data improves merge frequency estimates → more usable merges at 256K.
- **P2 — Source-attribution diagnostic/pilot**: audit merge provenance (one-source tail); if a
  clean signal appears, 64K pilot with a mild R==1 skip-floor. Otherwise rejected by evidence,
  documented.
- **P4 — Pre-tokenizer CJK boundary**: do **not** chunk; test a pilot with `--min-word-freq-cjk 1`
  only if memory stays safe at pilot scale (this was the only arm with positive CJK signal).
  Han4 chunk remains rejected.
- **P5 — min-freq 2 vs 3**: cheap ablation on pilot to confirm 2 is still optimal.

Full-256K run is authorized only if a pilot shows **≥ +2.5% B/T at 64K vs the matched
v0.3-recipe pilot**; otherwise the cycle ends NOT PROMOTED with the polished v0.3-v2 intact.
