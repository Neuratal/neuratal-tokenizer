# NTokenizer v0.4 Vocabulary Report

Date: 2026-11-15
Status: measured on the same 544-file benchmark and the pilot20 corpus; the
50M-model conclusion is arithmetic, not speculation.

## 1. Export ladder (v0.3 merge order, `ntokenizer export`)

All exports from the same master (v0.3), same specialized tokens, each validated
and load-tested.

| Vocab | Merges | File size | Used vocab | Usage % | B/T (total) | Equal-cat B/T | Tokens |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 8,192 | 7,929 | 562,862 B | 8,090 / 8,192 | 98.75% | 3.2547 | 2.9480 | 63,818,561 |
| 16,384 | 16,121 | 1,200,313 B | 16,203 / 16,384 | 98.90% | 3.7202 | 3.3840 | 55,833,781 |
| 32,768 | 32,505 | 2,540,964 B | 32,296 / 32,768 | 98.56% | 4.1967 | 3.8295 | 49,494,884 |
| 65,536 | 65,273 | 5,347,050 B | 64,195 / 65,536 | 97.95% | 4.6869 | 4.2787 | 44,317,780 |
| 262,144 | 261,881 | 22,712,471 B | 232,659 / 262,144 | 88.75% | 4.9290* | 4.6857* | 42,140,653* |

*262K row shows v0.2; v0.3 master gives 5.5543 / 5.1057 / 37,397,108 / 93.59%.

Marginal B/T gain per doubling: +0.47 (8→16K), +0.48 (16→32K), +0.49 (32→64K),
+0.87 (64→256K on v0.2 master). Returns are roughly flat per 4× vocabulary.

## 2. Tiny-LM (BPB) reference — what the vocab size does to a small model

Tiny GPT (d_model=192, 4 layers, 4 heads, ctx=256, tied embeddings, AdamW,
LR 1e-3, 3,000 steps, batch 16×256, same corpus bytes: the pilot20 210 MB slice,
same seed) measured bits/byte on the held-out 21 MB tail:

| Vocab | Params | Val loss (nats) | tokens/byte | **BPB** |
|---:|---:|---:|---:|---:|
| 8,192 | 3.4 M | 7.258 | 0.356 | 3.73 |
| 16,384 | 5.0 M | 8.079 | 0.310 | 3.61 |
| 32,768 | 8.1 M | 8.675 | 0.270 | 3.38 |
| 65,536 | 14.4 M | 9.195 | 0.239 | 3.18 |
| 262,144 | 52.2 M | 9.950 | 0.199 | 2.86 |

Caveat (important): all arms are **undertrained** (≈12.3M tokens seen). BPB falls
monotonically with vocab because embedding capacity scales with vocab and dominates
at tiny token budgets. This is **not** evidence that 262K is the right choice for a
real 50M run; it shows only that at our budget bigger vocab looks better.
For a properly trained 50M model with billions of tokens, smaller vocabularies
(over)fit slower and cost fewer softmax parameters — the crossover is expected to
move toward 16K–32K.

## 3. The 50M arithmetic (hidden size 512, tied embeddings)

Single `V × 512` matrix:

| Vocab | Embedding params | % of 50M |
|---:|---:|---:|
| 8,192 | 4,194,304 | 8.4 % |
| 16,384 | 8,388,608 | 16.8 % |
| 32,768 | 16,777,216 | 33.6 % |
| 65,536 | 33,554,432 | 67.1 % |
| 262,144 | 134,217,728 | **268 %** — exceeds entire 50M budget |

Untied input/output heads roughly double these numbers.

## 4. Recommendation for the 50M model

**32,768** with tied embeddings (33.6% of parameters — the largest spend that is
still defensible at this scale), or **16,384** if parameter efficiency dominates
(16.8%; then B/T drops to 3.72 vs 4.20 and tokens grow 13%).

256K is disqualified by arithmetic alone; 64K is disqualified for a 50M model
unless embeddings are aggressively tied/shared and the rest of the model is shrunk
below 50M-total-effect.

## 5. Honest limitations

- Only 8/16/32/64/256K were measured; intermediate sizes follow the ladder.
- BPB numbers are single-seed; no variance reported; undertrained regime.
- Untied-embedding cost not measured (all tied).
- The 50M model has not been trained; the report answers "which vocab would you
  start with and why", not "here is the trained model".
