# NTokenizer v0.3 Memory Report

Date: 2026-11-15
Status: measured. Reference machine: Intel Core i7-12700H, 14 GiB RAM, Linux.
`MALLOC_ARENA_MAX=2` was set for all large training runs to contain per-thread
malloc arena growth; inference runs were left at the default
(`/usr/bin/time -v`, default allocator) so the numbers are directly comparable
with ordinary deployments.

## 1. Training peak RSS

| Run | Corpus / slice | Unique pre-tokens | Pruned | Used | Wall | Peak RSS |
|---|---|---:|---:|---:|---:|---:|
| v0.2 release (lws, 256K, mwf=1) | 1.019 GB | 10,820,251 | 0 | 10,820,251 | 13 min 30 s | **≈ 9.7 GB** (stated in `reports/v0.2/FINAL_REPORT.md`; not re-run) |
| Pilot 32K, LWS, mwf=1 | 210 MB | 2,584,248 | 0 | 2,584,248 | 2 min 26 s | 2.38 GiB (2,492,008 KiB) |
| Pilot 32K, ConservativeUnicode, mwf=1 | 210 MB | 2,380,589 | 0 | 2,380,589 | 1 min 48 s | 2.27 GiB (2,380,140 KiB) |
| Pilot 32K, ScriptAndDigitAware, mwf=1 | 210 MB | 2,334,124 | 0 | 2,334,124 | 1 min 53 s | 2.20 GiB (2,306,240 KiB) |
| Pilot 32K, Hybrid, mwf=1 | 210 MB | 3,484,787 | 0 | 3,484,787 | 2 min 28 s | 2.55 GiB (2,669,584 KiB) |
| Pilot 32K, Math, mwf=1 | 210 MB | 3,475,553 | 0 | 3,475,553 | 2 min 24 s | 2.55 GiB (2,669,204 KiB) |
| Pilot 32K, Code, mwf=1 | 210 MB | 3,552,531 | 0 | 3,552,531 | 3 min 11 s | 3.07 GiB (3,217,000 KiB) |
| Pilot 32K, ScriptTransition, mwf=1 | 210 MB | 2,254,689 | 0 | 2,254,689 | 1 min 31 s | 1.76 GiB (1,840,108 KiB) |
| Pilot 32K, RawByte, mwf=1 | 210 MB | 1,658,018 | 0 | — | **aborted at 1 min 53 s** | **7.52 GiB (7,884,192 KiB)** |
| Pilot 64K, Hybrid, mwf=2 | 210 MB | 3,484,787 | 2,467,587 | 1,017,200 | 44 s | 0.68 GiB (715,548 KiB) |
| Pilot 64K, Hybrid, mwf=1 (control) | 210 MB | 3,484,787 | 0 | 3,484,787 | 3 min 07 s | 2.58 GiB (2,702,784 KiB) |
| Pilot 64K, Hybrid, mwf=2 | 631 MB | 8,911,201 | 6,557,830 | 2,353,371 | 1 min 56 s | 1.55 GiB (1,628,236 KiB) |
| Pilot 64K, LWS, mwf=2 | 631 MB | 6,884,314 | 5,213,374 | 1,670,940 | 1 min 58 s | 1.11 GiB (1,167,032 KiB) |
| Pilot 64K, Math, mwf=2 | 631 MB | 8,891,360 | 6,544,493 | 2,346,867 | 2 min 19 s | 1.60 GiB (1,683,236 KiB) |
| Pilot 64K, Code, mwf=2 | 631 MB | 9,034,659 | 6,778,602 | 2,256,057 | 2 min 11 s | 1.59 GiB (1,665,772 KiB) |
| **v0.3 final, Math, mwf=2** | **1,963 MB** | **20,252,150** | **14,555,417** | **5,696,733** | **5 min 47 s** | **3.70 GiB (3,882,900 KiB)** |

Interpreting the table:

1. **`--min-word-freq 2` is the whole memory story.** It prunes 70–76% of unique
   pre-tokens before the pair index is built. On the 210 MB pilot it cut peak RSS
   from 2.58 GiB → 0.68 GiB (3.8×) at equal merge count; on the 631 MB pilot it is
   the only reason a 64K model fits at all; at 1.96 GB it caps the final run at
   3.70 GiB instead of an infeasible multiple of that.
2. v0.2 (mwf=1) used ≈ 9.7 GiB for a smaller corpus; v0.3 uses 3.70 GiB for 1.93×
   the bytes and 1.87× the unique pre-tokens, i.e. ≈ 4.5× better memory efficiency
   per byte of corpus.
3. RawByte is disqualified on memory grounds: 7.52 GiB RSS at 325/32,505 merges,
   extrapolating to >50 GiB for the full run.

## 2. Inference peak RSS

| Model | Threads | Repeats median peak (RSS) | Total incl. decode pass |
|---|---:|---:|---:|
| v0.3 | 1 | 406.3 MiB | 1119.5 MiB |
| v0.3 | 2 | 473.7 MiB | — |
| v0.3 | 4 | 619.5 MiB | — |
| v0.3 | 8 | 928.0 MiB | 1478.8 MiB |
| v0.3 | 20 | 1784–1810 MiB | — |
| v0.2 | 1 | 313.5 MiB | 1029.0 MiB |
| v0.2 | 8 | 510–531 MiB | 1154.4 MiB |
| v0.2 | 20 | 923–949 MiB | — |

Two distinct numbers are reported, and the difference is deliberate:

- **Repeat RSS** is the peak inside the timed encode pass (what `encode()` costs you).
- **Total RSS** additionally includes the harness’s decode-verification pass, which
  pre-encodes every benchmark file and holds all token IDs (~170 MB) before timing
  the decode — this is what `ru_maxrss` at process exit reports.

v0.3 costs ~90 MiB more than v0.2 at equal thread counts: the math-aware
pre-tokenizer keeps more per-file scratch, and the model file itself is 23.40 MB vs
22.71 MB. Multi-thread RSS grows ~70 MiB per additional encoder thread
(406 → 474 → 620 → 928 → ~1800 MiB at 1/2/4/8/20 threads); budget ≈ 0.2 GiB +
0.07 GiB per worker for the v0.3 model.

## 3. Model and data sizes

| Artifact | Size |
|---|---:|
| v0.3 `.ntok` | 23,403,000 B (22.3 MiB) |
| v0.2 `.ntok` | 22,712,471 B |
| v0.3 HF `tokenizer.json` | 10,401,898 B (9.9 MiB) |
| v0.3 production manifest | 10,555 files, 1,963.48 MB |
| Benchmark (held out) | 544 files, 207.71 MB |

## 4. Memory-safe usage notes

- Training the final model in a single process needed ≤ 4 GiB; a laptop with 16 GiB
  is sufficient headroom for `mwf=2` runs at this corpus scale. Running two training
  jobs concurrently, or `mwf=1`, is not.
- Inference at 1–8 threads fits comfortably in < 1.5 GiB per process; at 20 threads
  plan for ≈ 1.8 GiB.
- The benchmark harness holds the full token-ID set in memory for decode
  verification only when `--no-decode` is not passed; production pipelines that never
  verify decode will sit at the “Repeat RSS” column.
- `MALLOC_ARENA_MAX=2` measurably reduces RSS spread on many-threaded encode jobs
  (it is what all training runs used); it is optional for single-thread inference.
