# NTokenizer v0.4 Research Notes

Date: 2026-11-15
Purpose: record the literature that informed v0.4 design decisions, what was
(and was not) adopted, and why. Every external claim is labeled with its source;
nothing from these papers is presented as a result measured on our own data.

## 1. Foroutan, Meister, Paul, Niklaus, Ahmadi, Bosselut, Sennrich — Parity-Aware Byte-Pair Encoding

- Venue: ACL 2026 (Volume 1: Long Papers), pp. 7514–7538, July 2026.
- DOI: `10.18653/v1/2026.acl-long.342` · arXiv `2508.04796` · Anthology ID `2026.acl-long.342`
- Source type: published conference paper + MIT-licensed reference implementation
  (github.com/swiss-ai/parity-aware-bpe, MIT).
- Core idea: at each merge step, maintain per-language pair statistics and select
  the best merge **for the currently worst-compressed language** ("fair-max"),
  instead of the global pair frequency. Greedy, per-language expansion order.
- Reported scope/results (their data): 60 languages, two vocabulary sizes; Gini
  coefficient of per-language token costs reduced by up to 89% vs classical BPE;
  "negligible impact on global compression rate"; no systematic downstream LM
  degradation on 12–13 multilingual benchmarks.
- Limitations they state: requires a multi-parallel development set or ratio
  targets; pre-tokenization bounds achievable compression; fairness is defined
  only through token counts.
