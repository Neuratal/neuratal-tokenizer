# NTokenizer v0.4 Baseline Report — v0.3, re-measured

Date: 2026-11-15
Method: native harness `examples/benchmark_ntok` (release build), fresh run.
Dataset: continuity benchmark — 544 files, 207,712,944 bytes, held-out from training.

| Check | Value |
|---|---|
| Model | `output/v0.3/ntokenizer-0.3-256k.ntok` |
| Model SHA-256 | `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9` |
| Total bytes | 207,712,944 |
| Total tokens | **37,397,108** |
| Byte-weighted B/T | **5.5543** |
| Equal-category B/T | **5.1057** |
| Tokens/byte | 0.18004 |
| Tokens/word | 1.6380 |
| Used vocabulary | 245,334 / 262,144 (93.59%) |
| Decode exactness | 544/544 |
| Encode (1 thread, this session) | 9.54 MB/s, 1.72 M tokens/s (note: run-to-run variance on this machine is ±10%; v0.3 measured 10.18 MB/s in v0.3 session) |
| Decode (1 thread / 8 threads) | ~246–343 MB/s / ~1.4–1.9 GB/s (variance; see PERFORMANCE_REPORT) |
| Peak inference RSS | ~1.03 GiB (1 thread) / ~1.15 GiB (8 threads) |
| Training | seed 42, mwf=2, 1.963 GB corpus, 261,881 merges, 5 min 47 s, 3.70 GiB |

Per-bucket (byte-weighted B/T, continuity benchmark): english 4.463, turkish 5.305,
arabic 7.198, hindi 9.928, korean 5.000, spanish 4.736, chinese 3.538,
japanese 5.014, programming 4.096, stem 3.564, other_languages 3.320.

## 1. Reference SHA-256s

- v0.1 `output/ntokenizer-256k.ntok` — `8abe252f159ed58a1a0072eaac0f9861dab74b4e6bed5da9b69a218493662f00`
- v0.2 `output/v0.2/ntokenizer-0.2-256k.ntok` — `1fc3ee8568b7a08a124a5e27ee7a8078d5bdfae495a253b3f97ba0ad29ab54ac`
- v0.3 `output/v0.3/ntokenizer-0.3-256k.ntok` — `1f0d4ad7c7fb609001b2af4d9741ec5b8a5d7686fa3b92eb8eefd057fea936d9`

All three unchanged after archiving v0.4-fail.

## 2. Continuity benchmark is compromised for decisions

The 544-file benchmark was used to pick v0.2/v0.3 arms, so it is a *reference*, not
a pristine final test. The new sealed benchmark is
`data/v0.4/benchmark/holdout` (+ validation split + synthetic Unicode stress),
leakage-audited (`reports/v0.4-fail/BENCHMARK_SPEC.md`, `leakage_audit.json`).
We report both; decisions this round were made on the continuity set for
comparability with v0.3.

## 3. Session note

Numbers here were re-measured after archiving v0.4-fail with the *current* source
(which contains only additive code). Token count and B/T reproduce exactly.
