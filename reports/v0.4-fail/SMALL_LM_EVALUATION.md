# NTokenizer v0.4 Small-LM Evaluation

Date: 2026-11-15
Scope: does the v0.4 tokenizer (256K master vocabulary, exported downwards) hold up
when a small transformer is trained on identical bytes? Methodology is fully
controlled: same architecture, same corpus bytes, same steps, same hyperparameters;
vocab size is the only variable.

## 1. Setup

- Corpus: `pilot20` — 210.38 MB of the training distribution, split deterministically
  into 90% train (189.35 MB) and 10% validation (21.04 MB) on byte boundaries
  (`experiments/v0.4/pilot20_train.txt` / `_val.txt`). Identical bytes fed to all
  arms via `examples/lm_encode.rs` (uint32 little-endian streams).
- Model: GPT decoder — d_model 192, 4 layers, 4 heads, ctx 256, tied embeddings,
  AdamW (0.9, 0.95), LR 1e-3 linear warm-up 100 steps + cosine decay to 0.1×,
  weight decay 0.01, grad-clip 1.0, fp32, batch 16×256.
- Tokens/arm: 8192, 16384, 32768, 65536, 262144 (exports of the v0.3 master;
  merge order identical).
- Metric: bits-per-byte on the 21 MB held-out tail:
  `BPB = val_loss_nats × (tokens/byte) / ln 2`.

## 2. Results

| Vocab | Params | Val loss (nats) | tokens/byte | **BPB** |
|---:|---:|---:|---:|---:|
| 8,192 | 3,401,856 | 7.258 | 0.356 | 3.73 |
| 16,384 | 4,974,720 | 8.079 | 0.310 | 3.61 |
| 32,768 | 8,120,448 | 8.675 | 0.270 | 3.38 |
| 65,536 | 14,411,904 | 9.195 | 0.239 | 3.18 |
| 262,144 | 52,160,640 | 9.950 | 0.199 | 2.86 |

Training cost: 8K/16K/32K/64K ≈ 45/62/100/182 s; 262K ≈ 1,168 s (float32 embeddings
at 262K × 192 ≈ 50 M params hit GPU memory walls — needed chunked loss).

## 3. What the numbers say

- Per-token cross-entropy rises with vocab size (more tokens to predict), exactly
  as expected; BPB falls monotonically because the token stream gets denser.
- **The ranking is budget-dependent.** All arms are undertrained (≈12.3M tokens
  each). With a full 50M-run data budget (billions of tokens), the optimal vocab
  for a fixed parameter count shifts downward (entropy-vs-capacity argument);
  our data cannot measure that crossover.
- Even so, the efficiency ladder (B/T vs parameter cost) is the decisive signal:
  16K→32K buys +0.48 B/T for 3.1 M extra embedding params; 32K→64K buys +0.49 for
  16.8 M; 64K→256K buys +0.87 for 117 M (disqualifying for 50M).

## 4. Recommendation (see `VOCABULARY_REPORT.md` §4)

For the 50M model: **32K** (tied embeddings), **16K** if the parameter budget is
strict. 256K impossible (268% of budget). 64K only with aggressive sharing.

## 5. Honest limitations

- Small model, few steps, single seed; no error bars.
- The tiny-LM result cannot transfer quantitatively to a 50M model; it indexes the
  ladder direction at one compute budget, not the endpoint.
- The v0.4 candidate (cjk8) was not re-exported for the LM pilot — the exports use
  the v0.3 master; the conclusion is master-agnostic for the ladder shape.
- BPB is byte-level prediction quality, not downstream NLU/NLG quality.

## 6. Reproduction (token streams are regenerable, not committed)

The uint32 token streams and the 210 MB corpus slice used for BPB were deleted as
regenerable scratch. Rebuild: `lm_encode` (see `examples/lm_encode.rs`) over a
90/10 deterministic split of `experiments/v0.3/pilot_manifest_20mb.json` corpus
bytes; the tiny GPT definition is in `experiments/v0.4/tiny_lm.py`; per-vocab
results in `experiments/v0.4/lm/*/tiny_lm_result.json`.

## 7. Next step for the 50M decision

Train the actual 50M model at 16K and 32K for a few hundred million tokens and
compare loss curves; the tokenizer artifacts for both sizes are ready
(`experiments/v0.4/exports/`).