- NTokenizer relevance:
  - Our corpus is **not** parallel — each language appears in different (Wikimedia/GUTENBERG/code)
    documents, which breaks the method's premise. `--ratio` can be used to derive
    per-language targets from training-data compression, as the reference tool allows.
  - Training cost: roughly an extra O(#languages) statistics pass per merge — the
    reference implementation keeps full per-language pair counts. At our scale
    (262K merges), this would roughly double trainer memory; not viable on 14 GB RAM
    without a compact pair index.
  - We ran a **screening experiment** with the reference implementation on our data
    (see `ALGORITHM_COMPARISON.md`): at 300 merges and ratio=1.0, parity-aware BPE
    reduces token-cost Gini (0.0917 → 0.0552, −40%) but raises total tokens by
    25.9% (B/T 1.67 → 1.33). With `--global-merges 250` the trade-off is under test.
  - **Decision input, not a committed design**: the result will decide; the CJK problem
    we are actually chasing is caused by pre-token pruning (see `CJK_UNICODE_REPORT.md`),
    not by merge selection's optimization direction.

## 2. Vemula, Dandapat, Sharma, Krishnamurthy — Rethinking Tokenization for Rich Morphology

- Venue: IJCNLP-AACL 2025 Student Research Workshop, pp. 232–252, December 2025.
- DOI: `10.18653/v1/2025.ijcnlp-srw.20` · arXiv `2508.08424` · Anthology ID `2025.ijcnlp-srw.20`
- Source type: published workshop paper.
- Core findings (their data): small BERT models (8.5M params) on
  **Telugu, Hindi, English** — Unigram-based tokenizers consistently beat BPE on
  downstream tasks; morphological alignment (as measured by their new gold
  Telugu morpheme-segmentation datasets) correlates only moderately with downstream
  performance (Spearman r ≈ 0.47 for recall); linguistically-motivated
  pre-segmentation helps BPE but not Unigram.
- Key caveats they state: encoder-only BERT, NLU tasks only, 8.5M-parameter models,
  three languages — do not treat as universal evidence that Unigram > BPE.
- NTokenizer relevance: motivated testing Unigram on our pilot corpus (Stage 4).
  Their result is **not** evidence for NTokenizer — we evaluate on our benchmark with
  the same protocol as BPE. The Turkish/Hindi morphology claim is exactly the axis we
  care about, so the experiment is justified; it is not a licence to assume the result
  will transfer.

## 3. Nouri — MDL-Calibrated Significance-Gain Pair Encoding (MDL-SG)

- Venue: arXiv preprint `2609.31705`, September 2026. **Not peer-reviewed.**
- License: CC BY 4.0 (paper text).
- Core idea: BPE stops being "vocab size fixed by user" and instead: (i) discovery by
  **significance-gain** `c_xy · z(x,y) · c_xy^α (α=0.25)` — how strongly the observed
  pair exceeds independence; (ii) replication via exact one-sided hypergeometric test
  with per-iteration Benjamini–Hochberg correction on a disjoint partition; (iii)
  utility by net MDL gain on a third partition; automatic stop.
- Their results (their data): WikiText-103, 500K-char training → stops at 847 merges,
  vocab 1,017; TinyGPT (2,024,448 params, 500 optimizer updates): test BPC 3.2436
  (MDL-SG) vs 3.2894 (SG-BPE) vs 3.3493 (frequency BPE); note frequency BPE wins on raw
  compression, MDL-SG wins on predictive efficiency; they themselves stress the
  compression/LM-utility divergence.
- Limitations: single small dataset (WikiText-103), tiny model, single author
  preprint, no public code identified; methodological sophistication (three partitions,
  multiple-testing correction) is heavy for our pilot phase.
- NTokenizer relevance: MDL-SG is a **research hypothesis**, not an acceptance
  criterion. We test a significance-gain-style merge scoring in a pilot only
  (see `ALGORITHM_COMPARISON.md`). MDL auto-stop is out of scope for v0.4.
  Also important for our evaluation design: "best compression" and "best LM utility"
  measurably diverge — so we track both bits-per-byte (LM pilot) and B/T.

## 4. Kudo & Richardson — SentencePiece (reference for Unigram)

- Venue: EMNLP 2018 System Demonstrations, `D18-2012`.
- SentencePiece (Google) is BSD-3-Clause; subword regularization is in Kudo 2018
  (arXiv `1804.10959`).
- NTokenizer relevance: Unigram is a legitimate candidate algorithm family. We may use
  SentencePiece **only as an external reference implementation** in an isolated
  environment to screen whether Unigram helps on our corpus before writing our own
  trainer. We will not copy its code into NTokenizer; NTokenizer keeps its own
  deterministic trainer. Any "SentencePiece result" is labeled as external.

## 5. Provilkov et al. — BPE-Dropout

- Venue: ACL 2020, `2020.acl-main.170`.
- Purpose-built to produce **variable segmentations at training time** as
  regularization for the downstream LM. Inference remains deterministic.
- NTokenizer relevance: not suitable as an inference-time behavior. Only relevant if
  we do an LLM-pretraining augmentation experiment (Stage 12); currently no 50M LM
  training is scheduled, so BPE-dropout is **out of scope** and documented as such.

## 6. Unicode Standard Annex #29 — Text Segmentation (Unicode 18.0)

- Latest: Unicode 18.0.0, tr29-49, dated 2026-09-01 (UAX #29, rev 49).
- What changed in 18.0: Rule GB9c (Indic_Conjunct_Break Linker) revised; otherwise
  incremental. Relevant because our Devanagari and other Indic fixtures should match
  the current standard, not an older one.
- Key positions for NTokenizer:
  - Grapheme clusters are user-perceived characters; nothing requires one grapheme ==
    one tokenizer token. NTokenizer does not force grapheme boundaries.
  - Word boundaries (WB rules) are a separate concept; NTokenizer's pretokenizer uses
    script/category rules and is byte-preserving; it does **not** normalize.
  - UAX #29 default rules do not handle CJK word segmentation (no dictionary-based
    segmentation); that is expected — CJK runs are single runs in our pretokenizer,
    which motivated the CJK chunk arms.

## 7. Reference tokenizers (external, for comparison only)

- OpenAI tiktoken (o200k_base, cl100k_base, GPT-2 r50k) — MIT license (code); the
  encodings ship data, and usage of the encoding files is permitted for evaluation.
  tiktoken **is not** used to build NTokenizer vocab; it is a benchmark reference.
  All measurements done with the library's official `get_encoding` on the same
  benchmark bytes.
- Qwen family tokenizer (HF `Qwen/Qwen2-0.5B` and similar) — Apache-2.0 for many
  Qwen2/2.5 model files; exact revision will be pinned and recorded before use.
  Used as external reference tokenizer if its files load in `tokenizers`; otherwise
  documented as unavailable (never copied into NTokenizer).
- BERT WordPiece / bert-multilingual — Apache-2.0 (Google) — available as sanity
  reference; not a target to beat.

## 8. What v0.4 adopted (preliminary, before final benchmarks)

| Method | Status | Evidence |
|---|---|---|
| CJK chunk arms (MathSymbolAwareCjk2/4/8) | implemented, pilot-tested, memory-safe | `CJK_UNICODE_REPORT.md` |
| Flat `--min-word-freq` kept as default | kept | ablation shows CJK arms trade ≤2% elsewhere for +8–22% CJK |
| Parity-aware merge selection | **screened, not adopted** | `ALGORITHM_COMPARISON.md` |
| Unigram LM | screened via external reference | `ALGORITHM_COMPARISON.md` |
| MDL-SG significance-gain | small pilot (LLR-style score) | `ALGORITHM_COMPARISON.md` |
| BPE-dropout | out of scope (no LM pretraining scheduled) | this file |
| Unicode normalization (NFC/NFD…) | NOT applied by default; optional, documented | byte-exact decode invariant |
| UAX #29 regex segmentation | not used as BPE boundary rule | see §6 |
