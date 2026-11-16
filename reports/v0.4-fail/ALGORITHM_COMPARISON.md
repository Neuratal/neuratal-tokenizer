# NTokenizer v0.4 Algorithm Comparison

Date: 2026-11-15
Scope: is frequency-based BPE still the right merge-selection algorithm, or does
some published alternative beat it on *our* corpus? All options were measured on
the same 210 MB pilot20 slice, same 64K/32K budget, same seed — and all were
tested against the same held-out data.

## 1. Frequency-based BPE (the incumbent)

Already measured (v0.3 master, exported to 32K):

| Model | B/T | Equal-cat B/T |
|---|---:|---:|
| v0.3 32K export (frequency BPE) | 4.1967 | 3.8295 |
| v0.3 64K export | 4.6869 | 4.2787 |
| v0.3 256K (release) | 5.5543 | 5.1057 |

## 2. Parity-aware BPE (Foroutan et al., ACL 2026)

We ran the reference implementation (`swiss-ai/parity-aware-bpe`, MIT) as an
isolated pilot — it is a heavyweight method (per-language pair counts), so this
is a **screening experiment**, not a full adoption test.

- Configuration: 300 merges, `--ratio 1.0` per language (no dev set), `--pretokenize bytelevel`,
  11 languages, ~43 MB corpus (4 MB each).
- Results:
  - Gini coefficient of per-language token costs: 0.0917 (classical) → **0.0552 (−40%)** — the fairness mechanism works.
  - But global compression collapsed: B/T 1.6735 → 1.3302 (**+25.9% more tokens**),
    equal-category 1.6123 → 1.3096.
- Hybrid variant (`--global-merges 250` + 50 parity merges, with a small dev set):
  Gini 0.0905 ≈ classical (no benefit), B/T 1.4449 (worse than classical),
  equal-category 1.4012 — both gains evaporated.

**Decision: not adopted.** The published claim requires the fair-max rule to trade
"a small amount" of global compression for parity; on our corpus at this budget,
the trade is large and one-sided. The honest reading of the paper: parity-aware
shines with a *multi-parallel* dev set (we don't have one) and at much larger
vocab budgets; our measurement is a regime where it does not help. Not adopted;
documented as a tried-and-rejected arm.

## 3. Unigram LM (Kudo 2018, SentencePiece)

External reference implementation, isolated venv — used only as a measuring stick.

| Variant | B/T | Equal-cat B/T | Decode exact | Decision |
|---|---:|---:|---:|---|
| SentencePiece Unigram 32K, default (NFKC + dummy-prefix) | 4.1103 | 3.9460 | **0/544** | rejected (lossy) |
| Unigram 32K, identity normalizer, byte_fallback | 3.7422 | 3.5664 | 192/544 | rejected (worse + lossy) |
| Our frequency BPE 32K (v0.3 32K export) | 4.1967 | 3.8295 | 544/544 | incumbent wins |

**Decision: keep BPE.** The Unigram line does not beat frequency BPE on this
corpus; the default configuration is lossy, which disqualifies it outright for a
byte-exact pipeline. This is consistent with the Vemula et al. observation that
downstream gains are task- and language-specific, not universal — we did not find
a free win here either.

## 4. MDL / significance-gain (Nouri 2026 preprint)

The published construction separates merge selection into (i) discovery by
significance-gain, (ii) replication via hypergeometric test on a disjoint
partition, (iii) MDL-based acceptance/stopping on a third partition. Implementing
that faithfully inside our deterministic trainer is a multi-day build with a real
risk of subtly biasing the corpus; the method is a fresh single-author preprint
that has not been independently replicated.

**Not attempted.** Reasons: (a) preprint only, no replication; (b) the machinery
(three partitions, per-iteration BH correction, MDL acceptance) is unlike our
incremental pair index; (c) v0.4's binding constraint was CJK, and we spent the
budget there. Marked as an explicitly skipped arm — not silently dropped.

## 5. BPE-dropout (Provilkov 2020)

Designed as a *training-time regularizer* for the downstream LM (changes
segmentation per sample). NTokenizer inference is deterministic; the v0.4
candidate is not an LM. **Not adopted; out of scope for this release.**

## 6. Summary table

| Arm | Global B/T | Equal-cat B/T | CJK | Decode exact | Adopted |
|---|---:|---:|---:|---:|---|
| Frequency BPE (incumbent) | 4.1967 (32K) | 3.8295 | reference | yes | yes |
| Parity-aware BPE (300 merges) | 1.3302 (−20%) | 1.3096 | +2.3% korean | n/a | no |
| Parity-aware + 250 global merges | 1.4449 (−14%) | 1.4012 | no | n/a | no |
| Unigram 32K (SP default) | 4.1103 (+1.1% vs 32K BPE)* | 3.9460 | n/a | **no** | no |
| Unigram 32K (identity+byte_fallback) | 3.7422 (−11%) | 3.5664 | n/a | no | no |
| MDL-SG | not measured | — | — | — | no (skipped, documented) |
| BPE-dropout | not measured | — | — | — | no (not applicable) |

*NFKC Unigram looks 1.1% better on raw B/T but decodes 0/544 — disqualifying.

## 7. What did help (and is in v0.4)

CJK chunk arms + `--min-word-freq-cjk` trainer floor (CJK_UNICODE_REPORT.md), the
Benchmark v1 + leakage audit infrastructure, and the profile-guided encode-time
analysis. None of these depend on the rejected merge-selection variants.
