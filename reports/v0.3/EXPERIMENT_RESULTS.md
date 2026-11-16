# NTokenizer v0.3 Experiment Results (narrative)

Machine-readable data: `EXPERIMENT_RESULTS.csv`. Every row cites the benchmark JSON
in `experiments/v0.3/results/` and (for trained rows) the training log in
`experiments/v0.3/logs/`. This narrative summarises what the rows show.

## Headline comparisons

| Arm | B/T | Equal B/T | Notes |
|---|---:|---:|---|
| o200k_base | 4.0751 | 4.0899 | independent tiktoken measurement |
| v0.2 256K (LWS) | 4.9290 | 4.6857 | verified baseline |
| v0.2 vocab sweep (8K→256K) | 2.99 → 4.929 | 2.81 → 4.686 | diminishing returns per doubling |
| 32K pilots (210 MB/bucket) | see CSV | see CSV | math 4.0626 / code 4.0691 / hybrid 4.0604 / lws 3.8945 / script-transition 2.8994 / script-digit 2.8114 / conservative 2.9221 / raw **aborted** |
| 64K pilots (631 MB/bucket) | see CSV | see CSV | math **4.6033/4.2640** > hybrid 4.5992/4.2585 ≈ code 4.6004/4.2366 ≫ lws 4.3549/4.0288 |
| **Final v0.3 (1.96 GB, 256K, math, mwf=2)** | **5.5543** | **5.1057** | single-thread 10.18 MB/s; 8-thread 56.30 MB/s; 544/544 byte-exact |

## What the tournaments showed

1. **Leading-space attachment is the single largest pretokenizer win.**
   ScriptTransitionAware (LWS minus the leading-space attachment) collapses to
   2.89 B/T, ~1.0 B/T below LWS on the same pilot; leading-space attachment is
   kept.
2. **MathSymbolAware ≥ Hybrid ≈ CodeAware** on both 32K and 64K slices; all three
   are within 0.15% of each other, while LWS is 4–7% behind.
3. **Corpus scaling helps modestly**: 20→60 MB/bucket buys +1.7% B/T at 64K; the
   final 1.96 GB/256K model captures the full training advantage.
4. **Vocabulary scaling saturates**: after 128K, each doubling buys <5%.
5. **`--min-word-freq 2` is free at mwf2 scale**: pilot20 rlf2 gives 4.5243/4.2205
   vs mwf1 4.5259/4.3069 — a tie on bytes/token, with 3.8× lower peak RSS.
6. **RawByte is infeasible** with the current trainer; recorded, not retried.

## Honest negatives

- chinese −5.76%, japanese −6.75%, other_languages −15.91% vs v0.2 (root cause in
  `BENCHMARK_REPORT.md` §4).
- Encode throughput is 12–14% below v0.2; v0.3 is heavier per file.
- 8.15 B/T stretch target not reached; gap 2.6 B/T.
- HF export parity is 3/16 token-IDs; decode is byte-exact both ways.
