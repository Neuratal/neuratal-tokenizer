# Vocabulary Size Analysis for a 50M Parameter Autoregressive Language Model

> This document presents a **consistent, reproducible** parameter accounting for
> a ~50M-parameter decoder-only transformer with tied input/output embeddings,
> used to pick a target vocabulary size for Neuratal Tokenizer training. Numbers were
> recomputed for this revision; the model must still compute its own parameter
> count rather than assuming it.

## 1. Parameter Budget Methodology

Architecture assumed per layer (modern decoder-only transformer, no linear biases):

| Component | Parameters per layer |
|-----------|---------------------:|
| Attention: Q, K, V, O projections (no bias) | `4 × D²` |
| FFN with 4× expansion (no bias) | `8 × D²` |
| 2 normalization layers (weight only, 2×2D) | `4 × D` |
| **Total per layer** | **`12 × D² + 4 × D`** |

Plus token embeddings. With **tied embeddings** (LM head shares the embedding
matrix), the embedding cost is paid once:

```
Total = V × D + L × (12 × D² + 4 × D)
```

Where `V` = vocabulary size, `D` = hidden dimension, `L` = number of layers.
Untied embeddings add one more `V × D` matrix (see §6).

No positional-embedding table is counted (rotary/ALiBi-style position handling
is assumed); biases, logit soft-capping, and gating variants shift counts
slightly. The conclusion (§5) is robust to those shifts.

## 2. Candidate Vocabulary Sizes

| Vocab Size | Learned Merges | Special Tokens |
|-----------:|---------------:|---------------:|
| 16,384     | ~16,117        | ~11            |
| 32,768     | ~32,501        | ~11            |
| 49,152     | ~48,885        | ~11            |
| 65,536     | ~65,269        | ~11            |

(256 base bytes + learned merges + special tokens; exact merge counts depend on
corpus and `min_frequency`.)

## 3. Parameter Count Table (Tied Embeddings, ≤ 50,000,000)

`per_layer(D) = 12D² + 4D`. `L_max = floor((50,000,000 − V×D) / per_layer(D))`.

### D = 384 (per_layer = 1,771,008)

| Vocab | Embedding | Max L | Chosen L | Total params |
|-------|----------:|------:|---------:|-------------:|
| 16,384 | 6,291,456  | 24 | **24** | **48,795,648** ✓ |
| 32,768 | 12,582,912 | 21 | **21** | **49,774,080** ✓ |
| 49,152 | 18,874,368 | 17 | **17** | **48,981,504** ✓ |
| 65,536 | 25,165,824 | 14 | **14** | **49,959,936** ✓ |

### D = 512 (per_layer = 3,147,776)

| Vocab | Embedding | Max L | Chosen L | Total params |
|-------|----------:|------:|---------:|-------------:|
| 16,384 | 8,388,608  | 13 | **13** | **49,309,696** ✓ |
| 32,768 | 16,777,216 | 10 | **10** | **48,254,976** ✓ |
| 49,152 | 25,165,824 |  7 | **7**  | **47,200,256** ✓ |
| 65,536 | 33,554,432 |  5 | **5**  | **49,293,312** ✓ |

### D = 768 (per_layer = 7,080,960)

| Vocab | Embedding | Max L | Chosen L | Total params |
|-------|----------:|------:|---------:|-------------:|
| 16,384 | 12,582,912 |  5 | **5** | **47,987,712** ✓ |
| 32,768 | 25,165,824 |  3 | **3** | **46,408,704** ✓ |
| 49,152 | 37,748,736 |  1 | **1** | **44,829,696** ✓ |
| 65,536 | 50,331,648 |  0 | —        | exceeds at L=0 ✗ |

> At D=768, V=32,768 only fits 3 layers — too shallow for a general-purpose
> model. D=768 is not suitable at larger vocabularies.

## 4. Trade-offs

### Compression efficiency

BPE compression improves roughly with `log(V)` for natural language. Larger
vocabularies cost embedding parameters but fit more text per context window.

| Vocab | Approx. bytes/token (trained multilingual) |
|-------|---------------------------------------------|
| 16,384 | ~3.5–4.0 |
| 32,768 | ~4.5–5.5 |
| 49,152 | ~5.0–6.5 |
| 65,536 | ~5.5–7.5 |

> Order-of-magnitude estimates only. Measure on your corpus with
> `ntokenizer benchmark` before deciding. Do not treat these as guarantees.

