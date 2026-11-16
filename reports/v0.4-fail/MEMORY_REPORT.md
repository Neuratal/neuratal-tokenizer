# NTokenizer v0.4 Memory Report

Date: 2026-11-15
Reference machine: Intel Core i7-12700H, 14 GiB RAM, Linux. `MALLOC_ARENA_MAX=2`
was used for all large training runs; inference runs used the default allocator.

## 1. Training peak RSS (final/full-scale runs)

| Run | Corpus | Wall | Peak RSS |
|---|---|---|---|
| v0.2 release (lws, 256K, mwf=1) | 1.019 GB | 13:30 | ≈ 9.7 GB (stated in v0.2 report) |
| v0.3 (math, mwf=2) | 1.963 GB | 5:47 | 3.70 GiB (3,882,900 KiB) |
| v0.4 control arm (math, mwf=2, carved-exclusion corpus) | 1.928 GB | 3:59 | 3.58 GiB (3,758,260 KiB) |
| v0.4 candidate (cjk8, mwf=2, full corpus) | 1.963 GB | 4:38 | 3.75 GiB (3,934,728 KiB) |
| v0.4 rejected arm (cjk4, mwf=1) | 1.963 GB | 3:58 | 3.40 GiB (3,559,572 KiB) |
| v0.4 E pilot (mwf=2 + CJK-exempt) | 0.21 GB | — | 1.95 GiB pilot-scale |
| v0.4 E full-scale (aborted) | 1.963 GB | — | **8.3 GiB at 3.5% of merges, 235M heap entries — killed** |

The v0.4 candidate holds the v0.3 memory envelope exactly: CJK chunking alone
keeps the pair index at the v0.3 size; the CJK-exemption alternative blows it up
(the `--min-word-freq-cjk` flag is available but must not be used at
production scale — see `CJK_UNICODE_REPORT.md`).

## 2. Inference peak RSS (544-file benchmark, median of 5 runs)

| Model | 1 thread | 8 threads |
|---|---|---|
| v0.4 candidate | 1,123 MiB | 1,332 MiB |
| v0.3 | 1,119 MiB | 1,479 MiB |
| v0.2 | 1,029 MiB | 1,154 MiB |

Multi-thread RSS grows ~70 MiB per additional encoder thread (arena + per-file
scratch). Production inference at 1–8 threads fits comfortably in < 1.5 GiB.

## 3. Model and data sizes

| Artifact | Size |
|---|---|
| v0.4 candidate model | 23,387,324 B (22.3 MiB) |
| v0.3 model | 23,403,000 B |
| v0.2 model | 22,712,471 B |
| v0.4 HF tokenizer.json | 10,393,332 B |
| v0.4 production manifest | 10,555 files, 1.96 GB |

## 4. Memory-safety notes

- Training the v0.4 candidate fits in ≤ 4 GiB RSS; `MALLOC_ARENA_MAX=2` is
  recommended for ≥8-thread encode jobs to contain per-thread arenas.
- The abort evidence for `--min-word-freq-cjk` at full scale is the cautionary
  record that CJK corpora alone can consume the memory envelope if the
  frequency floor is naive.
- No swap was used by any run (`Swaps: 0` in every `/usr/bin/time -v` log).