### Embedding cost share (D = 512)

- V=16,384 @ L=13: embedding = 8.39M of 49.31M (17.0%)
- V=32,768 @ L=10: embedding = 16.78M of 48.25M (34.8%)
- V=65,536 @ L=5: embedding = 33.55M of 49.29M (68.1%)

### Context efficiency

At 2048 tokens context, V=32,768 covers on the order of 9–11 KB of text per
sequence versus 7–8 KB at V=16,384 (estimate, corpus-dependent).

## 5. Recommendation

> **Recommended configuration for a strict 50M budget: V = 32,768, D = 512, L = 10**
> (tied embeddings, no biases)
>
> - Total parameters: **48,254,976** (under the 50M ceiling; ~1.75M headroom for
>   rotary buffers, extra norms, or an untied final norm)
> - Depth 10 at D=512 is reasonable for this size class.
> - Embedding share 34.8% is acceptable.
> - Learned merges ≈ 32.5k, feasible for a multilingual corpus.

### Alternative: maximum depth

> **V = 16,384, D = 512, L = 13** — **49,309,696** parameters
>
> 3 extra layers of depth at better embedding efficiency (17% of budget), at the
> cost of less compression and fewer tokens' coverage per context.

If the model runs on extremely memory-constrained hardware, this is the more
efficient pick.

## 6. Notes on Untied Embeddings

Untied embeddings add `V × D` parameters:

| Config | Tied total | Untied total | Untied fits? |
|--------|-----------:|-------------:|:------------:|
| V=32,768, D=512, L=10 | 48,254,976 | 65,032,192 | ✗ |
| V=49,152, D=512, L=7  | 47,200,256 | 72,366,080 | ✗ |
| V=16,384, D=512, L=13 | 49,309,696 | 57,698,304 | ✗ |
| V=16,384, D=512, L=10 | 39,866,368 | 48,254,976 | ✓ |
| V=16,384, D=384, L=24 | 48,795,648 | 55,087,104 | ✗ |

**Conclusion:** at V=32,768+, tied embeddings are required to stay under 50M.
Untied embeddings only fit at V=16,384 with L ≤ 10.

## 7. Configuration Summary

```
Recommended: V=32768, D=512, L=10, tied embeddings, no biases
  Embedding matrix:  32768 × 512 = 16,777,216 params
  Transformers:      10 × 3,147,776 = 31,477,760 params
  Grand total:       48,254,976 params (< 50M ✓)

Alternative: V=16384, D=512, L=13, tied embeddings, no biases
  Embedding matrix:  16384 × 512 =  8,388,608 params
  Transformers:      13 × 3,147,776 = 40,921,088 params
  Grand total:       49,309,696 params (< 50M ✓)
```

**Disclaimer:** these counts cover the embedding and transformer blocks only.
Final counts must include the model's own extra components (final norm, rotary
buffers, embeddings of any aux tables) and must be computed by the model code
before training, not assumed from this document.

## 8. Measured v0.2 export curve (2026-11-15, same merge history)

Held-out benchmark (544 files, 207.7 MB; leakage-free vs train), bytes/token:

| Export | vocab | bytes/token | vs smaller step |
|---|---:|---:|---:|
| ntokenizer-0.2-256k-8192.ntok | 8,192 | 2.991 | — |
| ntokenizer-0.2-256k-16384.ntok | 16,384 | 3.397 | +13.6% |
| ntokenizer-0.2-256k-32768.ntok | 32,768 | 3.822 | +12.5% |
| ntokenizer-0.2-256k-65536.ntok | 65,536 | 4.229 | +10.7% |
| ntokenizer-0.2-256k-131072.ntok | 131,072 | 4.606 | +8.9% |
| ntokenizer-0.2-256k-196608.ntok | 196,608 | 4.800 | +4.2% |
| ntokenizer-0.2-256k.ntok | 262,144 | 4.929 | +2.7% |

128K→256K gain on v0.2 is **+7.0%**; 192K→256K is **+2.7%** — larger than v0.1's
+2.45% / +0.97% because the lws pretokenizer lets bigger merge budgets help
everywhere. For deployment prefer the largest vocab your embedding table affords;
`ntokenizer-0.2-256k-65536` (4.23 B/tok) already matches/beats v0.1's 256K on this
benchmark. Export sizing DOES NOT retrain: exports share one merge history and are
bit-exactly consistent (rank-order prefix), independently validated.
